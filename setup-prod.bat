@echo off
setlocal enabledelayedexpansion

echo ========================================
echo  rust-conductor Setup - PROD Mode
echo ========================================
echo.
echo Full production setup with ALL features enabled:
echo   - Kafka (event streaming)
echo   - GraphQL API
echo   - Server-Sent Events (SSE)
echo   - WebSocket real-time feeds
echo   - gRPC with reflection
echo   - API v2 endpoints
echo   - Structured logging (Seq)
echo   - Connection pooling (PgBouncer)
echo   - Load balancing (Nginx)
echo   - Multi-shard scaling
echo   - External storage (optional, RustFS/S3)
echo.
echo DEV mode uses only Redis Streams (no Kafka) and minimal features.
echo PROD enables the full "feature-rich" Cargo build profile.
echo.


REM -- Shard configuration --
:ask_shards
set /p NUM_SHARDS="How many database shards do you want? (1-16): "
set "valid="
for /l %%n in (1,1,16) do if "!NUM_SHARDS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 16.
    goto ask_shards
)

:ask_replicas
set /p NUM_REPLICAS="How many backend replicas do you want? (1-8): "
set "valid="
for /l %%n in (1,1,8) do if "!NUM_REPLICAS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 8.
    goto ask_replicas
)

:ask_redis_shards
set /p NUM_REDIS_SHARDS="How many Redis shards do you want? (1-8): "
set "valid="
for /l %%n in (1,1,8) do if "!NUM_REDIS_SHARDS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 8.
    goto ask_redis_shards
)

:ask_kafka_brokers
set /p NUM_KAFKA_BROKERS="How many Kafka brokers do you want? (1-4): "
set "valid="
for /l %%n in (1,1,4) do if "!NUM_KAFKA_BROKERS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 4.
    goto ask_kafka_brokers
)


REM -- Port configuration --
echo.
echo ========================================
echo  Port Configuration
echo ========================================
echo.
echo Default ports: Backend=8090, gRPC=50055, Frontend=3170, Seq=9321
echo.

:ask_ports
set /p RANDOMIZE_PORTS="Randomize ports? (y/N): "
if /i "!RANDOMIZE_PORTS!"=="y" (
    REM Generate pseudo-random offsets using %time% centiseconds
    set "T=!time:~-2!"
    set /a "T=1!T! - 100"
    set /a BACKEND_PORT=8100 + !T! %% 900
    set /a GRPC_PORT=50100 + !T! %% 900
    set /a FRONTEND_PORT=3200 + !T! %% 800
    set /a SEQ_PORT=9400 + !T! %% 500
    echo.
    echo Generated ports:
    echo   Backend API:  !BACKEND_PORT!
    echo   gRPC:         !GRPC_PORT!
    echo   Frontend:     !FRONTEND_PORT!
    echo   Seq Logs:     !SEQ_PORT!
    echo.
    set /p CONFIRM_PORTS="Accept these ports? (Y/n): "
    if /i "!CONFIRM_PORTS!"=="n" goto ask_ports
) else (
    set "BACKEND_PORT=8090"
    set "GRPC_PORT=50055"
    set "FRONTEND_PORT=3170"
    set "SEQ_PORT=9321"
    echo.
    echo Using default ports. You can override each one:
    set /p "BACKEND_PORT=Backend API port [!BACKEND_PORT!]: " || set "BACKEND_PORT=!BACKEND_PORT!"
    set /p "GRPC_PORT=gRPC port [!GRPC_PORT!]: " || set "GRPC_PORT=!GRPC_PORT!"
    set /p "FRONTEND_PORT=Frontend port [!FRONTEND_PORT!]: " || set "FRONTEND_PORT=!FRONTEND_PORT!"
    set /p "SEQ_PORT=Seq log viewer port [!SEQ_PORT!]: " || set "SEQ_PORT=!SEQ_PORT!"
)


REM -- External payload storage (RustFS / S3) --
echo.
echo ========================================
echo  External Payload Storage
echo ========================================
echo.
echo RustFS provides S3-compatible object storage for large workflow/task
echo payloads. When enabled, payloads exceeding 32KB are offloaded to
echo object storage instead of being stored in PostgreSQL.
echo.

