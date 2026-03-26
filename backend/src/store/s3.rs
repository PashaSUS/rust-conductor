//! S3-compatible external payload storage (MinIO, AWS S3, etc.)
//!
//! When enabled, large workflow/task payloads are automatically offloaded to
//! object storage instead of being stored inline in PostgreSQL JSONB columns.
//! The `external_input_payload_storage_path` and `external_output_payload_storage_path`
//! fields on workflow/task models store the S3 key, matching the official
//! Conductor external payload storage contract.
//!
//! Enable via the `external-storage` feature flag and set:
//!   S3_ENDPOINT=http://minio:9000
//!   S3_BUCKET=conductor-payloads
//!   S3_REGION=us-east-1          (default)
//!   S3_ACCESS_KEY=minioadmin
//!   S3_SECRET_KEY=minioadmin
//!   EXTERNAL_PAYLOAD_THRESHOLD=32768  (bytes, default 32KB)

use s3::Region;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use serde_json::Value;

/// Threshold in bytes above which payloads are externalized.
const DEFAULT_THRESHOLD: usize = 32 * 1024; // 32KB

#[derive(Clone)]
pub struct ExternalPayloadStorage {
    bucket: Box<Bucket>,
    #[allow(dead_code)]
    threshold: usize,
}

impl ExternalPayloadStorage {
    /// Create from environment variables.
    pub fn from_env() -> Result<Self, String> {
        let endpoint = std::env::var("S3_ENDPOINT").map_err(|_| "S3_ENDPOINT not set")?;
        let bucket_name =
            std::env::var("S3_BUCKET").unwrap_or_else(|_| "conductor-payloads".into());
        let region_name = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into());
        let access_key = std::env::var("S3_ACCESS_KEY").map_err(|_| "S3_ACCESS_KEY not set")?;
        let secret_key = std::env::var("S3_SECRET_KEY").map_err(|_| "S3_SECRET_KEY not set")?;

        let region = Region::Custom {
            region: region_name,
            endpoint,
        };

        let credentials = Credentials::new(Some(&access_key), Some(&secret_key), None, None, None)
            .map_err(|e| format!("Failed to create S3 credentials: {e}"))?;

        let mut bucket = Bucket::new(&bucket_name, region, credentials)
            .map_err(|e| format!("Failed to create S3 bucket handle: {e}"))?;

        // MinIO requires path-style addressing
        bucket.set_path_style();

        let threshold = std::env::var("EXTERNAL_PAYLOAD_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_THRESHOLD);

        Ok(Self { bucket, threshold })
    }

    /// Check if a payload exceeds the externalization threshold.
    #[allow(dead_code)]
    pub fn should_externalize(&self, payload: &Value) -> bool {
        // Quick estimate: serialize to check size
        match serde_json::to_vec(payload) {
            Ok(bytes) => bytes.len() > self.threshold,
            Err(_) => false,
        }
    }

    /// Upload a payload to S3 and return the storage path (key).
    ///
    /// Key format: `{entity_type}/{workflow_id}/{uuid}.json`
    #[allow(dead_code)]
    pub async fn upload(
        &self,
        entity_type: &str,
        workflow_id: &str,
        payload: &Value,
    ) -> Result<String, String> {
        let key = format!("{entity_type}/{workflow_id}/{}.json", uuid::Uuid::new_v4());
        let data =
            serde_json::to_vec(payload).map_err(|e| format!("Failed to serialize payload: {e}"))?;

        self.bucket
            .put_object(&key, &data)
            .await
            .map_err(|e| format!("S3 upload failed for key '{key}': {e}"))?;

        tracing::debug!(
            key = %key,
            size = data.len(),
            "Externalized payload to S3"
        );

        Ok(key)
    }

    /// Download a payload from S3 by its storage path (key).
    #[allow(dead_code)]
    pub async fn download(&self, key: &str) -> Result<Value, String> {
        let response = self
            .bucket
            .get_object(key)
            .await
            .map_err(|e| format!("S3 download failed for key '{key}': {e}"))?;

        serde_json::from_slice(response.as_slice())
            .map_err(|e| format!("Failed to parse S3 payload for key '{key}': {e}"))
    }

    /// Delete a payload from S3.
    #[allow(dead_code)]
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.bucket
            .delete_object(key)
            .await
            .map_err(|e| format!("S3 delete failed for key '{key}': {e}"))?;
        Ok(())
    }

    /// Ensure the bucket exists, creating it if necessary.
    pub async fn ensure_bucket(&self) -> Result<(), String> {
        // Try a HEAD request; if it fails, attempt creation
        match self.bucket.head_object("/").await {
            Ok(_) => Ok(()),
            Err(_) => {
                tracing::info!(bucket = %self.bucket.name(), "Creating S3 bucket");
                let creds = self
                    .bucket
                    .credentials()
                    .await
                    .map_err(|e| format!("Failed to get credentials: {e}"))?;
                Bucket::create_with_path_style(
                    &self.bucket.name(),
                    self.bucket.region().clone(),
                    creds,
                    s3::BucketConfiguration::default(),
                )
                .await
                .map_err(|e| format!("Failed to create bucket: {e}"))?;
                Ok(())
            }
        }
    }
}
