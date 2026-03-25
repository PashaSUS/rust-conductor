use crate::store::postgres::DbPool;

/// Deterministic GUID-based sharding across N Postgres instances.
///
/// Routing: `workflow_id` is parsed as a UUID, the last 4 bytes are read as
/// a u32, and `u32 % num_shards` selects the shard index. All workflow data
/// (workflow row + tasks + task_logs) lives on the shard determined by its
/// `workflow_id`.
///
/// Metadata tables (workflow_def, task_def, event_handler, config) live on
/// **every** shard (migrations run on all shards), but are read/written from
/// shard 0 as the canonical source.
#[derive(Clone)]
pub struct ShardedPool {
    shards: Vec<DbPool>,
    /// Optional read replicas — one per shard. If present, read-only queries
    /// (search, stats, get) can be routed here to offload the primary.
    replicas: Vec<DbPool>,
}

impl ShardedPool {
    /// Create a new sharded pool from a list of connection pools.
    /// The order of pools defines shard indices (0, 1, 2, ...).
    pub fn new(shards: Vec<DbPool>) -> Self {
        assert!(!shards.is_empty(), "At least one shard is required");
        Self {
            shards,
            replicas: Vec::new(),
        }
    }

    /// Set optional read-replica pools (must match shard count).
    pub fn with_replicas(mut self, replicas: Vec<DbPool>) -> Self {
        if !replicas.is_empty() {
            assert_eq!(
                replicas.len(),
                self.shards.len(),
                "Replica count must match shard count"
            );
        }
        self.replicas = replicas;
        self
    }

    /// Number of shards.
    #[allow(dead_code)]
    pub fn num_shards(&self) -> usize {
        self.shards.len()
    }

    /// Get the shard pool for a given workflow_id.
    pub fn shard_for(&self, workflow_id: &str) -> &DbPool {
        let idx = self.shard_index(workflow_id);
        &self.shards[idx]
    }

    /// Compute the shard index for a workflow_id.
    pub fn shard_index(&self, workflow_id: &str) -> usize {
        let hash = workflow_id_to_u32(workflow_id);
        (hash as usize) % self.shards.len()
    }

    /// The primary shard (index 0) — used for metadata tables.
    pub fn primary(&self) -> &DbPool {
        &self.shards[0]
    }

    /// Iterate over all shard pools (for fan-out queries like search/stats).
    pub fn all_shards(&self) -> &[DbPool] {
        &self.shards
    }

    /// Read-replica pool for a given workflow_id. Falls back to the primary
    /// shard if no replica is configured.
    pub fn read_shard_for(&self, workflow_id: &str) -> &DbPool {
        if self.replicas.is_empty() {
            return self.shard_for(workflow_id);
        }
        let idx = self.shard_index(workflow_id);
        &self.replicas[idx]
    }

    /// Read-replica pools for fan-out queries. Falls back to primary shards
    /// if no replicas configured.
    pub fn read_shards(&self) -> &[DbPool] {
        if self.replicas.is_empty() {
            &self.shards
        } else {
            &self.replicas
        }
    }

    /// Read-replica for the primary shard (index 0). Falls back to primary.
    pub fn read_primary(&self) -> &DbPool {
        if self.replicas.is_empty() {
            &self.shards[0]
        } else {
            &self.replicas[0]
        }
    }

    /// Whether read replicas are configured.
    pub fn has_replicas(&self) -> bool {
        !self.replicas.is_empty()
    }
}

/// Deterministic hash: parse UUID bytes, take last 4 bytes as u32.
/// Falls back to byte-summing for non-UUID strings.
fn workflow_id_to_u32(workflow_id: &str) -> u32 {
    if let Ok(uuid) = uuid::Uuid::parse_str(workflow_id) {
        let bytes = uuid.as_bytes();
        u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]])
    } else {
        // Fallback: simple byte sum for non-UUID workflow ids
        workflow_id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_routing() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let a = workflow_id_to_u32(id);
        let b = workflow_id_to_u32(id);
        assert_eq!(a, b, "Same ID must always produce the same hash");
    }

    #[test]
    fn different_ids_may_differ() {
        let a = workflow_id_to_u32("550e8400-e29b-41d4-a716-446655440000");
        let b = workflow_id_to_u32("550e8400-e29b-41d4-a716-446655440001");
        // They *can* collide, but with different trailing bytes they shouldn't
        assert_ne!(a, b);
    }
}
