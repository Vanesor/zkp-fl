from flask import Flask, render_template, jsonify
import json
import os
from datetime import datetime
import glob
import subprocess
import time

app = Flask(__name__)

def load_benchmark_data():
    # Get the most recent benchmark files
    benchmark_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'benchmarks')
    
    # Find the latest benchmark files
    client_benchmarks = glob.glob(os.path.join(benchmark_dir, 'benchmark_*_client_*.json'))
    summary_file = glob.glob(os.path.join(benchmark_dir, 'benchmark_summary_*.txt'))
    
    if not client_benchmarks or not summary_file:
        return None
    
    # Get latest files based on timestamp
    latest_client_benchmark = max(client_benchmarks, key=os.path.getctime)
    latest_summary = max(summary_file, key=os.path.getctime)
    
    # Load client benchmark data
    with open(latest_client_benchmark, 'r') as f:
        client_data = json.load(f)
        
    # Load summary data
    with open(latest_summary, 'r') as f:
        summary_data = f.read()
    
    # Extract training metrics
    training_metrics = {
        'loss_history': client_data['training_metrics']['loss_history'],
        'epochs': list(range(1, len(client_data['training_metrics']['loss_history']) + 1)),
        'final_loss': client_data['training_metrics']['final_loss'],
        'initial_loss': client_data['training_metrics']['initial_loss'],
        'training_time': client_data['training_metrics']['training_time_ms']
    }
    
    # Extract ZKP metrics
    zkp_metrics = {
        'setup_time': client_data['zkp_metrics']['setup_time_ms'],
        'witness_gen_time': client_data['zkp_metrics']['witness_generation_time_ms'],
        'proof_gen_time': client_data['zkp_metrics']['proof_generation_time_ms'],
        'proof_verify_time': client_data['zkp_metrics']['proof_verification_time_ms'],
        'proof_size': client_data['zkp_metrics']['proof_size_bytes']
    }
    
    return {
        'training_metrics': training_metrics,
        'zkp_metrics': zkp_metrics,
        'summary': summary_data,
        'timestamp': client_data['start_time']
    }

@app.route('/')
def index():
    data = load_benchmark_data()
    if data is None:
        return "No benchmark data found"
    return render_template('index.html', data=data)

@app.route('/run-benchmark', methods=['POST'])
def run_benchmark():
    try:
        print("Starting benchmark process...")
        # Get the project root directory
        root_dir = os.path.dirname(os.path.dirname(__file__))
        
        # Command to run the benchmark
        benchmark_script = os.path.join(os.path.dirname(__file__), "benchmark.bat")
        
        # Ensure the benchmark script exists
        if not os.path.exists(benchmark_script):
            raise FileNotFoundError(f"Benchmark script not found at {benchmark_script}")
            
        print(f"Running benchmark script from: {benchmark_script}")
        
        # Run the benchmark command
        process = subprocess.Popen(
            benchmark_script,
            cwd=root_dir,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            shell=True
        )
        
        stdout, stderr = process.communicate()
        print(f"Benchmark process output: {stdout}")
        print(f"Benchmark process errors: {stderr}")
        
        if process.returncode != 0:
            error_msg = f"Benchmark failed with return code {process.returncode}. Error: {stderr}"
            print(error_msg)
            return jsonify({
                'success': False,
                'error': error_msg
            })
            
        # Wait a moment for files to be written
        time.sleep(2)
        
        # Load the new benchmark data
        data = load_benchmark_data()
        
        if data is None:
            return jsonify({
                'success': False,
                'error': "Could not load benchmark data after running"
            })
            
        return jsonify({
            'success': True,
            'data': data
        })
        
    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e)
        })

if __name__ == '__main__':
    app.run(debug=True, port=8000)
