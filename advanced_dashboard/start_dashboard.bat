@echo off
echo ZKP-FL Advanced Dashboard Startup Script
echo ========================================

REM Check if Python is installed
python --version >nul 2>&1
if errorlevel 1 (
    echo Error: Python is not installed or not in PATH
    echo Please install Python 3.8 or higher
    pause
    exit /b 1
)

REM Check if we're in the right directory
if not exist "server.py" (
    echo Error: server.py not found
    echo Please run this script from the advanced_dashboard directory
    pause
    exit /b 1
)

REM Install dependencies if needed
echo Installing Python dependencies...
python -m pip install -r requirements.txt

if errorlevel 1 (
    echo Warning: Failed to install some dependencies
    echo Continuing anyway...
)

REM Start the server
echo.
echo Starting ZKP-FL Dashboard Server...
echo The dashboard will be available at: http://localhost:8000
echo Press Ctrl+C to stop the server
echo.

python server.py --host 0.0.0.0 --port 8000

pause
