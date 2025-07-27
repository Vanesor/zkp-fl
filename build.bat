@echo off
REM ZKP Federated Learning - Windows Build Script

setlocal enabledelayedexpansion

echo ===================================
echo ZKP Federated Learning Build Script
echo ===================================

REM Configuration
set PROJECT_NAME=zkp-fl
set BUILD_TYPE=release
set TARGET_DIR=target
set DIST_DIR=dist
set LOG_FILE=build.log

REM Colors for output (Windows)
set INFO=[INFO]
set SUCCESS=[SUCCESS]
set WARNING=[WARNING]
set ERROR=[ERROR]

REM Function to check prerequisites
:check_prerequisites
echo %INFO% Checking prerequisites...

REM Check Rust installation
cargo --version >nul 2>&1
if %errorlevel% neq 0 (
    echo %ERROR% Cargo is not installed. Please install Rust: https://rustup.rs/
    exit /b 1
)

REM Check Python installation
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo %WARNING% Python not found. Visualization scripts will not work.
)

REM Check Rust version
for /f "tokens=2" %%i in ('rustc --version') do set RUST_VERSION=%%i
echo %INFO% Found Rust version: %RUST_VERSION%

echo %SUCCESS% Prerequisites check completed
goto :eof

REM Function to clean previous builds
:clean_build
echo %INFO% Cleaning previous builds...

if exist "%TARGET_DIR%" (
    rmdir /s /q "%TARGET_DIR%"
    echo %INFO% Removed target directory
)

if exist "%DIST_DIR%" (
    rmdir /s /q "%DIST_DIR%"
    echo %INFO% Removed dist directory
)

REM Clean cargo cache
cargo clean >> "%LOG_FILE%" 2>&1

echo %SUCCESS% Clean completed
goto :eof

REM Function to build all crates
:build_project
echo %INFO% Building project in %BUILD_TYPE% mode...

REM Create log file
echo Build started at %date% %time% > "%LOG_FILE%"

REM Build flags
set BUILD_FLAGS=
if "%BUILD_TYPE%"=="release" (
    set BUILD_FLAGS=--release
)

REM Build all crates
echo %INFO% Building common library...
cargo build %BUILD_FLAGS% -p common >> "%LOG_FILE%" 2>&1

echo %INFO% Building client...
cargo build %BUILD_FLAGS% -p client >> "%LOG_FILE%" 2>&1

echo %INFO% Building server...
cargo build %BUILD_FLAGS% -p server >> "%LOG_FILE%" 2>&1

echo %INFO% Building benchmarks...
cargo build %BUILD_FLAGS% -p benchmarks >> "%LOG_FILE%" 2>&1

echo %SUCCESS% Build completed successfully
goto :eof

REM Function to run tests
:run_tests
echo %INFO% Running tests...

REM Run unit tests
echo %INFO% Running unit tests...
cargo test >> "%LOG_FILE%" 2>&1

REM Run integration tests
echo %INFO% Running integration tests...
cargo test --tests >> "%LOG_FILE%" 2>&1

echo %SUCCESS% All tests passed
goto :eof

REM Function to create distribution package
:create_distribution
echo %INFO% Creating distribution package...

REM Create dist directory structure
mkdir "%DIST_DIR%\bin" 2>nul
mkdir "%DIST_DIR%\config" 2>nul
mkdir "%DIST_DIR%\data" 2>nul
mkdir "%DIST_DIR%\scripts" 2>nul
mkdir "%DIST_DIR%\docs" 2>nul

REM Set binary path
if "%BUILD_TYPE%"=="release" (
    set BINARY_PATH=target\release
) else (
    set BINARY_PATH=target\debug
)

REM Copy binaries
copy "%BINARY_PATH%\client.exe" "%DIST_DIR%\bin\" 2>nul || echo %WARNING% Client binary not found
copy "%BINARY_PATH%\server.exe" "%DIST_DIR%\bin\" 2>nul || echo %WARNING% Server binary not found
copy "%BINARY_PATH%\benchmarks.exe" "%DIST_DIR%\bin\" 2>nul || echo %WARNING% Benchmarks binary not found

REM Copy configuration files
copy "config.toml" "%DIST_DIR%\config\" 2>nul || echo %WARNING% Config file not found

REM Copy sample data
xcopy "data\*" "%DIST_DIR%\data\" /s /y 2>nul || echo %WARNING% Data directory not found

