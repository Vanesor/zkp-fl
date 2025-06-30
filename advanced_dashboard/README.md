# ZKP-FL Advanced Dashboard

A comprehensive web-based dashboard for running and visualizing Zero-Knowledge Proof Federated Learning benchmarks.

## Features

### 🚀 Benchmark Execution

- **Configurable Parameters**: Set number of clients, rounds, delays, and concurrency
- **Multiple Scenarios**: Single client, multi-client sequential/concurrent, stress tests
- **Real-time Monitoring**: Live status updates and progress tracking
- **Background Processing**: Non-blocking benchmark execution

### 📊 Advanced Visualizations

- **Training Metrics**: Loss convergence, training time distribution, epochs completion
- **ZKP Metrics**: Proof generation vs verification time, proof size distribution, circuit complexity
- **Performance Analysis**: Scalability analysis, throughput monitoring, resource usage
- **Comparative Analysis**: Cross-scenario comparisons with flexible grouping

### 📈 Real-time Metrics

- Active clients count
- Average proof generation/verification times
- Training performance metrics
- System resource utilization

### 📋 Data Management

- Automatic benchmark data loading
- Historical results table
- Export functionality (JSON format)
- Detailed result inspection

## Quick Start

### Prerequisites

- Python 3.8 or higher
- Rust (for benchmark execution)
- Modern web browser

### Installation & Startup

#### Windows

1. Open Command Prompt or PowerShell
2. Navigate to the advanced_dashboard directory:
   ```cmd
   cd c:\Users\ASUS\OneDrive\Desktop\ZKP\zkp-fl\advanced_dashboard
   ```
3. Run the startup script:
   ```cmd
   start_dashboard.bat
   ```

#### Manual Setup

1. Install Python dependencies:
   ```bash
   pip install -r requirements.txt
   ```
2. Start the server:
   ```bash
   python server.py --host 0.0.0.0 --port 8000
   ```
3. Open your browser and navigate to: http://localhost:8000

## Usage Guide

### Running Benchmarks

1. **Configure Parameters**:

   - **Scenario**: Choose from single-client, multi-client sequential/concurrent, stress test, or custom
   - **Number of Clients**: Set how many clients to simulate (1-100)
   - **Number of Rounds**: Set training rounds per benchmark (1-20)
   - **Client Delay**: Delay between client starts in milliseconds
   - **Max Concurrent**: Maximum concurrent client connections
   - **Server URL**: Optional server endpoint (leave empty for local)

2. **Execute Benchmark**:

   - Click "Run Benchmark" to start
   - Monitor progress in real-time
   - Use "Stop Benchmark" to cancel if needed

3. **View Results**:
   - Metrics update automatically
   - Charts refresh with new data
   - Results appear in the table below

### Dashboard Navigation

#### Training Metrics Tab

- **Loss Convergence**: Shows how loss decreases over epochs for each client
- **Training Time Distribution**: Bar chart of training times across clients
- **Epochs Completed**: Number of training epochs each client completed
- **Dataset Size Distribution**: Pie chart showing dataset size ranges

#### ZKP Metrics Tab

- **Proof Generation vs Verification**: Scatter plot comparing proof times
- **Proof Size Distribution**: Histogram of proof sizes
- **Circuit Complexity**: Radar chart showing circuit parameters
- **ZKP Setup Time**: Line chart of setup times across runs

#### Performance Tab

- **Scalability Analysis**: How performance scales with client count
- **Throughput Over Time**: Operations per second over time
- **System Resource Usage**: CPU and memory utilization
- **Network Latency**: Latency breakdown by component

#### Comparison Tab

- **Flexible Comparisons**: Compare any metric across scenarios, client counts, rounds, or dates
- **Dynamic Grouping**: Change how data is grouped for analysis
- **Interactive Charts**: Hover and click for detailed information

### Data Export

- Click "Export" button to download all benchmark data as JSON
- Exported file includes metrics, configurations, and raw results
- Use for external analysis or reporting

