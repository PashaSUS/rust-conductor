use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::Message;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const CONSUMER_POOL_SIZE: usize = 4;

/// Kafka-backed task queue. Each task type maps to a Kafka topic
/// (`conductor.task.{task_type}`). Provides durable, at-least-once delivery
/// with automatic redelivery on consumer failure.
///
/// Uses a pool of consumers per task type to avoid mutex serialization
/// under high concurrency.
#[derive(Clone)]
pub struct KafkaTaskQueue {
    producer: FutureProducer,
    /// Per-task-type pool of consumers. Each entry is a Vec of mutex-wrapped
    /// consumers; callers round-robin across them to reduce contention.
    consumer_pools: Arc<std::sync::RwLock<HashMap<String, Arc<Vec<Arc<Mutex<StreamConsumer>>>>>>>,
    brokers: String,
    next_idx: Arc<std::sync::atomic::AtomicUsize>,
}

impl KafkaTaskQueue {
    pub fn new(brokers: &str) -> anyhow::Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "10000")
            .set("queue.buffering.max.messages", "100000")
            .set("batch.size", "65536")
            .set("linger.ms", "5")
            .set("acks", "all")
            .set("enable.idempotence", "true")
            .set("compression.type", "lz4")
            .create()
            .map_err(|e| anyhow::anyhow!("Failed to create Kafka producer: {e}"))?;

        tracing::info!(brokers = %brokers, "Kafka producer initialized");

        Ok(Self {
            producer,
            consumer_pools: Arc::new(std::sync::RwLock::new(HashMap::new())),
            brokers: brokers.to_string(),
            next_idx: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    fn topic_name(task_type: &str) -> String {
        format!("conductor.task.{task_type}")
    }

    /// Get or create a consumer pool for the given task type.
    /// Returns one consumer from the pool via round-robin to reduce contention.
    fn get_or_create_consumer(&self, task_type: &str) -> Arc<Mutex<StreamConsumer>> {
        let pool = self.get_or_create_pool(task_type);
        let idx = self.next_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % pool.len();
        pool[idx].clone()
    }

    fn get_or_create_pool(&self, task_type: &str) -> Arc<Vec<Arc<Mutex<StreamConsumer>>>> {
        // Fast path: read lock
        {
            let pools = self.consumer_pools.read().unwrap();
            if let Some(pool) = pools.get(task_type) {
                return pool.clone();
            }
        }

        // Slow path: create consumer pool outside lock, then insert
        let topic = Self::topic_name(task_type);
        let mut pool = Vec::with_capacity(CONSUMER_POOL_SIZE);
        for i in 0..CONSUMER_POOL_SIZE {
            let consumer: StreamConsumer = ClientConfig::new()
                .set("bootstrap.servers", &self.brokers)
                .set("group.id", "conductor-workers")
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest")
                .set("session.timeout.ms", "10000")
                .set("max.poll.interval.ms", "300000")
                .set("fetch.min.bytes", "1")
                .set("fetch.wait.max.ms", "100")
                .create()
                .expect("Failed to create Kafka consumer");

            consumer
                .subscribe(&[&topic])
                .expect("Failed to subscribe to Kafka topic");

            tracing::info!(task_type = %task_type, topic = %topic, pool_idx = i, "Created Kafka consumer");
            pool.push(Arc::new(Mutex::new(consumer)));
        }

        let arc_pool = Arc::new(pool);

        // Write lock to insert (double-check)
        let mut pools = self.consumer_pools.write().unwrap();
        pools
            .entry(task_type.to_string())
            .or_insert_with(|| arc_pool.clone());
        pools.get(task_type).unwrap().clone()
    }

    /// Enqueue a task_id onto the Kafka topic for the given task type.
    pub async fn enqueue(&self, task_type: &str, task_id: &str) -> Result<(), String> {
        let topic = Self::topic_name(task_type);
        self.producer
            .send(
                FutureRecord::to(&topic)
                    .key(task_id)
                    .payload(task_id),
                Duration::from_secs(5),
            )
            .await
            .map_err(|(e, _)| {
                tracing::error!(
                    task_type = %task_type,
                    task_id = %task_id,
                    error = %e,
                    "Kafka produce failed"
                );
                e.to_string()
            })?;
        Ok(())
    }

    /// Produce an arbitrary message to a given Kafka topic.
    pub async fn produce(&self, topic: &str, payload: &str) -> Result<(), String> {
        self.producer
            .send(
                FutureRecord::to(topic)
                    .key(topic)
                    .payload(payload),
                Duration::from_secs(5),
            )
            .await
            .map_err(|(e, _)| {
                tracing::error!(topic = %topic, error = %e, "Kafka produce failed");
                e.to_string()
            })?;
        Ok(())
    }

    /// Dequeue a single task_id from the Kafka topic for the given task type.
    /// Returns `None` if no message is available within the timeout.
    pub async fn dequeue(&self, task_type: &str) -> Option<String> {
        let consumer_handle = self.get_or_create_consumer(task_type);
        let consumer = consumer_handle.lock().await;
        match tokio::time::timeout(Duration::from_millis(100), consumer.recv()).await {
            Ok(Ok(msg)) => {
                let task_id = msg
                    .payload_view::<str>()
                    .and_then(|r| r.ok())
                    .map(|s| s.to_string());
                if task_id.is_some() {
                    let _ = consumer.commit_message(&msg, CommitMode::Async);
                }
                task_id
            }
            _ => None,
        }
    }

    /// Dequeue up to `count` task_ids from the topic, waiting up to `timeout`.
    pub async fn batch_dequeue(
        &self,
        task_type: &str,
        count: usize,
        timeout: Duration,
    ) -> Vec<String> {
        let consumer_handle = self.get_or_create_consumer(task_type);
        let consumer = consumer_handle.lock().await;
        let mut results = Vec::with_capacity(count);
        let deadline = tokio::time::Instant::now() + timeout;

        while results.len() < count {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let poll_time = remaining.min(Duration::from_millis(50));
            match tokio::time::timeout(poll_time, consumer.recv()).await {
                Ok(Ok(msg)) => {
                    if let Some(Ok(task_id)) = msg.payload_view::<str>() {
                        results.push(task_id.to_string());
                        let _ = consumer.commit_message(&msg, CommitMode::Async);
                    }
                }
                _ => break, // timeout or error — stop polling
            }
        }
        results
    }

    /// Check that the Kafka brokers are reachable.
    pub async fn health_check(&self) -> bool {
        // Try a metadata fetch with a short timeout
        let admin_client: Result<rdkafka::admin::AdminClient<rdkafka::client::DefaultClientContext>, _> =
            ClientConfig::new()
                .set("bootstrap.servers", &self.brokers)
                .set("request.timeout.ms", "3000")
                .create();
        match admin_client {
            Ok(client) => {
                let metadata = client.inner().fetch_metadata(None, Duration::from_secs(3));
                metadata.is_ok()
            }
            Err(_) => false,
        }
    }
}
