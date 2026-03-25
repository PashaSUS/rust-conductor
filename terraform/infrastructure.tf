# ── VPC ──────────────────────────────────────────────────────────────
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "${var.project_name}-${var.environment}"
  cidr = var.vpc_cidr

  azs             = var.availability_zones
  private_subnets = [for i, az in var.availability_zones : cidrsubnet(var.vpc_cidr, 4, i)]
  public_subnets  = [for i, az in var.availability_zones : cidrsubnet(var.vpc_cidr, 4, i + length(var.availability_zones))]

  enable_nat_gateway   = true
  single_nat_gateway   = var.environment == "staging"
  enable_dns_hostnames = true
  enable_dns_support   = true

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb"                               = "1"
    "kubernetes.io/cluster/${var.project_name}-${var.environment}" = "shared"
  }

  public_subnet_tags = {
    "kubernetes.io/role/elb"                                        = "1"
    "kubernetes.io/cluster/${var.project_name}-${var.environment}" = "shared"
  }
}

# ── EKS Cluster ─────────────────────────────────────────────────────
module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 20.0"

  cluster_name    = "${var.project_name}-${var.environment}"
  cluster_version = var.eks_cluster_version

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  cluster_endpoint_public_access = true

  eks_managed_node_groups = {
    default = {
      ami_type       = "AL2_x86_64"
      instance_types = var.eks_node_instance_types
      min_size       = var.eks_node_min_size
      max_size       = var.eks_node_max_size
      desired_size   = var.eks_node_desired_size

      labels = {
        workload = "conductor"
      }
    }
  }

  # Cluster add-ons
  cluster_addons = {
    coredns                = { most_recent = true }
    kube-proxy             = { most_recent = true }
    vpc-cni                = { most_recent = true }
    aws-ebs-csi-driver     = { most_recent = true }
  }
}

# ── RDS PostgreSQL (per shard) ──────────────────────────────────────
resource "aws_db_subnet_group" "conductor" {
  name       = "${var.project_name}-${var.environment}"
  subnet_ids = module.vpc.private_subnets
}

