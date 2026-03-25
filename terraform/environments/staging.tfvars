# Staging environment
aws_region  = "us-east-1"
environment = "staging"

eks_node_instance_types = ["t3.large"]
eks_node_min_size       = 2
eks_node_max_size       = 5
eks_node_desired_size   = 2

db_instance_class          = "db.t4g.medium"
db_allocated_storage       = 50
db_multi_az                = false
db_backup_retention_period = 7
db_num_shards              = 1

redis_node_type  = "cache.t4g.medium"
redis_num_shards = 1

kafka_broker_instance_type = "kafka.t3.small"
kafka_broker_count         = 1
kafka_ebs_volume_size      = 50