set "USE_MINIO=N"
set /p USE_MINIO="Enable external payload storage (RustFS/S3)? (y/N): "
if /i "!USE_MINIO!"=="y" (
    set "MINIO_PORT=9000"
    set "MINIO_CONSOLE_PORT=9001"
    set /p "MINIO_PORT=RustFS API port [!MINIO_PORT!]: " || set "MINIO_PORT=!MINIO_PORT!"
    set /p "MINIO_CONSOLE_PORT=RustFS Console port [!MINIO_CONSOLE_PORT!]: " || set "MINIO_CONSOLE_PORT=!MINIO_CONSOLE_PORT!"
    echo.
    echo RustFS will be available at:
    echo   API:     http://localhost:!MINIO_PORT!
    echo   Console: http://localhost:!MINIO_CONSOLE_PORT!
    echo   Default credentials: rustfsadmin / rustfsadmin
    echo.
)


REM -- URL configuration --
echo.
echo ========================================
echo  URL Configuration
echo ========================================
echo.
echo The frontend serves the UI and proxies /api requests to the backend.
echo Both share a single public URL. For production, enter your domain.
echo.

set "DEFAULT_PUBLIC_URL=http://localhost:!FRONTEND_PORT!"

set /p "PUBLIC_URL=Public URL [!DEFAULT_PUBLIC_URL!]: " || set "PUBLIC_URL=!DEFAULT_PUBLIC_URL!"
if "!PUBLIC_URL!"=="" set "PUBLIC_URL=!DEFAULT_PUBLIC_URL!"

echo.
echo Configuration:
echo   Public URL:   !PUBLIC_URL!
echo   API endpoint: !PUBLIC_URL!/api  - proxied to backend internally
echo   CORS origin:  !PUBLIC_URL!
echo.


REM -- Generate nginx-lb.conf --
echo Generating nginx-lb.conf...
set "NLB=nginx-lb.conf"
> "!NLB!" echo resolver 127.0.0.11 valid=10s ipv6=off;
>> "!NLB!" echo.
>> "!NLB!" echo server {
>> "!NLB!" echo     listen !BACKEND_PORT!;
>> "!NLB!" echo.
>> "!NLB!" echo     location / {
>> "!NLB!" echo         set $backend http://backend:!BACKEND_PORT!;
>> "!NLB!" echo         proxy_pass $backend;
>> "!NLB!" echo         proxy_http_version 1.1;
>> "!NLB!" echo         proxy_set_header Host $host;
>> "!NLB!" echo         proxy_set_header X-Real-IP $remote_addr;
>> "!NLB!" echo         proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
>> "!NLB!" echo         proxy_set_header X-Forwarded-Proto $scheme;
>> "!NLB!" echo         proxy_read_timeout 300s;
>> "!NLB!" echo         proxy_connect_timeout 5s;
>> "!NLB!" echo         proxy_send_timeout 60s;
>> "!NLB!" echo.
>> "!NLB!" echo         # Allow large workflow/task payloads
>> "!NLB!" echo         client_max_body_size 10m;
>> "!NLB!" echo     }
>> "!NLB!" echo }
>> "!NLB!" echo.
>> "!NLB!" echo # gRPC load balancer
>> "!NLB!" echo server {
>> "!NLB!" echo     listen !GRPC_PORT! http2;
>> "!NLB!" echo.
>> "!NLB!" echo     http2_max_concurrent_streams 1024;
>> "!NLB!" echo     http2_max_requests           100000;
>> "!NLB!" echo.
>> "!NLB!" echo     location / {
>> "!NLB!" echo         set $grpc_backend grpc://backend:!GRPC_PORT!;
>> "!NLB!" echo         grpc_pass $grpc_backend;
>> "!NLB!" echo         grpc_read_timeout 300s;
>> "!NLB!" echo         grpc_send_timeout 60s;
>> "!NLB!" echo         grpc_connect_timeout 5s;
>> "!NLB!" echo         grpc_buffer_size 64k;
>> "!NLB!" echo     }
>> "!NLB!" echo }

REM -- frontend/nginx.conf no longer needed (using serve) --

echo.
echo Configuring %NUM_SHARDS% shards [PROD mode]...
echo.


REM ================================================================
REM  Generate docker-compose.yml
REM ================================================================
set "FILE=docker-compose.yml"
> "%FILE%" echo services:

