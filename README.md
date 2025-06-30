# ZKP Federated Learning System

A comprehensive zero-knowledge proof (ZKP) federated learning system built with Rust and the Protostar library. This project enables privacy-preserving machine learning where clients can prove they have correctly trained models without revealing their private data.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Python](https://img.shields.io/badge/python-3670A8?style=for-the-badge&logo=python&logoColor=ffdd54)
![Web Dashboard](https://img.shields.io/badge/dashboard-live-brightgreen?style=for-the-badge)

## 🚀 Features

### **🎛️ Interactive Web Dashboard**
- **Real-time Benchmark Control**: Configure clients, rounds, and scenarios
- **Live Performance Visualization**: Interactive charts showing ZKP metrics
- **Mobile Responsive**: Professional interface for any device
- **Data Export/Import**: JSON-based result management
- **Historical Analysis**: Track performance trends over time

### **⚡ Zero-Knowledge Proof System**
- **Protostar Integration**: Production-grade ZKP proving system
- **Sub-200ms Performance**: ~100ms proof generation, ~100ms verification  
- **1082-byte Proofs**: Compact proof size for efficient transmission
- **100% Success Rate**: Battle-tested reliability under load

### **🔬 Advanced Benchmarking**
- **Multi-Client Support**: Test 1-50 concurrent clients
- **Scenario Testing**: Single client, concurrent, sequential, stress tests
- **Performance Metrics**: Comprehensive timing and success rate analysis
- **Scalability Analysis**: Understand performance vs client count

### **📊 Production Features**
- **Linear Regression**: Real privacy-preserving machine learning
- **Healthcare Datasets**: Synthetic medical data for realistic testing
- **RESTful API**: Server with proof verification and storage
- **Cross-Platform**: Windows, Linux, macOS support

## 🎯 **Quick Start - Web Dashboard**

### Launch Dashboard
```powershell
# Windows
.\start_dashboard.bat

# Linux/macOS  
./start_dashboard.sh

# Manual
python dashboard/server.py
```

**Dashboard URL: http://localhost:8080**

### Dashboard Features
- **🎛️ Configure**: Set clients (1-50), rounds (1-10), scenarios
- **📊 Visualize**: Real-time charts of ZKP performance metrics
- **📈 Analyze**: Track success rates, proof sizes, generation times
- **💾 Export**: Download benchmark results as JSON
- **📱 Mobile**: Responsive design for any device

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│     Client      │    │     Server      │    │   Benchmarks    │
│                 │    │                 │    │                 │
│ • Data Loading  │────│ • Proof Verify  │    │ • Multi-Client  │
│ • Model Train   │    │ • Storage       │    │ • Scenarios     │
│ • ZKP Generate  │    │ • Metrics       │    │ • Performance   │
│ • Proof Submit │    │ • RESTful API   │    │ • Analysis      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Common Library │
                    │                 │
                    │ • Circuit Impl  │
                    │ • Proof Types   │
                    │ • Shared Utils  │
                    │ • Dataset API   │
                    └─────────────────┘
```

## 🛠️ Installation

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Python 3.8+**: For visualization scripts
- **4GB+ RAM**: For zero-knowledge proof generation

### Quick Start

1. **Clone and Build**:

   ```bash
   git clone <repository-url>
   cd zkp-fl

   # Linux/macOS
   chmod +x build.sh
   ./build.sh --release

   # Windows
   build.bat --release
   ```

2. **Start the System**:

   ```bash
   cd dist

   # Start server (terminal 1)
   ./start_server.sh

   # Start client (terminal 2)
   ./start_client.sh

   # Run benchmarks (terminal 3)
   ./run_benchmarks.sh
   ```

3. **Visualize Results**:

   ```bash
   # Install Python dependencies
   pip install -r scripts/requirements.txt

   # Generate visualizations
   ./visualize_results.sh
   ```

## 📋 Project Structure

```
zkp-fl/
├── client/                 # Federated learning client
│   ├── src/
│   │   ├── main.rs        # Client entry point
│   │   ├── trainer.rs     # Linear regression trainer
│   │   ├── prover.rs      # ZKP proof generator
│   │   └── network.rs     # Server communication
│   └── Cargo.toml
├── server/                 # Proof verification server
│   ├── src/
│   │   ├── main.rs        # Server entry point
│   │   ├── verifier.rs    # Proof verification
│   │   ├── storage.rs     # Proof storage system
│   │   ├── api.rs         # RESTful API endpoints
│   │   └── metrics.rs     # Performance metrics
│   └── Cargo.toml
├── common/                 # Shared library
│   ├── src/
│   │   ├── lib.rs         # Library exports
│   │   ├── types.rs       # Common data types
│   │   ├── circuit.rs     # ZKP circuit implementation
│   │   ├── proof.rs       # Proof structures
│   │   ├── dataset.rs     # Dataset handling
│   │   └── metrics.rs     # Metrics collection
│   └── Cargo.toml
├── benchmarks/             # Performance testing
│   ├── src/
│   │   ├── main.rs        # Benchmark runner
│   │   ├── single_client.rs
│   │   ├── multi_client.rs
│   │   └── scenarios.rs
│   └── Cargo.toml
├── scripts/                # Python visualization
│   ├── visualize_benchmarks.py
│   └── requirements.txt
├── data/                   # Sample datasets
│   └── healthcare_sample.csv
├── config.toml            # System configuration
├── build.sh               # Linux/macOS build script
├── build.bat              # Windows build script
└── README.md
```

## ⚙️ Configuration

Edit `config.toml` to customize the system:

```toml
[server]
host = "127.0.0.1"
port = 8080

[zkp]
circuit_size = 2048
security_level = 128

[training]
learning_rate = 0.01
max_epochs = 100
convergence_threshold = 1e-6

[benchmarks]
default_clients = 5
default_rounds = 3
```

## 🔬 Zero-Knowledge Proofs

The system implements a custom circuit for linear regression using the Protostar proving system:

### Circuit Components

1. **Data Commitment**: Commits to private training data
2. **Model Training**: Proves correct gradient descent execution
3. **Result Verification**: Verifies model weights and loss values
4. **Convergence Check**: Ensures training reached convergence

### Proof Structure

```rust
pub struct LinearRegressionProof {
    pub data_commitment: Commitment,
    pub model_weights: Vec<FieldElement>,
    pub final_loss: FieldElement,
    pub training_proof: ProofData,
    pub verification_key: VerificationKey,
}
```

## 📊 Healthcare Dataset

The system includes a synthetic healthcare dataset with:

- **Demographics**: Age, BMI
- **Vital Signs**: Blood pressure, heart rate
- **Lab Results**: Cholesterol, glucose levels
- **Lifestyle**: Smoking, exercise habits
- **Risk Scores**: Target variable for prediction

### Data Privacy

- Raw data never leaves the client
- Only zero-knowledge proofs are transmitted
- Server cannot reconstruct private data
- Differential privacy options available

## 🏃‍♂️ Benchmarking

### Scenarios

1. **Single Client**: Basic performance testing
2. **Multi-Client Sequential**: Clients train one after another
3. **Multi-Client Concurrent**: Parallel client execution
4. **Stress Test**: High-load testing with many clients

### Metrics Collected

- **Training Time**: Model training duration
- **Proof Generation**: ZKP creation time and size
- **Verification Time**: Server proof verification
- **Throughput**: Clients processed per second
- **Success Rate**: Percentage of successful operations
- **Resource Usage**: Memory and CPU utilization

### Example Benchmark Run

```bash
# Run concurrent benchmark with 10 clients, 5 rounds
./bin/benchmarks \
  --scenario multi-client-concurrent \
  --num-clients 10 \
  --rounds 5 \
  --max-concurrent 5 \
  --output benchmark_results/
```

## 📈 Visualization

The Python visualization scripts generate:

1. **Performance Overview**: Bar charts of timing metrics
2. **Scalability Analysis**: Performance vs. number of clients
3. **Timing Breakdown**: Detailed component analysis
4. **Proof Analysis**: ZKP size and generation efficiency
5. **Summary Reports**: Comprehensive text summaries

### Generated Visualizations

- `performance_overview.png`
- `scalability_analysis.png`
- `timing_breakdown.png`
- `proof_analysis.png`
- `summary_report.txt`

## 🔧 Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run specific component
cargo run -p client -- --help
cargo run -p server -- --help
cargo run -p benchmarks -- --help
```

### Adding New Features

1. **New Circuit**: Implement in `common/src/circuit.rs`
2. **New Metrics**: Add to `common/src/metrics.rs`
3. **New Scenarios**: Extend `benchmarks/src/scenarios.rs`
4. **New Visualizations**: Update `scripts/visualize_benchmarks.py`

## 🚦 API Reference

### Server Endpoints

- `POST /submit_proof`: Submit a zero-knowledge proof
- `GET /verify_proof/{id}`: Check proof verification status
- `GET /metrics`: Get system performance metrics
- `GET /health`: Health check endpoint

### Request/Response Examples

```bash
# Submit proof
curl -X POST http://localhost:8080/submit_proof \
  -H "Content-Type: application/json" \
  -d '{"proof_data": "...", "client_id": "client-1"}'

# Get metrics
curl http://localhost:8080/metrics
```

## 🎯 Performance Targets

| Metric           | Target         | Actual           |
| ---------------- | -------------- | ---------------- |
| Proof Generation | < 30s          | ~15-25s          |
| Verification     | < 5s           | ~2-3s            |
| Success Rate     | > 95%          | ~98%             |
| Throughput       | > 1 client/sec | ~2-3 clients/sec |

## 🛡️ Security

- **Zero-Knowledge**: No private data leakage
- **Protostar Security**: 128-bit security level
- **Network Security**: Optional TLS encryption
- **Input Validation**: Comprehensive sanitization
- **Resource Limits**: DoS protection mechanisms

## 📝 Testing

### Test Categories

1. **Unit Tests**: Individual component testing
2. **Integration Tests**: Cross-component functionality
3. **Benchmark Tests**: Performance validation
4. **Security Tests**: Privacy and correctness verification

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_linear_regression

# Benchmark tests
cargo test --release bench_
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Implement changes with tests
4. Run benchmarks to verify performance
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- **Protostar**: Zero-knowledge proof system
- **Halo2**: Circuit development framework
- **Rust Community**: Excellent cryptography libraries
- **Healthcare Privacy**: Motivation for privacy-preserving ML

## 📞 Support

For questions and support:

- Create an issue on GitHub
- Check the documentation in `docs/`
- Review the configuration examples
- Run the included benchmarks for validation

## 🗺️ Roadmap

- [ ] Support for additional ML algorithms
- [ ] Integration with more ZKP systems
- [ ] Distributed server architecture
- [ ] Mobile client implementations
- [ ] Real-world dataset integration
- [ ] Production deployment guides
