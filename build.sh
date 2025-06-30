#!/bin/bash

# ZKP Federated Learning - Build and Deployment Script

set -e  # Exit on any error

echo "==================================="
echo "ZKP Federated Learning Build Script"
echo "==================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="zkp-fl"
BUILD_TYPE="${BUILD_TYPE:-release}"
TARGET_DIR="target"
DIST_DIR="dist"
LOG_FILE="build.log"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    # Check Python installation (for visualization scripts)
    if ! command -v python3 &> /dev/null; then
        print_warning "Python3 not found. Visualization scripts will not work."
    fi
    
    # Check required Rust version
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    print_status "Found Rust version: $RUST_VERSION"
    
    print_success "Prerequisites check completed"
}

# Function to clean previous builds
clean_build() {
    print_status "Cleaning previous builds..."
    
    if [ -d "$TARGET_DIR" ]; then
        rm -rf "$TARGET_DIR"
        print_status "Removed target directory"
    fi
    
    if [ -d "$DIST_DIR" ]; then
        rm -rf "$DIST_DIR"
        print_status "Removed dist directory"
    fi
    
    # Clean cargo cache
    cargo clean >> "$LOG_FILE" 2>&1
    
    print_success "Clean completed"
}

# Function to build all crates
build_project() {
    print_status "Building project in $BUILD_TYPE mode..."
    
    # Create log file
    echo "Build started at $(date)" > "$LOG_FILE"
    
    # Build flags
    BUILD_FLAGS=""
    if [ "$BUILD_TYPE" = "release" ]; then
        BUILD_FLAGS="--release"
    fi
    
    # Build all crates
    print_status "Building common library..."
    cargo build $BUILD_FLAGS -p common >> "$LOG_FILE" 2>&1
    
    print_status "Building client..."
    cargo build $BUILD_FLAGS -p client >> "$LOG_FILE" 2>&1
    
    print_status "Building server..."
    cargo build $BUILD_FLAGS -p server >> "$LOG_FILE" 2>&1
    
    print_status "Building benchmarks..."
    cargo build $BUILD_FLAGS -p benchmarks >> "$LOG_FILE" 2>&1
    
    print_success "Build completed successfully"
}

# Function to run tests
run_tests() {
    print_status "Running tests..."
    
    # Run unit tests
    print_status "Running unit tests..."
    cargo test >> "$LOG_FILE" 2>&1
    
    # Run integration tests
    print_status "Running integration tests..."
    cargo test --tests >> "$LOG_FILE" 2>&1
    
    print_success "All tests passed"
}