REM -- Postgres shards --
set /a LAST_SHARD=%NUM_SHARDS%-1
for /l %%i in (0,1,%LAST_SHARD%) do (
    set /a PG_PORT=5432+%%i
    >> "%FILE%" echo   # -- Postgres Shard %%i --
    >> "%FILE%" echo   postgres-shard-%%i:
    >> "%FILE%" echo     image: postgres:16-alpine
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     command: ^>
    >> "%FILE%" echo       postgres
    >> "%FILE%" echo       -c max_locks_per_transaction=256
    >> "%FILE%" echo       -c max_connections=200
    >> "%FILE%" echo       -c shared_buffers=256MB
    >> "%FILE%" echo       -c work_mem=8MB
    >> "%FILE%" echo       -c effective_cache_size=512MB
    >> "%FILE%" echo     environment:
    >> "%FILE%" echo       POSTGRES_USER: conductor
    >> "%FILE%" echo       POSTGRES_PASSWORD: conductor
    >> "%FILE%" echo       POSTGRES_DB: conductor
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!PG_PORT!:5432"
    >> "%FILE%" echo     volumes:
    >> "%FILE%" echo       - pgdata_shard%%i:/var/lib/postgresql/data
    >> "%FILE%" echo     healthcheck:
    >> "%FILE%" echo       test: ["CMD-SHELL", "pg_isready -U conductor"]
    >> "%FILE%" echo       interval: 5s
    >> "%FILE%" echo       timeout: 5s
    >> "%FILE%" echo       retries: 5
    >> "%FILE%" echo.
)

REM -- PgBouncer instances --
for /l %%i in (0,1,%LAST_SHARD%) do (
    set /a PGB_PORT=6432+%%i
    >> "%FILE%" echo   # PgBouncer - connection pooler in front of shard %%i
    >> "%FILE%" echo   pgbouncer-shard-%%i:
    >> "%FILE%" echo     image: edoburu/pgbouncer:latest
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     environment:
    >> "%FILE%" echo       DB_HOST: postgres-shard-%%i
    >> "%FILE%" echo       DB_PORT: "5432"
    >> "%FILE%" echo       DB_USER: conductor
    >> "%FILE%" echo       DB_PASSWORD: conductor
    >> "%FILE%" echo       DB_NAME: conductor
    >> "%FILE%" echo       POOL_MODE: transaction
    >> "%FILE%" echo       MAX_CLIENT_CONN: "400"
    >> "%FILE%" echo       DEFAULT_POOL_SIZE: "40"
    >> "%FILE%" echo       MIN_POOL_SIZE: "10"
    >> "%FILE%" echo       RESERVE_POOL_SIZE: "10"
    >> "%FILE%" echo       LISTEN_PORT: "6432"
    >> "%FILE%" echo       AUTH_TYPE: scram-sha-256
    >> "%FILE%" echo       IGNORE_STARTUP_PARAMETERS: extra_float_digits
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!PGB_PORT!:6432"
    >> "%FILE%" echo     depends_on:
    >> "%FILE%" echo       postgres-shard-%%i:
    >> "%FILE%" echo         condition: service_healthy
    >> "%FILE%" echo     healthcheck:
    >> "%FILE%" echo       test: ["CMD-SHELL", "pg_isready -h 127.0.0.1 -p 6432 -U conductor"]
    >> "%FILE%" echo       interval: 5s
    >> "%FILE%" echo       timeout: 5s
    >> "%FILE%" echo       retries: 5
    >> "%FILE%" echo.
)


REM -- Redis Shards --
set /a LAST_REDIS_SHARD=%NUM_REDIS_SHARDS%-1
set "REDIS_URLS="
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    set /a REDIS_PORT=6379+%%r
    >> "%FILE%" echo   redis-%%r:
    >> "%FILE%" echo     image: redis:7-alpine
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!REDIS_PORT!:6379"
    >> "%FILE%" echo     volumes:
    >> "%FILE%" echo       - redisdata-%%r:/data
    >> "%FILE%" echo     healthcheck:
    >> "%FILE%" echo       test: ["CMD", "redis-cli", "ping"]
    >> "%FILE%" echo       interval: 5s
    >> "%FILE%" echo       timeout: 5s
    >> "%FILE%" echo       retries: 5
    >> "%FILE%" echo.
    if "!REDIS_URLS!"=="" (
        set "REDIS_URLS=redis://redis-%%r:6379"
    ) else (
        set "REDIS_URLS=!REDIS_URLS!,redis://redis-%%r:6379"
    )
)


