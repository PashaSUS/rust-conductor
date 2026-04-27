@echo off
setlocal enabledelayedexpansion

echo ========================================
echo  rust-conductor Setup - DEV Mode
echo ========================================
echo.
echo Lightweight setup for development machines.
echo Single shard, no PgBouncer, no Nginx LB.
echo Saves ~1-2GB RAM while keeping full functionality.
echo.

echo   B. BUILD - Build images locally from source (for development)
echo   P. PULL  - Pull pre-built images from GHCR (faster startup)
echo.
:ask_build_mode
set /p "BUILD_MODE=Build or Pull? (B/P): "
if /i "!BUILD_MODE!"=="b" set "BUILD_MODE=build" & goto build_mode_set
if /i "!BUILD_MODE!"=="p" set "BUILD_MODE=pull" & goto build_mode_set
echo Invalid input. Please enter B or P.
goto ask_build_mode
:build_mode_set
echo.

REM -- Fixed defaults --
set "NUM_SHARDS=1"
set "NUM_REPLICAS=1"
set "NUM_REDIS_SHARDS=1"
set "BACKEND_PORT=8090"
set "GRPC_PORT=50055"
set "FRONTEND_PORT=3170"

echo ========================================
echo  DEV Defaults
echo ========================================
echo  Shards: 1, Replicas: 1, Redis: 1
echo  Kafka: OFF (uses Redis Streams instead)
echo  PgBouncer: OFF, Nginx LB: OFF
echo ========================================
echo.

echo You can override ports - press Enter to keep defaults:
set /p "BACKEND_PORT=Backend API port [!BACKEND_PORT!]: " || set "BACKEND_PORT=!BACKEND_PORT!"
set /p "GRPC_PORT=gRPC port [!GRPC_PORT!]: " || set "GRPC_PORT=!GRPC_PORT!"
set /p "FRONTEND_PORT=Frontend port [!FRONTEND_PORT!]: " || set "FRONTEND_PORT=!FRONTEND_PORT!"


REM ================================================================
REM  Interactive Feature Selection
REM ================================================================
echo.
echo ========================================
echo  Optional Features
echo ========================================
echo  Choose which features to enable.
echo  Press Enter to accept the default (shown in brackets).
echo ========================================
echo.

REM -- Seq (structured logging) --
set "USE_SEQ=N"
set /p USE_SEQ="Enable Seq structured logging? (y/N): "

REM -- Kafka --
set "USE_KAFKA=N"
set /p USE_KAFKA="Enable Kafka event streaming? (y/N): "

REM -- GraphQL --
set "USE_GRAPHQL=N"
set /p USE_GRAPHQL="Enable GraphQL API? (y/N): "

REM -- SSE --
set "USE_SSE=N"
set /p USE_SSE="Enable Server-Sent Events (SSE)? (y/N): "

REM -- WebSocket --
set "USE_WS=N"
set /p USE_WS="Enable WebSocket real-time feeds? (y/N): "

REM -- gRPC Reflection --
set "USE_GRPC_REFLECT=N"
set /p USE_GRPC_REFLECT="Enable gRPC reflection? (y/N): "

REM -- API v2 --
set "USE_V2=N"
set /p USE_V2="Enable API v2 endpoints? (y/N): "

