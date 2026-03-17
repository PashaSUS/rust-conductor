@echo off
setlocal enabledelayedexpansion
echo ========================================
echo  Docker Clean Rebuild - rust-conductor
echo ========================================
echo.

:ask_replicas
set /p NUM_REPLICAS="How many backend instances do you want? (1-8): "
set "valid="
for /l %%n in (1,1,8) do if "!NUM_REPLICAS!"=="%%n" set "valid=1"
if not defined valid (
    echo Invalid input. Please enter a number between 1 and 8.
    goto ask_replicas
)

echo.
echo [1/5] Stopping all running containers...
docker compose down --remove-orphans 2>nul

echo [2/5] Removing project containers...
for /f "tokens=*" %%i in ('docker compose ps -aq') do docker rm -f %%i 2>nul

echo [3/5] Removing unused networks...
docker network prune -f 2>nul

echo.
set /p WIPE_DB="Wipe database volumes? This will DELETE all workflow/task data. (y/N): "
if /i "%WIPE_DB%"=="y" (
    echo Removing database, Redis, and Kafka volumes...
    for /f "tokens=*" %%v in ('docker volume ls -q --filter "name=rust-conductor_pgdata"') do docker volume rm %%v 2>nul
    for /f "tokens=*" %%v in ('docker volume ls -q --filter "name=rust-conductor_redisdata"') do docker volume rm %%v 2>nul
    for /f "tokens=*" %%v in ('docker volume ls -q --filter "name=rust-conductor_kafkadata"') do docker volume rm %%v 2>nul
    docker volume rm rust-conductor_seqdata 2>nul
    echo Volumes removed.
) else (
    echo Keeping existing data.
)

echo [4/5] Scaling backend to %NUM_REPLICAS% instance(s)...
echo.
echo NOTE: Images are preserved to avoid Docker Hub rate limits.
echo       To force a full clean, run: docker system prune -af
echo.

docker compose up --build --scale backend=%NUM_REPLICAS% -d

echo.
echo ========================================
echo  Build complete! Services:
echo    Backend:  http://localhost:8080 (%NUM_REPLICAS% instance(s) via nginx-lb)
echo    gRPC:     localhost:50051
echo    Frontend: http://localhost:3000
echo    Kafka:    localhost:9092
echo    Redis:    localhost:6379
echo    Seq:      http://localhost:9321
echo ========================================
pause
pause
