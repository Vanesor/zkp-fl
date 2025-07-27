
@echo off
REM Enhanced benchmark script with parameter support
REM Usage: benchmark.bat [num_clients] [num_rounds] [scenario] [verbose]

echo Starting enhanced benchmark execution...
cd /d "%~dp0.."
echo Current directory: %CD%

REM Set default values
set NUM_CLIENTS=1
set NUM_ROUNDS=1
set SCENARIO=single-client
set VERBOSE_FLAG=--verbose
set CLIENT_DELAY=0
set MAX_CONCURRENT=1

REM Parse command line arguments
if not "%1"=="" set NUM_CLIENTS=%1
if not "%2"=="" set NUM_ROUNDS=%2
if not "%3"=="" set SCENARIO=%3
if "%4"=="quiet" set VERBOSE_FLAG=

REM Set max concurrent based on number of clients (but cap at 10 for performance)
if %NUM_CLIENTS% GTR 10 (
    set MAX_CONCURRENT=10
) else (
    set MAX_CONCURRENT=%NUM_CLIENTS%
)

echo ========================================
echo Benchmark Configuration:
echo   Clients: %NUM_CLIENTS%
echo   Rounds: %NUM_ROUNDS%
echo   Scenario: %SCENARIO%
echo   Max Concurrent: %MAX_CONCURRENT%
echo   Client Delay: %CLIENT_DELAY%ms
echo ========================================
echo.

REM Build the cargo command
set CARGO_CMD=cargo run --bin benchmarks -- --scenario %SCENARIO% --num-clients %NUM_CLIENTS% --rounds %NUM_ROUNDS% --client-delay-ms %CLIENT_DELAY% --max-concurrent %MAX_CONCURRENT% %VERBOSE_FLAG%

echo Executing: %CARGO_CMD%
echo.

REM Execute the benchmark
%CARGO_CMD%

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ========================================
    echo ERROR: Benchmark execution failed with error code %ERRORLEVEL%
    echo ========================================
    exit /b %ERRORLEVEL%
) else (
    echo.
    echo ========================================
    echo SUCCESS: Benchmark execution completed successfully
    echo ========================================
)

exit /b 0