REM -- Prometheus + Grafana monitoring --
set "USE_MONITORING=N"
set /p USE_MONITORING="Enable Prometheus + Grafana monitoring? (y/N): "
if /i "!USE_MONITORING!"=="y" (
    set "PROMETHEUS_PORT=9090"
    set "GRAFANA_PORT=3000"
    set /p "PROMETHEUS_PORT=Prometheus port [!PROMETHEUS_PORT!]: " || set "PROMETHEUS_PORT=!PROMETHEUS_PORT!"
    set /p "GRAFANA_PORT=Grafana port [!GRAFANA_PORT!]: " || set "GRAFANA_PORT=!GRAFANA_PORT!"
    echo.
    echo Monitoring will be available at:
    echo   Prometheus: http://localhost:!PROMETHEUS_PORT!
    echo   Grafana:    http://localhost:!GRAFANA_PORT!  (admin/admin)
    echo.
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

REM -- Build cargo features string --
set "CARGO_FEATURES="
if /i "!USE_SEQ!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!seq,"
if /i "!USE_KAFKA!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!kafka,"
if /i "!USE_GRAPHQL!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!graphql,"
if /i "!USE_SSE!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!sse,"
if /i "!USE_WS!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!websocket,"
if /i "!USE_GRPC_REFLECT!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!grpc-reflection,"
if /i "!USE_V2!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!api-v2,"
if /i "!USE_MINIO!"=="y" set "CARGO_FEATURES=!CARGO_FEATURES!external-storage,"

REM Remove trailing comma
if "!CARGO_FEATURES!"=="" (
    set "CARGO_FEATURES=bare-metal"
) else (
    set "CARGO_FEATURES=!CARGO_FEATURES:~0,-1!"
)


REM -- URL configuration --
echo.
echo ========================================
echo  URL Configuration
echo ========================================
echo.
echo The frontend serves the UI and proxies /api requests to the backend.
echo Both share a single public URL.
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


REM ================================================================
REM  Performance Tuning (DEV defaults are conservative)
REM ================================================================
echo.
echo ========================================
echo  Performance Tuning
echo ========================================
echo.

set "RUST_LOG_LEVEL=info"
set "RATE_LIMIT_ENABLED=true"
set "RATE_LIMIT_MAX_REQUESTS=1000"
set "RATE_LIMIT_WINDOW_SECS=60"
set "SWEEPER_MIN_INTERVAL_SECS=10"
set "SWEEPER_MAX_INTERVAL_SECS=60"
set "PG_MAX_CONNECTIONS=50"
set "PG_SHARED_BUFFERS=64MB"
set "PG_WORK_MEM=4MB"
set "PG_EFFECTIVE_CACHE=128MB"
set "PG_MAX_LOCKS=64"

set "TUNE_CHOICE=N"
set /p TUNE_CHOICE="Customize tuning values? (y/N): "
if /i "!TUNE_CHOICE!"=="y" (
    set /p "RUST_LOG_LEVEL=Backend log level (error/warn/info/debug) [!RUST_LOG_LEVEL!]: " || set "RUST_LOG_LEVEL=!RUST_LOG_LEVEL!"
    set /p "RATE_LIMIT_ENABLED=Enable rate limiter? (true/false) [!RATE_LIMIT_ENABLED!]: " || set "RATE_LIMIT_ENABLED=!RATE_LIMIT_ENABLED!"
    if /i "!RATE_LIMIT_ENABLED!"=="true" (
        set /p "RATE_LIMIT_MAX_REQUESTS=Max requests per window [!RATE_LIMIT_MAX_REQUESTS!]: " || set "RATE_LIMIT_MAX_REQUESTS=!RATE_LIMIT_MAX_REQUESTS!"
        set /p "RATE_LIMIT_WINDOW_SECS=Window length in seconds [!RATE_LIMIT_WINDOW_SECS!]: " || set "RATE_LIMIT_WINDOW_SECS=!RATE_LIMIT_WINDOW_SECS!"
    )
    set /p "SWEEPER_MIN_INTERVAL_SECS=Sweeper min interval (s) [!SWEEPER_MIN_INTERVAL_SECS!]: " || set "SWEEPER_MIN_INTERVAL_SECS=!SWEEPER_MIN_INTERVAL_SECS!"
    set /p "SWEEPER_MAX_INTERVAL_SECS=Sweeper max interval (s) [!SWEEPER_MAX_INTERVAL_SECS!]: " || set "SWEEPER_MAX_INTERVAL_SECS=!SWEEPER_MAX_INTERVAL_SECS!"
    set /p "PG_MAX_CONNECTIONS=Postgres max_connections [!PG_MAX_CONNECTIONS!]: " || set "PG_MAX_CONNECTIONS=!PG_MAX_CONNECTIONS!"
    set /p "PG_SHARED_BUFFERS=Postgres shared_buffers [!PG_SHARED_BUFFERS!]: " || set "PG_SHARED_BUFFERS=!PG_SHARED_BUFFERS!"
    set /p "PG_WORK_MEM=Postgres work_mem [!PG_WORK_MEM!]: " || set "PG_WORK_MEM=!PG_WORK_MEM!"
    set /p "PG_EFFECTIVE_CACHE=Postgres effective_cache_size [!PG_EFFECTIVE_CACHE!]: " || set "PG_EFFECTIVE_CACHE=!PG_EFFECTIVE_CACHE!"
)

echo.
echo Performance settings:
echo   Log level:  !RUST_LOG_LEVEL!
echo   Rate limit: !RATE_LIMIT_ENABLED! (max=!RATE_LIMIT_MAX_REQUESTS!/!RATE_LIMIT_WINDOW_SECS!s)
echo   Sweeper:    !SWEEPER_MIN_INTERVAL_SECS!-!SWEEPER_MAX_INTERVAL_SECS!s
echo   Postgres:   max_conn=!PG_MAX_CONNECTIONS! shared=!PG_SHARED_BUFFERS!
echo.


REM -- Generate nginx-lb.conf (for consistency, even though DEV doesn't use it) --
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
echo Configuring DEV mode - 1 shard...
echo.


REM ================================================================
REM  Generate docker-compose.yml
REM ================================================================
set "FILE=docker-compose.yml"
> "%FILE%" echo services:

REM -- Postgres (single shard, low memory) --
>> "%FILE%" echo   # Postgres Shard 0 - DEV low memory
>> "%FILE%" echo   postgres-shard-0:
>> "%FILE%" echo     image: postgres:17-alpine
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     command: ^>
>> "%FILE%" echo       postgres
>> "%FILE%" echo       -c max_locks_per_transaction=!PG_MAX_LOCKS!
>> "%FILE%" echo       -c max_connections=!PG_MAX_CONNECTIONS!
>> "%FILE%" echo       -c shared_buffers=!PG_SHARED_BUFFERS!
>> "%FILE%" echo       -c work_mem=!PG_WORK_MEM!
>> "%FILE%" echo       -c effective_cache_size=!PG_EFFECTIVE_CACHE!
>> "%FILE%" echo     environment:
>> "%FILE%" echo       POSTGRES_USER: conductor
>> "%FILE%" echo       POSTGRES_PASSWORD: conductor
>> "%FILE%" echo       POSTGRES_DB: conductor
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "5432:5432"
>> "%FILE%" echo     volumes:
>> "%FILE%" echo       - pgdata_shard0:/var/lib/postgresql/data
>> "%FILE%" echo     healthcheck:
>> "%FILE%" echo       test: ["CMD-SHELL", "pg_isready -U conductor"]
>> "%FILE%" echo       interval: 5s
>> "%FILE%" echo       timeout: 5s
>> "%FILE%" echo       retries: 5
>> "%FILE%" echo.

REM -- Redis (single) --
>> "%FILE%" echo   redis-0:
>> "%FILE%" echo     image: redis:8-alpine
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "6379:6379"
>> "%FILE%" echo     volumes:
>> "%FILE%" echo       - redisdata-0:/data
>> "%FILE%" echo     healthcheck:
>> "%FILE%" echo       test: ["CMD", "redis-cli", "ping"]
>> "%FILE%" echo       interval: 5s
>> "%FILE%" echo       timeout: 5s
>> "%FILE%" echo       retries: 5
>> "%FILE%" echo.

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

REM -- Backend Migrator --
>> "%FILE%" echo   backend-migrate:
if /i "!BUILD_MODE!"=="build" (
    >> "%FILE%" echo     build:
    >> "%FILE%" echo       context: ./backend
    >> "%FILE%" echo       dockerfile: Dockerfile
    >> "%FILE%" echo       args:
    >> "%FILE%" echo         CARGO_FEATURES: "!CARGO_FEATURES!"
) else (
    >> "%FILE%" echo     image: ghcr.io/pashasus/rust-conductor/backend:latest
)
>> "%FILE%" echo     environment:
>> "%FILE%" echo       HOST: "0.0.0.0"
>> "%FILE%" echo       PORT: "!BACKEND_PORT!"
>> "%FILE%" echo       NUM_SHARDS: "1"
>> "%FILE%" echo       SHARD_DB_URL_TEMPLATE: "postgres://conductor:conductor@postgres-shard-{i}:5432/conductor"
>> "%FILE%" echo       REDIS_URLS: "redis://redis-0:6379"
>> "%FILE%" echo       RUST_LOG: "info"
>> "%FILE%" echo       MIGRATE_ONLY: "true"
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       S3_ENDPOINT: "http://rustfs:9000"
    >> "%FILE%" echo       S3_BUCKET: "conductor-payloads"
    >> "%FILE%" echo       S3_REGION: "us-east-1"
    >> "%FILE%" echo       S3_ACCESS_KEY: "rustfsadmin"
    >> "%FILE%" echo       S3_SECRET_KEY: "rustfsadmin"
)
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       postgres-shard-0:
>> "%FILE%" echo         condition: service_healthy
>> "%FILE%" echo       redis-0:
>> "%FILE%" echo         condition: service_healthy
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       rustfs:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo.

REM -- Backend --
>> "%FILE%" echo   backend:
if /i "!BUILD_MODE!"=="build" (
    >> "%FILE%" echo     build:
    >> "%FILE%" echo       context: ./backend
    >> "%FILE%" echo       dockerfile: Dockerfile
    >> "%FILE%" echo       args:
    >> "%FILE%" echo         CARGO_FEATURES: "!CARGO_FEATURES!"
) else (
    >> "%FILE%" echo     image: ghcr.io/pashasus/rust-conductor/backend:latest
)
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "!BACKEND_PORT!:!BACKEND_PORT!"
>> "%FILE%" echo       - "!GRPC_PORT!:!GRPC_PORT!"
>> "%FILE%" echo     environment:
>> "%FILE%" echo       HOST: "0.0.0.0"
>> "%FILE%" echo       PORT: "!BACKEND_PORT!"
>> "%FILE%" echo       GRPC_PORT: "!GRPC_PORT!"
>> "%FILE%" echo       NUM_SHARDS: "1"
>> "%FILE%" echo       SHARD_DB_URL_TEMPLATE: "postgres://conductor:conductor@postgres-shard-{i}:5432/conductor"
>> "%FILE%" echo       REDIS_URLS: "redis://redis-0:6379"
>> "%FILE%" echo       CORS_ORIGIN: "!PUBLIC_URL!"
>> "%FILE%" echo       RUST_LOG: "!RUST_LOG_LEVEL!"
>> "%FILE%" echo       SKIP_MIGRATIONS: "true"
>> "%FILE%" echo       RATE_LIMIT_ENABLED: "!RATE_LIMIT_ENABLED!"
>> "%FILE%" echo       RATE_LIMIT_MAX_REQUESTS: "!RATE_LIMIT_MAX_REQUESTS!"
>> "%FILE%" echo       RATE_LIMIT_WINDOW_SECS: "!RATE_LIMIT_WINDOW_SECS!"
>> "%FILE%" echo       SWEEPER_MIN_INTERVAL_SECS: "!SWEEPER_MIN_INTERVAL_SECS!"
>> "%FILE%" echo       SWEEPER_MAX_INTERVAL_SECS: "!SWEEPER_MAX_INTERVAL_SECS!"
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       S3_ENDPOINT: "http://rustfs:9000"
    >> "%FILE%" echo       S3_BUCKET: "conductor-payloads"
    >> "%FILE%" echo       S3_REGION: "us-east-1"
    >> "%FILE%" echo       S3_ACCESS_KEY: "rustfsadmin"
    >> "%FILE%" echo       S3_SECRET_KEY: "rustfsadmin"
)
>> "%FILE%" echo     deploy:
>> "%FILE%" echo       replicas: 1
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       backend-migrate:
>> "%FILE%" echo         condition: service_completed_successfully
>> "%FILE%" echo       postgres-shard-0:
>> "%FILE%" echo         condition: service_healthy
>> "%FILE%" echo       redis-0:
>> "%FILE%" echo         condition: service_healthy
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo       rustfs:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo.