REM Copy scripts
xcopy "scripts\*" "%DIST_DIR%\scripts\" /s /y 2>nul || echo %WARNING% Scripts directory not found

REM Copy documentation
copy "README.md" "%DIST_DIR%\docs\" 2>nul || echo %WARNING% README.md not found
copy "Cargo.toml" "%DIST_DIR%\docs\" 2>nul

echo %SUCCESS% Distribution package created in %DIST_DIR%\
goto :eof

REM Function to create run scripts
:create_run_scripts
echo %INFO% Creating run scripts...

REM Server start script
(
echo @echo off
echo echo Starting ZKP Federated Learning Server...
echo cd /d "%%~dp0"
echo bin\server.exe --config config\config.toml
) > "%DIST_DIR%\start_server.bat"

REM Client start script
(
echo @echo off
echo echo Starting ZKP Federated Learning Client...
echo cd /d "%%~dp0"
echo bin\client.exe --config config\config.toml --server-url http://127.0.0.1:8080
) > "%DIST_DIR%\start_client.bat"

REM Benchmark script
(
echo @echo off
echo echo Running ZKP Federated Learning Benchmarks...
echo cd /d "%%~dp0"
echo bin\benchmarks.exe --config config\config.toml --scenario multi-client-concurrent --num-clients 5 --rounds 3
) > "%DIST_DIR%\run_benchmarks.bat"

REM Visualization script
(
echo @echo off
echo echo Generating benchmark visualizations...
echo cd /d "%%~dp0"
echo python scripts\visualize_benchmarks.py benchmark_results --output visualizations
) > "%DIST_DIR%\visualize_results.bat"

echo %SUCCESS% Run scripts created
goto :eof

REM Function to install Python dependencies
:install_python_deps
echo %INFO% Installing Python dependencies for visualization...

python --version >nul 2>&1
if %errorlevel% equ 0 (
    REM Create requirements.txt for visualization
    (
    echo matplotlib^>=3.5.0
    echo seaborn^>=0.11.0
    echo pandas^>=1.3.0
    echo numpy^>=1.21.0
    echo plotly^>=5.0.0
    echo jupyter^>=1.0.0
    echo scipy^>=1.7.0
    ) > "%DIST_DIR%\scripts\requirements.txt"
    
    REM Try to install dependencies
    pip --version >nul 2>&1
    if %errorlevel% equ 0 (
        echo %INFO% Installing Python packages...
        pip install -r "%DIST_DIR%\scripts\requirements.txt" >> "%LOG_FILE%" 2>&1 || echo %WARNING% Failed to install Python dependencies
    ) else (
        echo %WARNING% pip not found. Install Python dependencies manually: pip install -r scripts\requirements.txt
    )
) else (
    echo %WARNING% Python not found. Visualization scripts will not work.
)

goto :eof

REM Main execution
:main

REM Parse command line arguments
:parse_args
if "%1"=="--release" (
    set BUILD_TYPE=release
    shift
    goto parse_args
)
if "%1"=="--debug" (
    set BUILD_TYPE=debug
    shift
    goto parse_args
)
if "%1"=="--clean" (
    call :clean_build
    shift
    goto parse_args
)
if "%1"=="--no-tests" (
    set SKIP_TESTS=true
    shift
    goto parse_args
)
if "%1"=="--help" (
    echo Usage: %0 [--release^|--debug] [--clean] [--no-tests]
    echo   --release    Build in release mode ^(optimized^)
    echo   --debug      Build in debug mode ^(default^)
    echo   --clean      Clean before building
    echo   --no-tests   Skip running tests
    exit /b 0
)

echo Build started at %date% %time%

REM Execute build steps
call :check_prerequisites
call :build_project

if not "%SKIP_TESTS%"=="true" (
    call :run_tests
)

call :create_distribution
call :install_python_deps
call :create_run_scripts

REM Final success message
echo.
echo %SUCCESS% =========================================
echo %SUCCESS% Build completed successfully!
echo %SUCCESS% =========================================
echo.
echo %INFO% Distribution package: %DIST_DIR%\
echo %INFO% Build log: %LOG_FILE%
echo.
echo %INFO% Next steps:
echo   1. cd %DIST_DIR%
echo   2. start_server.bat
echo   3. start_client.bat ^(in another terminal^)
echo   4. run_benchmarks.bat
echo   5. visualize_results.bat
echo.

pause
