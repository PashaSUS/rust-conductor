# ── General ──
variable "aws_region" {
  type        = string
  default     = "us-east-1"
  description = "AWS region for all resources"
}

variable "environment" {
  type        = string
  default     = "staging"
  description = "Environment name (staging, production)"
  validation {
    condition     = contains(["staging", "production"], var.environment)
    error_message = "Environment must be staging or production."
  }
}

variable "project_name" {
  type    = string
  default = "rust-conductor"
}

# ── VPC ──
variable "vpc_cidr" {
  type    = string
  default = "10.0.0.0/16"
}

variable "availability_zones" {
  type    = list(string)
  default = ["us-east-1a", "us-east-1b", "us-east-1c"]
}

# ── EKS ──
variable "eks_cluster_version" {
  type    = string
  default = "1.30"
}

variable "eks_node_instance_types" {
  type    = list(string)
  default = ["t3.large"]
}

variable "eks_node_min_size" {
  type    = number
  default = 2
}

variable "eks_node_max_size" {
  type    = number
  default = 10
}

variable "eks_node_desired_size" {
  type    = number
  default = 3
}

# ── RDS (PostgreSQL) ──
variable "db_instance_class" {
  type    = string
  default = "db.r6g.large"
}

variable "db_allocated_storage" {
  type    = number
  default = 100
}

variable "db_multi_az" {
  type    = bool
  default = true
}

variable "db_backup_retention_period" {
  type    = number
  default = 30
}

variable "db_num_shards" {
  type        = number
  default     = 1
  description = "Number of PostgreSQL shard instances"
}

# ── ElastiCache (Redis) ──
variable "redis_node_type" {
  type    = string
  default = "cache.r6g.large"
}

variable "redis_num_shards" {
  type    = number
  default = 1
}

# ── MSK (Kafka) ──
variable "kafka_broker_instance_type" {
  type    = string
  default = "kafka.m5.large"
}

variable "kafka_broker_count" {
  type    = number
  default = 3
}

variable "kafka_ebs_volume_size" {
  type    = number
  default = 100
}