## API Documentation

The dashboard provides a REST API for programmatic access:

### Endpoints

- `GET /api/health` - Health check
- `GET /api/benchmarks` - List all benchmarks
- `GET /api/benchmarks/{session_id}` - Get specific benchmark
- `GET /api/metrics` - Get aggregate metrics
- `POST /api/run-benchmark` - Start new benchmark
- `POST /api/stop-benchmark` - Stop running benchmark
- `GET /api/status` - Get benchmark status
- `GET /api/files` - List benchmark files

### Example API Usage

```python
import requests

# Start a benchmark
response = requests.post('http://localhost:8000/api/run-benchmark', json={
    'command': 'cargo run --bin benchmarks --',
    'config': {
        'scenario': 'multi-client-concurrent',
        'numClients': 5,
        'numRounds': 3,
        'clientDelay': 1000,
        'maxConcurrent': 10
    }
})

# Get metrics
metrics = requests.get('http://localhost:8000/api/metrics').json()
print(f"Average proof time: {metrics['avg_proof_time']}ms")
```

## Architecture

### Frontend (dashboard.js)

- **ZKPDashboard Class**: Main dashboard controller
- **Chart.js Integration**: Interactive visualizations
- **Real-time Updates**: Automatic data refresh
- **Responsive Design**: Works on desktop and mobile

### Backend (server.py)

- **FastAPI Framework**: Modern async Python web framework
- **Background Tasks**: Non-blocking benchmark execution
- **File System Integration**: Direct access to benchmark results
- **CORS Support**: Cross-origin requests enabled

### Data Flow

1. User configures benchmark parameters
2. Frontend sends API request to backend
3. Backend executes Rust benchmark binary
4. Results are saved as JSON files
5. Frontend polls for updates and displays results

## Customization

### Adding New Metrics

1. Modify the benchmark JSON structure
2. Update the data parsing in `dashboard.js`
3. Add new chart configurations
4. Create corresponding UI elements

### Custom Scenarios

- Add new scenario types in the Rust benchmark code
- Update the scenario dropdown in `index.html`
- Implement scenario handling in `server.py`

### Styling

- Modify `styles.css` for visual customization
- Add new CSS classes for custom components
- Update color schemes and layouts

## Troubleshooting

### Common Issues

**Dashboard won't start**

- Check Python installation: `python --version`
- Install dependencies: `pip install -r requirements.txt`
- Check port availability: `netstat -an | findstr 8000`

**Benchmarks fail to run**

- Ensure Rust is installed: `cargo --version`
- Check benchmark binary exists: `cargo build --bin benchmarks`
- Verify working directory is correct

**No data appears**

- Check if benchmark files exist in `../benchmarks/`
- Verify JSON file format is correct
- Check browser console for errors

**Charts not displaying**

- Ensure Chart.js is loaded (check network tab)
- Verify data format matches chart expectations
- Check for JavaScript errors in console

### Debug Mode

Run with debug logging:

```bash
python server.py --debug --reload
```

### Log Files

- Backend logs appear in terminal/command prompt
- Frontend logs visible in browser developer tools
- Benchmark logs saved with results

## Performance Optimization

### Large Datasets

- Dashboard automatically limits table rows to 20 recent results
- Charts aggregate data for better performance
- Use pagination for large result sets

### Memory Usage

- Old benchmark results are loaded on-demand
- Charts destroy previous instances to prevent memory leaks
- Background refresh can be disabled if needed

### Network Optimization

- Static files served efficiently
- JSON responses compressed
- Minimal API calls with smart caching

## Security Considerations

- Dashboard runs on localhost by default
- No authentication required for local use
- For production deployment, add authentication
- Validate all user inputs in production

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

This project is part of the ZKP-FL framework. See the main project license for details.

## Support

For issues and questions:

1. Check this README
2. Review the troubleshooting section
3. Check existing GitHub issues
4. Create a new issue with detailed information