resource "aws_security_group" "rds" {
  name_prefix = "${var.project_name}-rds-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [module.eks.node_security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_db_instance" "shard" {
  count = var.db_num_shards

  identifier = "${var.project_name}-${var.environment}-shard-${count.index}"

  engine         = "postgres"
  engine_version = "16.4"
  instance_class = var.db_instance_class

  allocated_storage     = var.db_allocated_storage
  max_allocated_storage = var.db_allocated_storage * 4
  storage_encrypted     = true

  db_name  = "conductor"
  username = "conductor"
  password = random_password.db_password[count.index].result

  multi_az            = var.db_multi_az
  db_subnet_group_name   = aws_db_subnet_group.conductor.name
  vpc_security_group_ids = [aws_security_group.rds.id]

  backup_retention_period = var.db_backup_retention_period
  backup_window           = "03:00-04:00"
  maintenance_window      = "sun:04:00-sun:05:00"

  deletion_protection = var.environment == "production"
  skip_final_snapshot = var.environment == "staging"

  performance_insights_enabled = true

  parameter_group_name = aws_db_parameter_group.conductor.name
}

resource "aws_db_parameter_group" "conductor" {
  family = "postgres16"
  name   = "${var.project_name}-${var.environment}"

  parameter {
    name  = "max_connections"
    value = "200"
  }

  parameter {
    name  = "shared_buffers"
    value = "{DBInstanceClassMemory/4}"
  }

  parameter {
    name  = "work_mem"
    value = "8192"
  }

  parameter {
    name  = "log_min_duration_statement"
    value = "500"
  }
}

resource "random_password" "db_password" {
  count   = var.db_num_shards
  length  = 32
  special = false
}

# ── ElastiCache Redis ───────────────────────────────────────────────
resource "aws_security_group" "redis" {
  name_prefix = "${var.project_name}-redis-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [module.eks.node_security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_elasticache_subnet_group" "conductor" {
  name       = "${var.project_name}-${var.environment}"
  subnet_ids = module.vpc.private_subnets
}

resource "aws_elasticache_replication_group" "conductor" {
  replication_group_id = "${var.project_name}-${var.environment}"
  description          = "Redis for rust-conductor ${var.environment}"

  node_type            = var.redis_node_type
  num_cache_clusters   = var.redis_num_shards > 1 ? null : 2
  num_node_groups      = var.redis_num_shards > 1 ? var.redis_num_shards : null
  replicas_per_node_group = var.redis_num_shards > 1 ? 1 : null

  engine_version       = "7.1"
  port                 = 6379
  subnet_group_name    = aws_elasticache_subnet_group.conductor.name
  security_group_ids   = [aws_security_group.redis.id]

  at_rest_encryption_enabled = true
  transit_encryption_enabled = false  # Enable if clients support TLS

  automatic_failover_enabled = true
  multi_az_enabled           = var.environment == "production"

  snapshot_retention_limit = var.environment == "production" ? 7 : 1
  snapshot_window          = "02:00-03:00"
}

# ── MSK (Kafka) ─────────────────────────────────────────────────────
resource "aws_security_group" "kafka" {
  name_prefix = "${var.project_name}-kafka-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 9092
    to_port         = 9098
    protocol        = "tcp"
    security_groups = [module.eks.node_security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_msk_cluster" "conductor" {
  cluster_name           = "${var.project_name}-${var.environment}"
  kafka_version          = "3.6.0"
  number_of_broker_nodes = var.kafka_broker_count

  broker_node_group_info {
    instance_type   = var.kafka_broker_instance_type
    client_subnets  = module.vpc.private_subnets
    security_groups = [aws_security_group.kafka.id]

    storage_info {
      ebs_storage_info {
        volume_size = var.kafka_ebs_volume_size
      }
    }
  }

  encryption_info {
    encryption_in_transit {
      client_broker = "TLS_PLAINTEXT"
      in_cluster    = true
    }
  }

  logging_info {
    broker_logs {
      cloudwatch_logs {
        enabled   = true
        log_group = aws_cloudwatch_log_group.kafka.name
      }
    }
  }
}

resource "aws_cloudwatch_log_group" "kafka" {
  name              = "/aws/msk/${var.project_name}-${var.environment}"
  retention_in_days = 14
}

# ── S3 for backups & external payloads ──────────────────────────────
resource "aws_s3_bucket" "backups" {
  bucket = "${var.project_name}-${var.environment}-backups"
}

resource "aws_s3_bucket_lifecycle_configuration" "backups" {
  bucket = aws_s3_bucket.backups.id

  rule {
    id     = "expire-old-backups"
    status = "Enabled"

    expiration {
      days = var.db_backup_retention_period
    }

    transition {
      days          = 7
      storage_class = "GLACIER_IR"
    }
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "backups" {
  bucket = aws_s3_bucket.backups.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "aws:kms"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "backups" {
  bucket = aws_s3_bucket.backups.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket" "payloads" {
  bucket = "${var.project_name}-${var.environment}-payloads"
}

resource "aws_s3_bucket_server_side_encryption_configuration" "payloads" {
  bucket = aws_s3_bucket.payloads.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "aws:kms"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "payloads" {
  bucket = aws_s3_bucket.payloads.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# ── Secrets Manager ─────────────────────────────────────────────────
resource "aws_secretsmanager_secret" "db_credentials" {
  count = var.db_num_shards
  name  = "${var.project_name}/${var.environment}/db-shard-${count.index}"
}

resource "aws_secretsmanager_secret_version" "db_credentials" {
  count     = var.db_num_shards
  secret_id = aws_secretsmanager_secret.db_credentials[count.index].id
  secret_string = jsonencode({
    username = "conductor"
    password = random_password.db_password[count.index].result
    host     = aws_db_instance.shard[count.index].address
    port     = 5432
    dbname   = "conductor"
  })
}
