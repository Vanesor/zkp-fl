#!/usr/bin/env python3
"""
ZKP-FL Advanced Dashboard Backend Server
Handles benchmark execution, data loading, and API endpoints
"""

import asyncio
import json
import os
import glob
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Any
import argparse

try:
    from fastapi import FastAPI, HTTPException, BackgroundTasks
    from fastapi.staticfiles import StaticFiles
    from fastapi.responses import HTMLResponse, JSONResponse
    from fastapi.middleware.cors import CORSMiddleware
    from pydantic import BaseModel
    import uvicorn
except ImportError:
    print("Installing required dependencies...")
    subprocess.check_call([sys.executable, "-m", "pip", "install", "fastapi", "uvicorn[standard]", "pydantic"])
    from fastapi import FastAPI, HTTPException, BackgroundTasks
    from fastapi.staticfiles import StaticFiles
    from fastapi.responses import HTMLResponse, JSONResponse
    from fastapi.middleware.cors import CORSMiddleware
    from pydantic import BaseModel
    import uvicorn

# Configuration
BENCHMARK_DIR = Path("../benchmarks")
WORKSPACE_DIR = Path("..")
DASHBOARD_DIR = Path(".")

# Global state
app = FastAPI(title="ZKP-FL Dashboard API", version="1.0.0")
current_benchmark_process: Optional[subprocess.Popen] = None
benchmark_status = {"running": False, "message": ""}

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Pydantic models
class BenchmarkConfig(BaseModel):
    scenario: str
    numClients: int
    numRounds: int
    clientDelay: int
    maxConcurrent: int
    serverUrl: Optional[str] = None

class BenchmarkRequest(BaseModel):
    command: str
    config: BenchmarkConfig

class BenchmarkMetrics(BaseModel):
    setup_time_ms: float
    witness_generation_time_ms: float
    proof_generation_time_ms: float
    proof_verification_time_ms: float
    proof_size_bytes: int
    circuit_constraints: int
    circuit_advice_columns: int
    circuit_fixed_columns: int
    folding_iterations: int

class TrainingMetrics(BaseModel):
    dataset_size: int
    num_features: int
    training_time_ms: float
    epochs_completed: int
    final_loss: float
    initial_loss: float
    convergence_epoch: Optional[int]
    loss_history: List[float]

class BenchmarkResult(BaseModel):
    session_id: str
    client_id: str
    start_time: str
    end_time: str
    total_duration_ms: float
    zkp_metrics: BenchmarkMetrics
    training_metrics: TrainingMetrics
    system_metrics: List[Dict]
    scenario: Optional[str] = None