REM -- Frontend --
>> "%FILE%" echo   frontend:
if /i "!BUILD_MODE!"=="build" (
    >> "%FILE%" echo     build:
    >> "%FILE%" echo       context: ./frontend
    >> "%FILE%" echo       dockerfile: Dockerfile
    >> "%FILE%" echo       args:
    >> "%FILE%" echo         VITE_API_BASE: "http://localhost:!BACKEND_PORT!"
) else (
    >> "%FILE%" echo     image: ghcr.io/pashasus/rust-conductor/frontend:latest
)
>> "%FILE%" echo     restart: unless-stopped
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "!FRONTEND_PORT!:3170"
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       - backend
>> "%FILE%" echo.

REM -- Prometheus (optional) --
if /i "!USE_MONITORING!"=="y" (
    >> "%FILE%" echo   # Prometheus - metrics collection
    >> "%FILE%" echo   prometheus:
    >> "%FILE%" echo     image: prom/prometheus:latest
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!PROMETHEUS_PORT!:9090"
    >> "%FILE%" echo     volumes:
    >> "%FILE%" echo       - ./monitoring/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    >> "%FILE%" echo       - prometheusdata:/prometheus
    >> "%FILE%" echo     depends_on:
    >> "%FILE%" echo       - backend
    >> "%FILE%" echo.
)

