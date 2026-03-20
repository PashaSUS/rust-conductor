@echo off
echo ========================================
echo  rust-conductor Setup
echo ========================================
echo.
echo   1. DEV  - Lightweight: 1 shard, no PgBouncer/Nginx LB/Seq
echo   2. PROD - Full production: scaling, pooling, logging
echo.

:ask_env
set /p "ENV_MODE=Environment (1=DEV / 2=PROD): "
if "%ENV_MODE%"=="1" goto run_dev
if "%ENV_MODE%"=="2" goto run_prod
echo Invalid input. Please enter 1 or 2.
goto ask_env

:run_dev
call "%~dp0setup-dev.bat"
goto :eof

:run_prod
call "%~dp0setup-prod.bat"
goto :eof