# Utility functions
def load_benchmark_files() -> List[Dict]:
    """Load all benchmark JSON files from the benchmarks directory."""
    try:
        benchmark_files = []
        patterns = [
            BENCHMARK_DIR / "benchmark_*.json",
            BENCHMARK_DIR / "benchmark_report_*.json"
        ]
        
        print(f"DEBUG: Looking for files in {BENCHMARK_DIR.absolute()}")
        print(f"DEBUG: Current working directory: {os.getcwd()}")
        
        all_files = []
        for pattern in patterns:
            pattern_files = glob.glob(str(pattern))
            print(f"DEBUG: Pattern {pattern} found {len(pattern_files)} files")
            all_files.extend(pattern_files)
        
        print(f"DEBUG: Total files found: {len(all_files)}")
        
        # Sort by modification time (newest first)
        all_files.sort(key=os.path.getmtime, reverse=True)
        
        for file_path in all_files:
            try:
                with open(file_path, 'r') as f:
                    data = json.load(f)
                    
                    # Check if this is a benchmark report file (has client_results)
                    if 'client_results' in data:
                        # This is a multi-client benchmark report - extract individual results
                        for client_result in data['client_results']:
                            # Add metadata
                            client_result['filename'] = os.path.basename(file_path)
                            client_result['file_mtime'] = os.path.getmtime(file_path)
                            
                            # Add report-level metadata
                            client_result['report_id'] = data.get('benchmark_id', 'unknown')
                            client_result['num_clients'] = data.get('num_clients', 1)
                            
                            # Infer scenario from filename or data
                            if 'scenario' not in client_result:
                                if 'multi' in file_path.lower():
                                    client_result['scenario'] = 'multi-client'
                                elif 'single' in file_path.lower():
                                    client_result['scenario'] = 'single-client'
                                else:
                                    client_result['scenario'] = 'unknown'
                            
                            benchmark_files.append(client_result)
                    else:
                        # This is an individual benchmark file
                        # Add metadata
                        data['filename'] = os.path.basename(file_path)
                        data['file_mtime'] = os.path.getmtime(file_path)
                        
                        # Infer scenario from filename or data
                        if 'scenario' not in data:
                            if 'multi' in file_path.lower():
                                data['scenario'] = 'multi-client'
                            elif 'single' in file_path.lower():
                                data['scenario'] = 'single-client'
                            else:
                                data['scenario'] = 'unknown'
                        
                        benchmark_files.append(data)
            except (json.JSONDecodeError, KeyError) as e:
                print(f"Warning: Failed to load {file_path}: {e}")
                continue
                
        print(f"Loaded {len(benchmark_files)} benchmark files")
        return benchmark_files
        
    except Exception as e:
        print(f"Error loading benchmark files: {e}")
        return []

def calculate_aggregate_metrics(benchmarks: List[Dict]) -> Dict:
    """Calculate aggregate metrics from benchmark data."""
    if not benchmarks:
        return {
            "total_benchmarks": 0,
            "avg_proof_time": 0,
            "avg_verify_time": 0,
            "avg_training_time": 0,
            "avg_final_loss": 0,
            "avg_proof_size": 0,
            "total_duration": 0
        }
    
    totals = {
        "proof_time": 0,
        "verify_time": 0,
        "training_time": 0,
        "final_loss": 0,
        "proof_size": 0,
        "duration": 0
    }
    
    count = 0
    for benchmark in benchmarks:
        if isinstance(benchmark, dict):
            zkp_metrics = benchmark.get('zkp_metrics', {})
            training_metrics = benchmark.get('training_metrics', {})
            
            totals["proof_time"] += zkp_metrics.get('proof_generation_time_ms', 0)
            totals["verify_time"] += zkp_metrics.get('proof_verification_time_ms', 0)
            totals["training_time"] += training_metrics.get('training_time_ms', 0)
            totals["final_loss"] += training_metrics.get('final_loss', 0)
            totals["proof_size"] += zkp_metrics.get('proof_size_bytes', 0)
            totals["duration"] += benchmark.get('total_duration_ms', 0)
            count += 1
    
    if count == 0:
        count = 1
    
    return {
        "total_benchmarks": len(benchmarks),
        "avg_proof_time": round(totals["proof_time"] / count, 2),
        "avg_verify_time": round(totals["verify_time"] / count, 2),
        "avg_training_time": round(totals["training_time"] / count, 2),
        "avg_final_loss": round(totals["final_loss"] / count, 4),
        "avg_proof_size": round(totals["proof_size"] / count, 2),
        "total_duration": round(totals["duration"] / count, 2)
    }

def build_benchmark_command(config: BenchmarkConfig) -> List[str]:
    """Build the benchmark command from configuration."""
    cmd = ["cargo", "run", "--bin", "benchmarks", "--"]
    
    # Map frontend scenario names to backend values
    scenario_mapping = {
        "single-client": "single-client",
        "multi-client-sequential": "multi-client-sequential", 
        "multi-client-concurrent": "multi-client-concurrent",
        "stress-test": "stress-test",
        "custom": "custom"
    }
    
    scenario = scenario_mapping.get(config.scenario, config.scenario)
    cmd.extend(["--scenario", scenario])
    cmd.extend(["--num-clients", str(config.numClients)])
    cmd.extend(["--rounds", str(config.numRounds)])
    cmd.extend(["--client-delay-ms", str(config.clientDelay)])
    cmd.extend(["--max-concurrent", str(config.maxConcurrent)])
    
    if config.serverUrl:
        cmd.extend(["--server-url", config.serverUrl])
    
    cmd.append("--verbose")
    
    return cmd