REM -- Grafana (optional) --
if /i "!USE_MONITORING!"=="y" (
    >> "%FILE%" echo   # Grafana - metrics dashboard
    >> "%FILE%" echo   grafana:
    >> "%FILE%" echo     image: grafana/grafana:latest
    >> "%FILE%" echo     restart: unless-stopped
    >> "%FILE%" echo     ports:
    >> "%FILE%" echo       - "!GRAFANA_PORT!:3000"
    >> "%FILE%" echo     environment:
    >> "%FILE%" echo       GF_SECURITY_ADMIN_USER: admin
    >> "%FILE%" echo       GF_SECURITY_ADMIN_PASSWORD: admin
    >> "%FILE%" echo     volumes:
    >> "%FILE%" echo       - grafanadata:/var/lib/grafana
    >> "%FILE%" echo       - ./monitoring/grafana/provisioning:/etc/grafana/provisioning:ro
    >> "%FILE%" echo       - ./monitoring/grafana/dashboards:/var/lib/grafana/dashboards:ro
    >> "%FILE%" echo     depends_on:
    >> "%FILE%" echo       - prometheus
    >> "%FILE%" echo.
)

REM -- Volumes --
>> "%FILE%" echo volumes:
>> "%FILE%" echo   pgdata_shard0:
>> "%FILE%" echo   redisdata-0:
if /i "!USE_MINIO!"=="y" (
    >> "%FILE%" echo   rustfsdata:
    >> "%FILE%" echo   rustfslogs:
)
if /i "!USE_MONITORING!"=="y" (
    >> "%FILE%" echo   prometheusdata:
    >> "%FILE%" echo   grafanadata:
)