REM -- Seq (structured log server) --
>> "%FILE%" echo   # Seq - structured log server
>> "%FILE%" echo   seq:
>> "%FILE%" echo     image: datalust/seq:latest
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     environment:
>> "%FILE%" echo       ACCEPT_EULA: "Y"
>> "%FILE%" echo       SEQ_FIRSTRUN_NOAUTHENTICATION: "true"
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "!SEQ_PORT!:80"
>> "%FILE%" echo     volumes:
>> "%FILE%" echo       - seqdata:/data
>> "%FILE%" echo     healthcheck:
>> "%FILE%" echo       test: ["CMD", "curl", "-f", "http://localhost:80/api"]
>> "%FILE%" echo       interval: 10s
>> "%FILE%" echo       timeout: 5s
>> "%FILE%" echo       retries: 5
>> "%FILE%" echo.


REM -- Kafka Brokers (KRaft mode) --
set /a LAST_KAFKA=%NUM_KAFKA_BROKERS%-1

REM Build controller quorum voters string
set "KAFKA_VOTERS="
for /l %%k in (0,1,%LAST_KAFKA%) do (
    set /a VOTER_ID=%%k+1
    if "!KAFKA_VOTERS!"=="" (
        set "KAFKA_VOTERS=!VOTER_ID!@kafka-%%k:9093"
    ) else (
        set "KAFKA_VOTERS=!KAFKA_VOTERS!,!VOTER_ID!@kafka-%%k:9093"
    )
)

REM Build KAFKA_BROKERS connection string
set "KAFKA_BROKERS="
for /l %%k in (0,1,%LAST_KAFKA%) do (
    if "!KAFKA_BROKERS!"=="" (
        set "KAFKA_BROKERS=kafka-%%k:9092"
    ) else (
        set "KAFKA_BROKERS=!KAFKA_BROKERS!,kafka-%%k:9092"
    )
)

for /l %%k in (0,1,%LAST_KAFKA%) do (
    set /a KAFKA_PORT=9092+%%k
    set /a KAFKA_NODE=%%k+1
    if !NUM_KAFKA_BROKERS! GTR 1 (
        set "KAFKA_RF=3"
        if !NUM_KAFKA_BROKERS! LSS 3 set "KAFKA_RF=!NUM_KAFKA_BROKERS!"
    ) else (
        set "KAFKA_RF=1"
    )
    >> "%FILE%" echo   # -- Kafka Broker %%k --
    >> "%FILE%" echo   kafka-%%k:
    >> "%FILE%" echo     image: apache/kafka:3.8.0
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!KAFKA_PORT!:9092"
    >> "%FILE%" echo     environment:
    >> "%FILE%" echo       KAFKA_NODE_ID: "!KAFKA_NODE!"
    >> "%FILE%" echo       KAFKA_PROCESS_ROLES: "broker,controller"
    >> "%FILE%" echo       KAFKA_CONTROLLER_QUORUM_VOTERS: "!KAFKA_VOTERS!"
    >> "%FILE%" echo       KAFKA_LISTENERS: "PLAINTEXT://0.0.0.0:9092,CONTROLLER://0.0.0.0:9093"
    >> "%FILE%" echo       KAFKA_ADVERTISED_LISTENERS: "PLAINTEXT://kafka-%%k:9092"
    >> "%FILE%" echo       KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: "PLAINTEXT:PLAINTEXT,CONTROLLER:PLAINTEXT"
    >> "%FILE%" echo       KAFKA_CONTROLLER_LISTENER_NAMES: "CONTROLLER"
    >> "%FILE%" echo       KAFKA_INTER_BROKER_LISTENER_NAME: "PLAINTEXT"
    >> "%FILE%" echo       KAFKA_LOG_DIRS: "/var/lib/kafka/data"
    >> "%FILE%" echo       KAFKA_AUTO_CREATE_TOPICS_ENABLE: "true"
    >> "%FILE%" echo       KAFKA_NUM_PARTITIONS: "%NUM_REPLICAS%"
    >> "%FILE%" echo       KAFKA_DEFAULT_REPLICATION_FACTOR: "!KAFKA_RF!"
    >> "%FILE%" echo       KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: "!KAFKA_RF!"
    >> "%FILE%" echo       KAFKA_LOG_RETENTION_HOURS: "168"
    >> "%FILE%" echo       KAFKA_LOG_SEGMENT_BYTES: "1073741824"
    >> "%FILE%" echo       CLUSTER_ID: "conductor-kafka-cluster-001"
    >> "%FILE%" echo     volumes:
    >> "%FILE%" echo       - kafkadata-%%k:/var/lib/kafka/data
    >> "%FILE%" echo     healthcheck:
    >> "%FILE%" echo       test: ["CMD-SHELL", "nc -z localhost 9092"]
    >> "%FILE%" echo       interval: 10s
    >> "%FILE%" echo       timeout: 10s
    >> "%FILE%" echo       retries: 10
    >> "%FILE%" echo       start_period: 30s
    >> "%FILE%" echo.
)