async def run_benchmark_process(config: BenchmarkConfig) -> Dict:
    """Run the benchmark process asynchronously."""
    global current_benchmark_process, benchmark_status
    
    # Save current working directory
    original_cwd = os.getcwd()
    
    try:
        benchmark_status["running"] = True
        benchmark_status["message"] = "Initializing benchmark..."
        
        # Change to workspace directory
        os.chdir(WORKSPACE_DIR)
        
        # Build command
        cmd = build_benchmark_command(config)
        print(f"Running command: {' '.join(cmd)}")
        
        # Start process
        current_benchmark_process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            cwd=WORKSPACE_DIR
        )
        
        benchmark_status["message"] = "Benchmark running..."
        
        # Wait for completion
        stdout, stderr = current_benchmark_process.communicate()
        return_code = current_benchmark_process.returncode
        
        if return_code == 0:
            benchmark_status["message"] = "Benchmark completed successfully"
            return {
                "success": True,
                "message": "Benchmark completed successfully",
                "stdout": stdout,
                "stderr": stderr
            }
        else:
            benchmark_status["message"] = f"Benchmark failed with code {return_code}"
            return {
                "success": False,
                "message": f"Benchmark failed with return code {return_code}",
                "stdout": stdout,                "stderr": stderr
            }
            
    except Exception as e:
        benchmark_status["message"] = f"Benchmark error: {str(e)}"
        return {
            "success": False,
            "message": f"Error running benchmark: {str(e)}",
            "stdout": "",
            "stderr": str(e)
        }
    finally:
        benchmark_status["running"] = False
        current_benchmark_process = None
        # Restore original working directory
        os.chdir(original_cwd)

# API Routes
@app.get("/api/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy", "timestamp": datetime.now().isoformat()}

