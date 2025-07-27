@echo off
echo Starting benchmark execution...
cd /d "%~dp0.."
echo Current directory: %CD%
cargo run --bin benchmarks -- --scenario single-client --num-clients 1 --rounds 1 --client-delay-ms 0 --max-concurrent 1 --verbose
if %ERRORLEVEL% NEQ 0 (
    echo Benchmark execution failed with error code %ERRORLEVEL%
    exit /b %ERRORLEVEL%
)
echo Benchmark execution completed successfully
exit /b 0