REM -- RustFS (optional) --
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo   # RustFS - S3-compatible object storage
    >> "%FILE%" echo   rustfs:
    >> "%FILE%" echo     image: rustfs/rustfs:latest
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!MINIO_PORT!:9000"
    >> "%FILE%" echo       - "!MINIO_CONSOLE_PORT!:9001"
    >> "%FILE%" echo     volumes:
    >> "%FILE%" echo       - rustfsdata:/data
    >> "%FILE%" echo       - rustfslogs:/logs
    >> "%FILE%" echo     healthcheck:
    >> "%FILE%" echo       test: ["CMD-SHELL", "nc -z localhost 9000"]
    >> "%FILE%" echo       interval: 10s
    >> "%FILE%" echo       timeout: 5s
    >> "%FILE%" echo       retries: 5
    >> "%FILE%" echo.
)


REM -- Compute CARGO_FEATURES for PROD --
REM PROD always builds with all features. If external storage (RustFS/S3) is
REM enabled, we add the "external-storage" feature on top.
if /i "!USE_MINIO!"=="y" (
    set "CARGO_FEATURES=feature-rich"
) else (
    set "CARGO_FEATURES=kafka,graphql,sse,websocket,grpc-reflection,api-v2,seq"
)
echo.
echo Cargo features: !CARGO_FEATURES!
echo.

