@echo off
setlocal enabledelayedexpansion

echo ========================================
echo  rust-conductor Shard Setup ^& Build
echo ========================================
echo.

:ask_shards
set /p NUM_SHARDS="How many database shards do you want? (1-16): "

:: Validate input is a number between 1 and 16
set "valid="
for /l %%n in (1,1,16) do if "!NUM_SHARDS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 16.
    goto ask_shards
)


:ask_replicas
set /p NUM_REPLICAS="How many backend replicas do you want? (1-8): "

:: Validate input is a number between 1 and 8
set "valid="
for /l %%n in (1,1,8) do if "!NUM_REPLICAS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 8.
    goto ask_replicas
)

:ask_redis_shards
set /p NUM_REDIS_SHARDS="How many Redis shards do you want? (1-8): "

:: Validate input is a number between 1 and 8
set "valid="
for /l %%n in (1,1,8) do if "!NUM_REDIS_SHARDS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 8.
    goto ask_redis_shards
)

echo.
echo Configuring %NUM_SHARDS% shard(s)...
echo.

:: ── Generate docker-compose.yml ──
set "FILE=docker-compose.yml"
> "%FILE%" echo services:

:: ── Postgres shards ──
set /a LAST_SHARD=%NUM_SHARDS%-1
for /l %%i in (0,1,%LAST_SHARD%) do (
    set /a PG_PORT=5432+%%i
    >> "%FILE%" echo   # ── Postgres Shard %%i ──
    >> "%FILE%" echo   postgres-shard-%%i:
    >> "%FILE%" echo     image: postgres:16-alpine
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

:: ── PgBouncer instances ──
for /l %%i in (0,1,%LAST_SHARD%) do (
    set /a PGB_PORT=6432+%%i
    >> "%FILE%" echo   # ── PgBouncer ^(connection pooler in front of shard %%i^) ──
    >> "%FILE%" echo   pgbouncer-shard-%%i:
    >> "%FILE%" echo     image: edoburu/pgbouncer:latest
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


:: ── Redis Shards ──
set /a LAST_REDIS_SHARD=%NUM_REDIS_SHARDS%-1
set "REDIS_URLS="
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    set /a REDIS_PORT=6379+%%r
    >> "%FILE%" echo   redis-%%r:
    >> "%FILE%" echo     image: redis:7-alpine
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


:: ── Seq (structured log server) ──
>> "%FILE%" echo   # ── Seq ^(structured log server^) ──
>> "%FILE%" echo   seq:
>> "%FILE%" echo     image: datalust/seq:latest
>> "%FILE%" echo     environment:
>> "%FILE%" echo       ACCEPT_EULA: "Y"
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "9321:80"
>> "%FILE%" echo     volumes:
>> "%FILE%" echo       - seqdata:/data
>> "%FILE%" echo     healthcheck:
>> "%FILE%" echo       test: ["CMD", "curl", "-f", "http://localhost:80/api"]
>> "%FILE%" echo       interval: 10s
>> "%FILE%" echo       timeout: 5s
>> "%FILE%" echo       retries: 5
>> "%FILE%" echo.


:: ── Backend Migrator (runs migrations once, then exits) ──
:: Connects DIRECTLY to Postgres (bypassing PgBouncer) to avoid
:: transaction-pooling interference with DDL statements.
>> "%FILE%" echo   backend-migrate:
>> "%FILE%" echo     build:
>> "%FILE%" echo       context: ./backend
>> "%FILE%" echo       dockerfile: Dockerfile
>> "%FILE%" echo     environment:
>> "%FILE%" echo       HOST: "0.0.0.0"
>> "%FILE%" echo       PORT: "8080"
>> "%FILE%" echo       NUM_SHARDS: "%NUM_SHARDS%"
>> "%FILE%" echo       SHARD_DB_URL_TEMPLATE: "postgres://conductor:conductor@postgres-shard-{i}:5432/conductor"
>> "%FILE%" echo       REDIS_URLS: "!REDIS_URLS!"
>> "%FILE%" echo       RUST_LOG: "info"
>> "%FILE%" echo       MIGRATE_ONLY: "true"
>> "%FILE%" echo       SEQ_URL: "http://seq:80"
>> "%FILE%" echo     depends_on:
for /l %%i in (0,1,%LAST_SHARD%) do (
    >> "%FILE%" echo       postgres-shard-%%i:
    >> "%FILE%" echo         condition: service_healthy
)
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    >> "%FILE%" echo       redis-%%r:
    >> "%FILE%" echo         condition: service_healthy
)
>> "%FILE%" echo       seq:
>> "%FILE%" echo         condition: service_healthy
>> "%FILE%" echo.


:: ── Backend (build depends_on migrator + all pgbouncers + redis) ──
>> "%FILE%" echo   backend:
>> "%FILE%" echo     build:
>> "%FILE%" echo       context: ./backend
>> "%FILE%" echo       dockerfile: Dockerfile
>> "%FILE%" echo     expose:
>> "%FILE%" echo       - "8080"
>> "%FILE%" echo     environment:
>> "%FILE%" echo       HOST: "0.0.0.0"
>> "%FILE%" echo       PORT: "8080"
>> "%FILE%" echo       NUM_SHARDS: "%NUM_SHARDS%"
>> "%FILE%" echo       SHARD_DB_URL_TEMPLATE: "postgres://conductor:conductor@pgbouncer-shard-{i}:6432/conductor"
>> "%FILE%" echo       REDIS_URLS: "!REDIS_URLS!"
>> "%FILE%" echo       RUST_LOG: "warn"
>> "%FILE%" echo       SKIP_MIGRATIONS: "true"
>> "%FILE%" echo       SEQ_URL: "http://seq:80"
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
>> "%FILE%" echo       seq:
>> "%FILE%" echo         condition: service_healthy
>> "%FILE%" echo.

:: ── Nginx Load Balancer (in front of backend replicas) ──
>> "%FILE%" echo   nginx-lb:
>> "%FILE%" echo     image: nginx:alpine
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "8080:8080"
>> "%FILE%" echo     volumes:
>> "%FILE%" echo       - ./nginx-lb.conf:/etc/nginx/conf.d/default.conf:ro
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       - backend
>> "%FILE%" echo.

:: ── Frontend ──
>> "%FILE%" echo   frontend:
>> "%FILE%" echo     build:
>> "%FILE%" echo       context: ./frontend
>> "%FILE%" echo       dockerfile: Dockerfile
>> "%FILE%" echo       args:
>> "%FILE%" echo         VITE_API_BASE: "http://localhost:8080"
>> "%FILE%" echo     ports:
>> "%FILE%" echo       - "3000:3000"
>> "%FILE%" echo     depends_on:
>> "%FILE%" echo       - backend
>> "%FILE%" echo.

:: ── Volumes ──
>> "%FILE%" echo volumes:
for /l %%i in (0,1,%LAST_SHARD%) do (
    >> "%FILE%" echo   pgdata_shard%%i:
)
for /l %%r in (0,1,%LAST_REDIS_SHARD%) do (
    >> "%FILE%" echo   redisdata-%%r:
)
>> "%FILE%" echo   seqdata:

echo.
echo ========================================
echo  docker-compose.yml generated with %NUM_SHARDS% shard(s)
echo ========================================
echo.

:: ── Clean and rebuild ──
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
    for /l %%i in (0,1,%LAST_SHARD%) do (
        docker volume rm rust-conductor_pgdata_shard%%i 2>nul
    )
    set /a LAST_REDIS=%NUM_REDIS_SHARDS%-1
    for /l %%i in (0,1,!LAST_REDIS!) do (
        docker volume rm rust-conductor_redisdata-%%i 2>nul
    )
    docker volume rm rust-conductor_seqdata 2>nul
    echo Database volumes removed.
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

docker compose up --build -d

set /a LAST_PG_PORT=5432+%LAST_SHARD%
set /a LAST_PGB_PORT=6432+%LAST_SHARD%

echo.
echo ========================================
echo  Build complete! Services:
echo    Backend:    http://localhost:8080 (via nginx-lb)
echo    Replicas:   %NUM_REPLICAS%
echo    Frontend:   http://localhost:3000
echo    Shards:     %NUM_SHARDS%
echo    Postgres:   ports 5432-!LAST_PG_PORT!
echo    PgBouncer:  ports 6432-!LAST_PGB_PORT!
echo    Redis:      localhost:6379
echo    Seq:        http://localhost:9321
echo ========================================
pause
