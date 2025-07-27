from flask import Flask, render_template, jsonify, request
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
    return render_template('index.html', data=data)

@app.route('/run-benchmark', methods=['POST'])
def run_benchmark():
    try:
        print("Starting benchmark process...")
        
        # Get parameters from request
        request_data = request.get_json()
        num_clients = request_data.get('num_clients', 1)
        num_rounds = request_data.get('num_rounds', 1)
        
        # Validate parameters
        if not isinstance(num_clients, int) or num_clients < 1 or num_clients > 100:
            return jsonify({
                'success': False,
                'error': 'Number of clients must be an integer between 1 and 100'
            })
            
        if not isinstance(num_rounds, int) or num_rounds < 1 or num_rounds > 50:
            return jsonify({
                'success': False,
                'error': 'Number of rounds must be an integer between 1 and 50'
            })
        
        print(f"Running benchmark with {num_clients} clients and {num_rounds} rounds")
        
        # Get the project root directory
        root_dir = os.path.dirname(os.path.dirname(__file__))
        
        # Build the benchmark command dynamically
        benchmark_command = [
            'cargo', 'run', '--bin', 'benchmarks', '--',
            '--scenario', 'single-client',
            '--num-clients', str(num_clients),
            '--rounds', str(num_rounds),
            '--client-delay-ms', '0',
            '--max-concurrent', str(min(num_clients, 10)),  # Limit concurrent clients
            '--verbose'
        ]
        
        print(f"Executing command: {' '.join(benchmark_command)}")
        
        # Run the benchmark command
        process = subprocess.Popen(
            benchmark_command,
            cwd=root_dir,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        stdout, stderr = process.communicate(timeout=300)  # 5 minute timeout
        print(f"Benchmark process output: {stdout}")
        if stderr:
            print(f"Benchmark process errors: {stderr}")
        
        if process.returncode != 0:
            error_msg = f"Benchmark failed with return code {process.returncode}."
            if stderr:
                error_msg += f" Error: {stderr}"
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
                'error': "Could not load benchmark data after running. Please check if benchmark files were generated."
            })
        
        # Add configuration info to the data
        data['config'] = {
            'num_clients': num_clients,
            'num_rounds': num_rounds
        }
            
        return jsonify({
            'success': True,
            'data': data
        })
        
    except subprocess.TimeoutExpired:
        return jsonify({
            'success': False,
            'error': 'Benchmark execution timed out after 5 minutes. Try reducing the number of clients or rounds.'
        })
    except Exception as e:
        print(f"Unexpected error: {str(e)}")
        return jsonify({
            'success': False,
            'error': f'Unexpected error: {str(e)}'
        })

@app.route('/get-benchmark-history', methods=['GET'])
def get_benchmark_history():
    """Get historical benchmark data for comparison"""
    try:
        benchmark_dir = os.path.join(os.path.dirname(os.path.dirname(__file__)), 'benchmarks')
        
        # Get all benchmark files
        client_benchmarks = glob.glob(os.path.join(benchmark_dir, 'benchmark_*_client_*.json'))
        
        if not client_benchmarks:
            return jsonify({
                'success': True,
                'data': []
            })
        
        # Sort by creation time and get last 10
        client_benchmarks.sort(key=os.path.getctime, reverse=True)
        recent_benchmarks = client_benchmarks[:10]
        
        history_data = []
        for benchmark_file in recent_benchmarks:
            try:
                with open(benchmark_file, 'r') as f:
                    data = json.load(f)
                    
                history_entry = {
                    'timestamp': data['start_time'],
                    'final_loss': data['training_metrics']['final_loss'],
                    'training_time': data['training_metrics']['training_time_ms'],
                    'total_zkp_time': (
                        data['zkp_metrics']['setup_time_ms'] +
                        data['zkp_metrics']['witness_generation_time_ms'] +
                        data['zkp_metrics']['proof_generation_time_ms'] +
                        data['zkp_metrics']['proof_verification_time_ms']
                    ),
                    'proof_size': data['zkp_metrics']['proof_size_bytes']
                }
                history_data.append(history_entry)
            except (KeyError, json.JSONDecodeError) as e:
                print(f"Error parsing benchmark file {benchmark_file}: {e}")
                continue
        
        return jsonify({
            'success': True,
            'data': history_data
        })
        
    except Exception as e:
        return jsonify({
            'success': False,
            'error': f'Error retrieving benchmark history: {str(e)}'
        })

if __name__ == '__main__':
    app.run(debug=True, port=8000)