REM -- Backend Migrator --
REM Always connects directly to postgres (migrations use DDL/advisory locks
REM which are incompatible with pgbouncer's transaction pooling mode).
>> "%FILE%" echo   backend-migrate:
>> "%FILE%" echo     build:
>> "%FILE%" echo       context: ./backend
>> "%FILE%" echo       dockerfile: Dockerfile
>> "%FILE%" echo       args:
>> "%FILE%" echo         CARGO_FEATURES: "!CARGO_FEATURES!"
>> "%FILE%" echo     environment:
>> "%FILE%" echo       HOST: "0.0.0.0"
>> "%FILE%" echo       PORT: "!BACKEND_PORT!"
>> "%FILE%" echo       NUM_SHARDS: "%NUM_SHARDS%"
>> "%FILE%" echo       SHARD_DB_URL_TEMPLATE: "postgres://conductor:conductor@postgres-shard-{i}:5432/conductor"
>> "%FILE%" echo       REDIS_URLS: "!REDIS_URLS!"
>> "%FILE%" echo       KAFKA_BROKERS: "!KAFKA_BROKERS!"
>> "%FILE%" echo       RUST_LOG: "info"
>> "%FILE%" echo       MIGRATE_ONLY: "true"
>> "%FILE%" echo       SEQ_URL: "http://seq:80"
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       S3_ENDPOINT: "http://rustfs:9000"
    >> "%FILE%" echo       S3_BUCKET: "conductor-payloads"
    >> "%FILE%" echo       S3_REGION: "us-east-1"
    >> "%FILE%" echo       S3_ACCESS_KEY: "rustfsadmin"
    >> "%FILE%" echo       S3_SECRET_KEY: "rustfsadmin"
)
>> "%FILE%" echo     depends_on:
for /l %%i in (0,1,%LAST_SHARD%) do (
    >> "%FILE%" echo       postgres-shard-%%i:
    >> "%FILE%" echo         condition: service_healthy
)
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    >> "%FILE%" echo       redis-%%r:
    >> "%FILE%" echo         condition: service_healthy
)
for /l %%k in (0,1,%LAST_KAFKA%) do (
    >> "%FILE%" echo       kafka-%%k:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo       seq:
>> "%FILE%" echo         condition: service_healthy
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       rustfs:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo.


REM -- Backend --
REM PROD: connect via pgbouncer, expose to nginx-lb only
>> "%FILE%" echo   backend:
>> "%FILE%" echo     build:
>> "%FILE%" echo       context: ./backend
>> "%FILE%" echo       dockerfile: Dockerfile
>> "%FILE%" echo       args:
>> "%FILE%" echo         CARGO_FEATURES: "!CARGO_FEATURES!"
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     expose:
>> "%FILE%" echo       - "!BACKEND_PORT!"
>> "%FILE%" echo       - "!GRPC_PORT!"
>> "%FILE%" echo     environment:
>> "%FILE%" echo       HOST: "0.0.0.0"
>> "%FILE%" echo       PORT: "!BACKEND_PORT!"
>> "%FILE%" echo       GRPC_PORT: "!GRPC_PORT!"
>> "%FILE%" echo       NUM_SHARDS: "%NUM_SHARDS%"
>> "%FILE%" echo       SHARD_DB_URL_TEMPLATE: "postgres://conductor:conductor@pgbouncer-shard-{i}:6432/conductor"
>> "%FILE%" echo       REDIS_URLS: "!REDIS_URLS!"
>> "%FILE%" echo       KAFKA_BROKERS: "!KAFKA_BROKERS!"
>> "%FILE%" echo       CORS_ORIGIN: "!PUBLIC_URL!"
>> "%FILE%" echo       RUST_LOG: "warn"
>> "%FILE%" echo       SKIP_MIGRATIONS: "true"
>> "%FILE%" echo       SEQ_URL: "http://seq:80"
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       S3_ENDPOINT: "http://rustfs:9000"
    >> "%FILE%" echo       S3_BUCKET: "conductor-payloads"
    >> "%FILE%" echo       S3_REGION: "us-east-1"
    >> "%FILE%" echo       S3_ACCESS_KEY: "rustfsadmin"
    >> "%FILE%" echo       S3_SECRET_KEY: "rustfsadmin"
)
>> "%FILE%" echo     deploy:
>> "%FILE%" echo       replicas: %NUM_REPLICAS%
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       backend-migrate:
>> "%FILE%" echo         condition: service_completed_successfully
for /l %%i in (0,1,%LAST_SHARD%) do (
    >> "%FILE%" echo       pgbouncer-shard-%%i:
    >> "%FILE%" echo         condition: service_healthy
)
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    >> "%FILE%" echo       redis-%%r:
    >> "%FILE%" echo         condition: service_healthy
)
for /l %%k in (0,1,%LAST_KAFKA%) do (
    >> "%FILE%" echo       kafka-%%k:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo       seq:
>> "%FILE%" echo         condition: service_healthy
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       rustfs:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo.

REM -- Nginx Load Balancer --
>> "%FILE%" echo   nginx-lb:
>> "%FILE%" echo     image: nginx:alpine
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "!BACKEND_PORT!:!BACKEND_PORT!"
>> "%FILE%" echo       - "!GRPC_PORT!:!GRPC_PORT!"
>> "%FILE%" echo     volumes:
>> "%FILE%" echo       - ./nginx-lb.conf:/etc/nginx/conf.d/default.conf:ro
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       - backend
>> "%FILE%" echo.

REM -- Frontend --
>> "%FILE%" echo   frontend:
>> "%FILE%" echo     build:
>> "%FILE%" echo       context: ./frontend
>> "%FILE%" echo       dockerfile: Dockerfile
>> "%FILE%" echo       args:
>> "%FILE%" echo         VITE_API_BASE: "http://localhost:!BACKEND_PORT!"
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "!FRONTEND_PORT!:3170"
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       - backend
>> "%FILE%" echo.

REM -- Volumes --
>> "%FILE%" echo volumes:
for /l %%i in (0,1,%LAST_SHARD%) do (
    >> "%FILE%" echo   pgdata_shard%%i:
)
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    >> "%FILE%" echo   redisdata-%%r:
)
for /l %%k in (0,1,%LAST_KAFKA%) do (
    >> "%FILE%" echo   kafkadata-%%k:
)
>> "%FILE%" echo   seqdata:
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo   rustfsdata:
    >> "%FILE%" echo   rustfslogs:
)