@app.get("/api/benchmarks")
async def get_benchmarks():
    """Get all benchmark results."""
    try:
        benchmarks = load_benchmark_files()
        return {
            "benchmarks": benchmarks,
            "count": len(benchmarks),
            "metrics": calculate_aggregate_metrics(benchmarks)
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to load benchmarks: {str(e)}")

@app.get("/api/benchmarks/{session_id}")
async def get_benchmark(session_id: str):
    """Get a specific benchmark by session ID."""
    try:
        benchmarks = load_benchmark_files()
        benchmark = next((b for b in benchmarks if b.get('session_id') == session_id), None)
        
        if not benchmark:
            raise HTTPException(status_code=404, detail="Benchmark not found")
        
        return benchmark
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to get benchmark: {str(e)}")

@app.get("/api/metrics")
async def get_metrics():
    """Get aggregate metrics."""
    try:
        benchmarks = load_benchmark_files()
        return calculate_aggregate_metrics(benchmarks)
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to calculate metrics: {str(e)}")

@app.post("/api/run-benchmark")
async def run_benchmark(request: BenchmarkRequest, background_tasks: BackgroundTasks):
    """Start a new benchmark run."""
    global benchmark_status
    
    if benchmark_status["running"]:
        raise HTTPException(status_code=400, detail="Benchmark already running")
    
    try:
        # Start benchmark in background
        background_tasks.add_task(run_benchmark_process, request.config)
        
        return {
            "success": True,
            "message": "Benchmark started",
            "config": request.config.dict()
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to start benchmark: {str(e)}")

@app.post("/api/stop-benchmark")
async def stop_benchmark():
    """Stop the current benchmark run."""
    global current_benchmark_process, benchmark_status
    
    if not benchmark_status["running"] or not current_benchmark_process:
        raise HTTPException(status_code=400, detail="No benchmark running")
    
    try:
        current_benchmark_process.terminate()
        current_benchmark_process.wait(timeout=10)
        benchmark_status["running"] = False
        benchmark_status["message"] = "Benchmark stopped by user"
        
        return {"success": True, "message": "Benchmark stopped"}
    except subprocess.TimeoutExpired:
        current_benchmark_process.kill()
        benchmark_status["running"] = False
        benchmark_status["message"] = "Benchmark forcefully terminated"
        return {"success": True, "message": "Benchmark forcefully terminated"}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to stop benchmark: {str(e)}")

@app.get("/api/status")
async def get_status():
    """Get the current benchmark status."""
    return benchmark_status

@app.get("/api/files")
async def list_benchmark_files():
    """List all benchmark files."""
    try:
        files = []
        patterns = [
            BENCHMARK_DIR / "benchmark_*.json",
            BENCHMARK_DIR / "benchmark_report_*.json"
        ]
        
        for pattern in patterns:
            for file_path in glob.glob(str(pattern)):
                stat = os.stat(file_path)
                files.append({                    "name": os.path.basename(file_path),
                    "path": file_path,
                    "size": stat.st_size,
                    "modified": datetime.fromtimestamp(stat.st_mtime).isoformat()
                })
        
        files.sort(key=lambda x: x["modified"], reverse=True)
        return {"files": files, "count": len(files)}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to list files: {str(e)}")

@app.get("/api/runs")
async def get_runs():
    """Get all available runs/sessions for filtering."""
    try:
        benchmarks = load_benchmark_files()
        runs = {}
        for b in benchmarks:
            # Prefer report_id if present, else session_id
            run_id = b.get('report_id') or b.get('session_id')
            if not run_id:
                continue
            if run_id not in runs:
                runs[run_id] = {
                    'run_id': run_id,
                    'scenario': b.get('scenario', 'unknown'),
                    'start_time': b.get('start_time'),
                    'num_clients': b.get('num_clients', 1),
                    'client_ids': [],
                }
            if b.get('client_id') and b['client_id'] not in runs[run_id]['client_ids']:
                runs[run_id]['client_ids'].append(b['client_id'])
        # Return as a sorted list (most recent first)
        run_list = sorted(runs.values(), key=lambda x: x['start_time'] or '', reverse=True)
        return {'runs': run_list, 'count': len(run_list)}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to list runs: {str(e)}")

# Add a root route to serve the main page
@app.get("/")
async def serve_dashboard():
    """Serve the main dashboard page."""
    try:
        with open(DASHBOARD_DIR / "index.html", "r") as f:
            content = f.read()
        return HTMLResponse(content=content)
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="Dashboard page not found")

# Mount static files for CSS, JS, and other assets
app.mount("/static", StaticFiles(directory=DASHBOARD_DIR), name="static")

# Development server
def main():
    parser = argparse.ArgumentParser(description="ZKP-FL Dashboard Server")
    parser.add_argument("--host", default="localhost", help="Host to bind to")
    parser.add_argument("--port", type=int, default=8000, help="Port to bind to")
    parser.add_argument("--reload", action="store_true", help="Enable auto-reload")
    parser.add_argument("--debug", action="store_true", help="Enable debug mode")
    
    args = parser.parse_args()
    
    print(f"Starting ZKP-FL Dashboard Server...")
    print(f"Dashboard will be available at: http://{args.host}:{args.port}")
    print(f"Benchmark directory: {BENCHMARK_DIR.absolute()}")
    print(f"Workspace directory: {WORKSPACE_DIR.absolute()}")
    
    # Ensure directories exist
    BENCHMARK_DIR.mkdir(exist_ok=True)
    
    # Start server
    uvicorn.run(
        "server:app",
        host=args.host,
        port=args.port,
        reload=args.reload,
        log_level="debug" if args.debug else "info"
    )

if __name__ == "__main__":
    main()