REM -- Summary --
echo.
echo ========================================
echo  docker-compose.yml generated [DEV]
echo ========================================
echo    Shards:     1 DB, 1 Redis
echo    Replicas:   1
echo    Public URL: !PUBLIC_URL!
echo    API:        !PUBLIC_URL!/api
echo    gRPC:       port !GRPC_PORT!
echo    CORS:       !PUBLIC_URL!
echo    PgBouncer:  OFF
echo    Nginx LB:   OFF
echo    Features:   !CARGO_FEATURES!
if /i "!USE_MONITORING!"=="y" (
    echo    Prometheus: http://localhost:!PROMETHEUS_PORT!
    echo    Grafana:    http://localhost:!GRAFANA_PORT!  (admin/admin)
)
if /i "!USE_MINIO!"=="y" (
    echo    RustFS API: http://localhost:!MINIO_PORT!
    echo    RustFS UI:  http://localhost:!MINIO_CONSOLE_PORT!
)
REM Base: postgres + redis + backend-migrate + backend + frontend = 5
set /a CONTAINER_COUNT=5
if /i "!USE_MINIO!"=="y" (
    set /a CONTAINER_COUNT=!CONTAINER_COUNT! + 1
)
if /i "!USE_MONITORING!"=="y" (
    set /a CONTAINER_COUNT=!CONTAINER_COUNT! + 2
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
    echo Removing database and Redis volumes...
    docker volume rm rust-conductor_pgdata_shard0 2>nul
    docker volume rm rust-conductor_redisdata-0 2>nul
    if /i "!USE_MINIO!"=="y" (
        docker volume rm rust-conductor_rustfsdata 2>nul
        docker volume rm rust-conductor_rustfslogs 2>nul
    )
    if /i "!USE_MONITORING!"=="y" (
        docker volume rm rust-conductor_prometheusdata 2>nul
        docker volume rm rust-conductor_grafanadata 2>nul
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
echo  Docker cleaned. Starting...
echo ========================================
echo.

if /i "!BUILD_MODE!"=="build" (
    docker compose up --build -d
) else (
    docker compose up -d
)

echo.
echo ========================================
echo  Setup complete! [DEV mode]
echo ========================================
echo    Public URL: !PUBLIC_URL!
echo    API:        !PUBLIC_URL!/api
echo    gRPC:       localhost:!GRPC_PORT!
echo    Postgres:   localhost:5432
echo    Redis:      localhost:6379
if /i "!USE_MONITORING!"=="y" (
    echo    Prometheus: http://localhost:!PROMETHEUS_PORT!
    echo    Grafana:    http://localhost:!GRAFANA_PORT!  (admin/admin)
)
if /i "!USE_MINIO!"=="y" (
    echo    RustFS API: http://localhost:!MINIO_PORT!
    echo    RustFS UI:  http://localhost:!MINIO_CONSOLE_PORT!
)
echo ========================================
pause