REM -- Summary --
echo.
echo ========================================
echo  docker-compose.yml generated [PROD]
echo ========================================
echo    Shards:     %NUM_SHARDS% DB, %NUM_REDIS_SHARDS% Redis, %NUM_KAFKA_BROKERS% Kafka
echo    Replicas:   %NUM_REPLICAS%
echo    Public URL: !PUBLIC_URL!
echo    API:        !PUBLIC_URL!/api
echo    gRPC:       port !GRPC_PORT!
echo    CORS:       !PUBLIC_URL!
echo    Features:   ALL (Kafka, GraphQL, SSE, WebSocket, gRPC, API v2)
echo    Cargo:      !CARGO_FEATURES!
echo    PgBouncer:  ON  - connection pooling per shard
echo    Nginx LB:   ON  - load balancing %NUM_REPLICAS% replicas
echo    Seq:        http://localhost:!SEQ_PORT!
if /i "!USE_MINIO!"=="y" (
    echo    MinIO API:  http://localhost:!MINIO_PORT!
    echo    MinIO UI:   http://localhost:!MINIO_CONSOLE_PORT!
)

REM Container count: postgres(shards) + pgbouncer(shards) + redis(shards) + kafka(brokers) + seq + nginx-lb + backend-migrate + backend + frontend
set /a CONTAINER_COUNT=%NUM_SHARDS% * 2 + %NUM_REDIS_SHARDS% + %NUM_KAFKA_BROKERS% + 5
if /i "!USE_MINIO!"=="y" (
    set /a CONTAINER_COUNT=!CONTAINER_COUNT! + 1
)
echo.
echo    Total containers: !CONTAINER_COUNT!
echo ========================================
echo.


REM -- Clean and rebuild --
echo [1/3] Stopping all running containers...
docker compose down --remove-orphans 2>nul

echo [2/3] Removing project containers...
for /f "tokens=*" %%i in ('docker compose ps -aq') do docker rm -f %%i 2>nul

echo [3/3] Removing unused networks...
docker network prune -f 2>nul

echo.
set /p WIPE_DB="Wipe database volumes? This will DELETE all workflow/task data. (y/N): "
if /i "%WIPE_DB%"=="y" (
    echo Removing database, Redis, Kafka, and Seq volumes...
    for /l %%i in (0,1,%LAST_SHARD%) do (
        docker volume rm rust-conductor_pgdata_shard%%i 2>nul
    )
    set /a LAST_REDIS=%NUM_REDIS_SHARDS%-1
    for /l %%i in (0,1,!LAST_REDIS!) do (
        docker volume rm rust-conductor_redisdata-%%i 2>nul
    )
    for /l %%k in (0,1,%LAST_KAFKA%) do (
        docker volume rm rust-conductor_kafkadata-%%k 2>nul
    )
    docker volume rm rust-conductor_seqdata 2>nul
    if /i "!USE_MINIO!"=="y" (
        docker volume rm rust-conductor_rustfsdata 2>nul
        docker volume rm rust-conductor_rustfslogs 2>nul
    )
    echo Volumes removed.
) else (
    echo Keeping existing database data.
)

echo.
echo NOTE: Images and build cache are preserved to avoid Docker Hub rate limits.
echo       To force a full clean, run: docker system prune -af

echo.
echo ========================================
echo  Docker cleaned. Starting build...
echo ========================================
echo.

docker compose up --build --scale backend=%NUM_REPLICAS% -d

set /a LAST_PG_PORT=5432+%LAST_SHARD%
set /a LAST_PGB_PORT=6432+%LAST_SHARD%
set /a LAST_KAFKA_PORT=9092+%LAST_KAFKA%

echo.
echo ========================================
echo  Build complete! [PROD mode]
echo ========================================
echo    Public URL: !PUBLIC_URL!
echo    API:        !PUBLIC_URL!/api
echo    gRPC:       localhost:!GRPC_PORT!
echo    Replicas:   %NUM_REPLICAS%
echo    CORS:       !PUBLIC_URL!
echo    Shards:     %NUM_SHARDS%
echo    Postgres:   ports 5432-!LAST_PG_PORT!
echo    PgBouncer:  ports 6432-!LAST_PGB_PORT!
echo    Kafka:      ports 9092-!LAST_KAFKA_PORT! - %NUM_KAFKA_BROKERS% brokers
echo    Redis:      localhost:6379
echo    Seq:        http://localhost:!SEQ_PORT!
if /i "!USE_MINIO!"=="y" (
    echo    RustFS API: http://localhost:!MINIO_PORT!
    echo    RustFS UI:  http://localhost:!MINIO_CONSOLE_PORT!
)
echo ========================================
pause
