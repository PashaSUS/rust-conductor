@echo off
echo ========================================
echo  Docker Clean Rebuild - rust-conductor
echo ========================================
echo.

echo [1/6] Stopping all running containers...
docker compose down --remove-orphans 2>nul
docker stop (docker ps -aq) 2>nul

echo [2/6] Removing all containers...
for /f "tokens=*" %%i in ('docker ps -aq') do docker rm -f %%i 2>nul

echo [3/6] Removing all images...
for /f "tokens=*" %%i in ('docker images -q') do docker rmi -f %%i 2>nul

echo [4/6] Removing all volumes...
docker volume prune -f 2>nul

echo [5/6] Removing all networks...
docker network prune -f 2>nul

echo [6/6] Removing build cache...
docker builder prune -af 2>nul

echo.
echo ========================================
echo  Docker cleaned. Starting build...
echo ========================================
echo.

docker compose up --build -d

echo.
echo ========================================
echo  Build complete! Services:
echo    Backend:  http://localhost:8080
echo    Frontend: http://localhost:3000
echo    Postgres: localhost:5432
echo    Redis:    localhost:6379
echo ========================================
pause