# Function to create distribution package
create_distribution() {
    print_status "Creating distribution package..."
    
    # Create dist directory structure
    mkdir -p "$DIST_DIR"/{bin,config,data,scripts,docs}
    
    # Copy binaries
    if [ "$BUILD_TYPE" = "release" ]; then
        BINARY_PATH="target/release"
    else
        BINARY_PATH="target/debug"
    fi
    
    cp "$BINARY_PATH/client" "$DIST_DIR/bin/" 2>/dev/null || print_warning "Client binary not found"
    cp "$BINARY_PATH/server" "$DIST_DIR/bin/" 2>/dev/null || print_warning "Server binary not found"
    cp "$BINARY_PATH/benchmarks" "$DIST_DIR/bin/" 2>/dev/null || print_warning "Benchmarks binary not found"
    
    # Copy configuration files
    cp config.toml "$DIST_DIR/config/" 2>/dev/null || print_warning "Config file not found"
    
    # Copy sample data
    cp -r data/* "$DIST_DIR/data/" 2>/dev/null || print_warning "Data directory not found"
    
    # Copy scripts
    cp -r scripts/* "$DIST_DIR/scripts/" 2>/dev/null || print_warning "Scripts directory not found"
    
    # Make scripts executable
    chmod +x "$DIST_DIR/scripts"/*.py 2>/dev/null || true
    chmod +x "$DIST_DIR/scripts"/*.sh 2>/dev/null || true
    
    # Copy documentation
    cp README.md "$DIST_DIR/docs/" 2>/dev/null || print_warning "README.md not found"
    cp Cargo.toml "$DIST_DIR/docs/" 2>/dev/null || true
    
    print_success "Distribution package created in $DIST_DIR/"
}

# Function to install Python dependencies
install_python_deps() {
    print_status "Installing Python dependencies for visualization..."
    
    if command -v python3 &> /dev/null; then
        # Create requirements.txt for visualization
        cat > "$DIST_DIR/scripts/requirements.txt" << EOF
matplotlib>=3.5.0
seaborn>=0.11.0
pandas>=1.3.0
numpy>=1.21.0
plotly>=5.0.0
jupyter>=1.0.0
scipy>=1.7.0
EOF
        
        # Try to install dependencies
        if command -v pip3 &> /dev/null; then
            print_status "Installing Python packages..."
            pip3 install -r "$DIST_DIR/scripts/requirements.txt" >> "$LOG_FILE" 2>&1 || \
                print_warning "Failed to install Python dependencies. Install manually: pip3 install -r scripts/requirements.txt"
        else
            print_warning "pip3 not found. Install Python dependencies manually: pip3 install -r scripts/requirements.txt"
        fi
    else
        print_warning "Python3 not found. Visualization scripts will not work."
    fi
}

# Function to create run scripts
create_run_scripts() {
    print_status "Creating run scripts..."
    
    # Server start script
    cat > "$DIST_DIR/start_server.sh" << 'EOF'
#!/bin/bash
echo "Starting ZKP Federated Learning Server..."
cd "$(dirname "$0")"
./bin/server --config config/config.toml
EOF
    
    # Client start script  
    cat > "$DIST_DIR/start_client.sh" << 'EOF'
#!/bin/bash
echo "Starting ZKP Federated Learning Client..."
cd "$(dirname "$0")"
./bin/client --config config/config.toml --server-url http://127.0.0.1:8080
EOF
    
    # Benchmark script
    cat > "$DIST_DIR/run_benchmarks.sh" << 'EOF'
#!/bin/bash
echo "Running ZKP Federated Learning Benchmarks..."
cd "$(dirname "$0")"
./bin/benchmarks --config config/config.toml --scenario multi-client-concurrent --num-clients 5 --rounds 3
EOF
    
    # Visualization script
    cat > "$DIST_DIR/visualize_results.sh" << 'EOF'
#!/bin/bash
echo "Generating benchmark visualizations..."
cd "$(dirname "$0")"
python3 scripts/visualize_benchmarks.py benchmark_results --output visualizations
EOF
    
    # Make scripts executable
    chmod +x "$DIST_DIR"/*.sh
    
    print_success "Run scripts created"
}

# Function to create README for distribution
create_distribution_readme() {
    cat > "$DIST_DIR/README.md" << 'EOF'
# ZKP Federated Learning - Distribution Package

This package contains a complete zero-knowledge proof federated learning system built with Rust and the Protostar library.

## Quick Start

### 1. Start the Server
```bash
./start_server.sh
```

### 2. Run a Client (in another terminal)
```bash
./start_client.sh
```

### 3. Run Benchmarks
```bash
./run_benchmarks.sh
```

### 4. Visualize Results
```bash
./visualize_results.sh
```

## Directory Structure

- `bin/` - Compiled binaries (client, server, benchmarks)
- `config/` - Configuration files
- `data/` - Sample datasets
- `scripts/` - Python visualization scripts
- `docs/` - Documentation

## Configuration

Edit `config/config.toml` to customize:
- Server settings (host, port)
- ZKP parameters (circuit size, security level)
- Training parameters (learning rate, epochs)
- Benchmark settings

## Components

### Server (`bin/server`)
- Receives and verifies zero-knowledge proofs
- Stores training results
- Provides metrics and monitoring

### Client (`bin/client`)
- Trains linear regression models on healthcare data
- Generates zero-knowledge proofs of training
- Submits proofs to server

### Benchmarks (`bin/benchmarks`)
- Tests system performance with multiple clients
- Measures throughput, latency, and reliability
- Supports various scenarios (sequential, concurrent, stress test)

## Visualization

The Python scripts in `scripts/` generate:
- Performance overview charts
- Scalability analysis
- Timing breakdowns
- Proof size analysis
- Summary reports

Install Python dependencies:
```bash
pip3 install -r scripts/requirements.txt
```

## Healthcare Dataset

The sample dataset (`data/healthcare_sample.csv`) contains synthetic patient data:
- Demographics (age, BMI)
- Vital signs (blood pressure, heart rate)
- Lab results (cholesterol, glucose)
- Lifestyle factors (smoking, exercise)
- Risk scores (target variable)

## Zero-Knowledge Proofs

The system uses the Protostar proving system for:
- Proving correct model training
- Preserving data privacy
- Enabling verifiable federated learning

## System Requirements

- Rust 1.70+ 
- 4GB+ RAM for ZKP generation
- Python 3.8+ (for visualizations)

## Support

For issues and questions, please refer to the project documentation.
EOF
}

# Function to generate performance report
generate_performance_report() {
    print_status "Generating performance report..."
    
    # Get binary sizes
    BINARY_SIZES=""
    for binary in client server benchmarks; do
        if [ -f "$BINARY_PATH/$binary" ]; then
            SIZE=$(du -h "$BINARY_PATH/$binary" | cut -f1)
            BINARY_SIZES="$BINARY_SIZES\n  $binary: $SIZE"
        fi
    done
    
    # Get build information
    BUILD_TIME=$(date)
    RUST_VERSION=$(rustc --version)
    
    cat > "$DIST_DIR/BUILD_INFO.txt" << EOF
ZKP Federated Learning - Build Information
==========================================

Build Time: $BUILD_TIME
Build Type: $BUILD_TYPE
Rust Version: $RUST_VERSION

Binary Sizes:$BINARY_SIZES

Build Log: See $LOG_FILE for detailed build output

Components:
- Common Library: Shared types, circuits, and utilities
- Client: Model training and proof generation
- Server: Proof verification and storage
- Benchmarks: Performance testing framework

Features:
- Zero-knowledge proof generation using Protostar
- Linear regression on healthcare datasets
- Multi-client benchmarking
- Performance visualization
- JSON-based proof storage
- RESTful API for proof submission
EOF
    
    print_success "Performance report generated"
}

# Main execution flow
main() {
    echo "Build started at $(date)"
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --release)
                BUILD_TYPE="release"
                shift
                ;;
            --debug)
                BUILD_TYPE="debug"
                shift
                ;;
            --clean)
                clean_build
                shift
                ;;
            --no-tests)
                SKIP_TESTS=true
                shift
                ;;
            --help)
                echo "Usage: $0 [--release|--debug] [--clean] [--no-tests]"
                echo "  --release    Build in release mode (optimized)"
                echo "  --debug      Build in debug mode (default)"
                echo "  --clean      Clean before building"
                echo "  --no-tests   Skip running tests"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Execute build steps
    check_prerequisites
    
    if [ "$1" = "--clean" ] || [ "$2" = "--clean" ]; then
        clean_build
    fi
    
    build_project
    
    if [ "$SKIP_TESTS" != "true" ]; then
        run_tests
    fi
    
    create_distribution
    install_python_deps
    create_run_scripts
    create_distribution_readme
    generate_performance_report
    
    # Final success message
    echo ""
    print_success "========================================="
    print_success "Build completed successfully!"
    print_success "========================================="
    echo ""
    print_status "Distribution package: $DIST_DIR/"
    print_status "Build log: $LOG_FILE"
    echo ""
    print_status "Next steps:"
    echo "  1. cd $DIST_DIR"
    echo "  2. ./start_server.sh"
    echo "  3. ./start_client.sh (in another terminal)"
    echo "  4. ./run_benchmarks.sh"
    echo "  5. ./visualize_results.sh"
    echo ""
}

# Run main function
main "$@"
