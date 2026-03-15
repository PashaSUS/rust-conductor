@echo off
echo ========================================
echo  Docker Clean Rebuild - rust-conductor
echo ========================================
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
    echo Removing database and Redis volumes...
    docker volume rm rust-conductor_pgdata_shard0 2>nul
    docker volume rm rust-conductor_pgdata_shard1 2>nul
    docker volume rm rust-conductor_pgdata_shard2 2>nul
    docker volume rm rust-conductor_pgdata_shard3 2>nul
    docker volume rm rust-conductor_redisdata-0 2>nul
    docker volume rm rust-conductor_redisdata-1 2>nul
    docker volume rm rust-conductor_redisdata-2 2>nul
    docker volume rm rust-conductor_redisdata-3 2>nul
    docker volume rm rust-conductor_seqdata 2>nul
    echo Database volumes removed.
) else (
    echo Keeping existing database data.
)

echo [4/5] Rebuilding...
echo.
echo NOTE: Images are preserved to avoid Docker Hub rate limits.
echo       To force a full clean, run: docker system prune -af
echo.

docker compose up --build -d

echo.
echo ========================================
echo  Build complete! Services:
echo    Backend:  http://localhost:8080
echo    Frontend: http://localhost:3000
echo    Postgres: localhost:5432
echo    Redis:    localhost:6379
echo    Seq:      http://localhost:9321
echo ========================================
pause
