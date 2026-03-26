/// Minimal Seq ingestion layer for `tracing`.
///
/// Ships structured CLEF events to a Seq server via HTTP POST to
/// `/api/events/raw`. Events are buffered in a bounded channel and
/// flushed in batches by a background Tokio task.
#[cfg(feature = "seq")]
mod inner {
    use std::fmt;
    use tokio::sync::mpsc;
    use tracing::Subscriber;
    use tracing::field::{Field, Visit};
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::Context;

    /// How many events we buffer before the oldest are dropped.
    const CHANNEL_CAPACITY: usize = 8192;
    /// Max events per HTTP POST.
    const BATCH_SIZE: usize = 256;
    /// Flush interval when batch is not full.
    const FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

    pub struct SeqLayer {
        tx: mpsc::Sender<String>,
    }

    impl SeqLayer {
        /// Create a new Seq layer and spawn the background flusher.
        ///
        /// `url` – Seq server base URL, e.g. `http://localhost:5341`
        /// `api_key` – optional Seq API key
        pub fn new(url: String, api_key: Option<String>) -> Self {
            let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
            tokio::spawn(flush_loop(rx, url, api_key));
            Self { tx }
        }
    }

    impl<S: Subscriber> Layer<S> for SeqLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let meta = event.metadata();

            let mut visitor = JsonVisitor::default();
            event.record(&mut visitor);

            let level = match *meta.level() {
                tracing::Level::ERROR => "Error",
                tracing::Level::WARN => "Warning",
                tracing::Level::INFO => "Information",
                tracing::Level::DEBUG => "Debug",
                tracing::Level::TRACE => "Verbose",
            };

            // Build CLEF envelope
            let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let message = visitor.message.as_deref().unwrap_or("");
            // Escape message for JSON safety
            let escaped_msg = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into());

            let mut clef = format!(
                r#"{{"@t":"{}","@l":"{}","@mt":{},"SourceContext":"{}""#,
                timestamp,
                level,
                escaped_msg,
                meta.target(),
            );

            for (k, v) in &visitor.fields {
                let escaped_val = serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into());
                clef.push_str(&format!(
                    r#","{}":"{}""#,
                    k,
                    v.replace('\\', "\\\\").replace('"', "\\\"")
                ));
                let _ = escaped_val; // use escaped_val for complex values
            }

            clef.push('}');

            // Non-blocking send — drop if buffer full
            let _ = self.tx.try_send(clef);
        }
    }

    /// Visitor that extracts fields from a tracing event.
    #[derive(Default)]
    struct JsonVisitor {
        message: Option<String>,
        fields: Vec<(String, String)>,
    }

    impl Visit for JsonVisitor {
        fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
            let val = format!("{:?}", value);
            if field.name() == "message" {
                self.message = Some(val);
            } else {
                self.fields.push((field.name().to_string(), val));
            }
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "message" {
                self.message = Some(value.to_string());
            } else {
                self.fields
                    .push((field.name().to_string(), value.to_string()));
            }
        }

        fn record_i64(&mut self, field: &Field, value: i64) {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }

        fn record_u64(&mut self, field: &Field, value: u64) {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }

        fn record_bool(&mut self, field: &Field, value: bool) {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    /// Background loop: drain events and POST to Seq in batches.
    async fn flush_loop(mut rx: mpsc::Receiver<String>, base_url: String, api_key: Option<String>) {
        let url = format!("{}/api/events/raw", base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to build reqwest client for Seq");

        let mut buf: Vec<String> = Vec::with_capacity(BATCH_SIZE);

        loop {
            // Wait for at least one event, or flush on timeout
            let event = tokio::time::timeout(FLUSH_INTERVAL, rx.recv()).await;

            match event {
                Ok(Some(line)) => {
                    buf.push(line);
                    // Drain as many ready events as possible up to BATCH_SIZE
                    while buf.len() < BATCH_SIZE {
                        match rx.try_recv() {
                            Ok(line) => buf.push(line),
                            Err(_) => break,
                        }
                    }
                }
                Ok(None) => {
                    // Channel closed — flush remaining and exit
                    if !buf.is_empty() {
                        let _ = send_batch(&client, &url, &api_key, &buf).await;
                    }
                    return;
                }
                Err(_) => {
                    // Timeout — flush whatever we have
                }
            }

            if !buf.is_empty() {
                if let Err(e) = send_batch(&client, &url, &api_key, &buf).await {
                    eprintln!("[seq] Failed to send batch ({} events): {e}", buf.len());
                }
                buf.clear();
            }
        }
    }

    async fn send_batch(
        client: &reqwest::Client,
        url: &str,
        api_key: &Option<String>,
        events: &[String],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let body = events.join("\n");
        let mut req = client
            .post(url)
            .header("Content-Type", "application/vnd.serilog.clef")
            .body(body);

        if let Some(key) = api_key {
            req = req.header("X-Seq-ApiKey", key);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Seq returned {status}: {text}").into());
        }
        Ok(())
    }
}

#[cfg(feature = "seq")]
pub use inner::SeqLayer;
