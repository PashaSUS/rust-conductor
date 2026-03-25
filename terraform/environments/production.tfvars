# Production environment
aws_region  = "us-east-1"
environment = "production"

availability_zones = ["us-east-1a", "us-east-1b", "us-east-1c"]

eks_node_instance_types = ["m6i.xlarge"]
eks_node_min_size       = 3
eks_node_max_size       = 20
eks_node_desired_size   = 5

db_instance_class          = "db.r6g.xlarge"
db_allocated_storage       = 200
db_multi_az                = true
db_backup_retention_period = 30
db_num_shards              = 2

redis_node_type  = "cache.r6g.large"
redis_num_shards = 2

kafka_broker_instance_type = "kafka.m5.large"
kafka_broker_count         = 3
kafka_ebs_volume_size      = 200
