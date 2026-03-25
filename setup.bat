@echo off
echo ========================================
echo  rust-conductor Setup
echo ========================================
echo.
echo   1. DEV  - Lightweight development (interactive feature selection)
echo   2. PROD - Full production (all features enabled)
echo.

:ask_env
set /p "ENV_MODE=Choose build profile (1-2): "
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
