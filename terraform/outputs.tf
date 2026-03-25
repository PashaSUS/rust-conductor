# ── EKS Outputs ──
output "eks_cluster_name" {
  value = module.eks.cluster_name
}

output "eks_cluster_endpoint" {
  value = module.eks.cluster_endpoint
}

output "eks_cluster_ca_certificate" {
  value     = module.eks.cluster_certificate_authority_data
  sensitive = true
}

# ── RDS Outputs ──
output "rds_shard_endpoints" {
  value = [for db in aws_db_instance.shard : db.endpoint]
}

output "rds_shard_connection_urls" {
  value     = [for i, db in aws_db_instance.shard : "postgres://conductor:${random_password.db_password[i].result}@${db.endpoint}/conductor"]
  sensitive = true
}

# ── Redis Outputs ──
output "redis_primary_endpoint" {
  value = aws_elasticache_replication_group.conductor.primary_endpoint_address
}

output "redis_reader_endpoint" {
  value = aws_elasticache_replication_group.conductor.reader_endpoint_address
}

# ── Kafka Outputs ──
output "kafka_bootstrap_brokers" {
  value = aws_msk_cluster.conductor.bootstrap_brokers
}

output "kafka_bootstrap_brokers_tls" {
  value = aws_msk_cluster.conductor.bootstrap_brokers_tls
}

# ── S3 Outputs ──
output "backup_bucket_name" {
  value = aws_s3_bucket.backups.id
}

output "payload_bucket_name" {
  value = aws_s3_bucket.payloads.id
}

# ── Helm values snippet ──
output "helm_values_snippet" {
  description = "Paste into your Helm values override for this environment"
  sensitive   = true
  value       = <<-EOT
    # Auto-generated from Terraform output
    postgresql:
      enabled: false
    externalDatabase:
      host: ${aws_db_instance.shard[0].address}
      port: 5432
      user: conductor
      password: ${random_password.db_password[0].result}
      database: conductor
    redis:
      enabled: false
    externalRedis:
      host: ${aws_elasticache_replication_group.conductor.primary_endpoint_address}
      port: 6379
    kafka:
      enabled: false
    externalKafka:
      brokers: ${aws_msk_cluster.conductor.bootstrap_brokers}
    backend:
      env:
        SHARD_DATABASE_URLS: ${join(",", [for i, db in aws_db_instance.shard : "postgres://conductor:${random_password.db_password[i].result}@${db.endpoint}/conductor"])}
        REDIS_URLS: "redis://${aws_elasticache_replication_group.conductor.primary_endpoint_address}:6379"
        KAFKA_BROKERS: ${aws_msk_cluster.conductor.bootstrap_brokers}
        S3_BUCKET: ${aws_s3_bucket.payloads.id}
        S3_REGION: ${var.aws_region}
  EOT
}
