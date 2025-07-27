#!/usr/bin/env python3

import json
import os
import glob
import matplotlib.pyplot as plt
import numpy as np
from datetime import datetime
import sys
import argparse

def load_benchmark_files(directory='../benchmarks'):
    """Load benchmark JSON report files from the specified directory."""
    pattern = os.path.join(directory, 'benchmark_report_*.json')
    files = glob.glob(pattern)
    files.sort(key=os.path.getmtime, reverse=True)
    
    if not files:
        print(f"No benchmark report files found in {directory}")
        return []
        
    reports = []
    for file in files:
        try:
            with open(file, 'r') as f:
                data = json.load(f)
                data['_filename'] = os.path.basename(file)
                reports.append(data)
            print(f"Loaded {file}")
        except Exception as e:
            print(f"Error loading {file}: {e}")
    
    return reports

def plot_performance_overview(reports):
    """Generate performance overview chart."""
    if not reports:
        print("No benchmark reports available for visualization")
        return
        
    fig, ax = plt.subplots(figsize=(12, 8))
    
    scenarios = []
    proof_gen_times = []
    verify_times = []
    training_times = []
    
    for report in reports:
        scenario_name = report.get('scenario', 'Unknown')
        num_clients = report.get('num_clients', 0)
        scenarios.append(f"{scenario_name}\n({num_clients} clients)")
        
        metrics = report.get('metrics', {})
        proof_gen_times.append(metrics.get('avg_proof_time', 0))
        verify_times.append(metrics.get('avg_verification_time', 0))
        training_times.append(metrics.get('avg_training_time', 0))
    
    bar_width = 0.25
    index = np.arange(len(scenarios))
    
    ax.bar(index - bar_width, training_times, bar_width, label='Training Time (ms)', color='#4CAF50')
    ax.bar(index, proof_gen_times, bar_width, label='Proof Generation Time (ms)', color='#2196F3')
    ax.bar(index + bar_width, verify_times, bar_width, label='Verification Time (ms)', color='#FF9800')
    
    ax.set_xlabel('Benchmark Scenario')
    ax.set_ylabel('Time (ms)')
    ax.set_title('ZKP-FL Performance Metrics by Scenario')
    ax.set_xticks(index)
    ax.set_xticklabels(scenarios)
    ax.legend()
    
    plt.tight_layout()
    plt.savefig('../benchmarks/performance_overview.png')
    print("Generated performance_overview.png")
    return fig
    
def plot_scalability_analysis(reports):
    """Generate scalability analysis chart."""
    # Filter reports to get only multi-client scenarios with different client counts
    multi_client_reports = [r for r in reports if 'multi-client' in r.get('scenario', '').lower()]
    
    if not multi_client_reports:
        print("No multi-client reports available for scalability analysis")
        return
    
    # Sort by number of clients
    multi_client_reports.sort(key=lambda x: x.get('num_clients', 0))
    
    fig, ax = plt.subplots(figsize=(12, 8))
    
    clients = []
    proof_gen_times = []
    verify_times = []
    throughputs = []
    
    for report in multi_client_reports:
        num_clients = report.get('num_clients', 0)
        clients.append(num_clients)
        
        metrics = report.get('metrics', {})
        proof_gen_times.append(metrics.get('avg_proof_time', 0))
        verify_times.append(metrics.get('avg_verification_time', 0))
        throughputs.append(metrics.get('throughput', 0))
    
    ax.plot(clients, proof_gen_times, 'o-', label='Proof Generation Time (ms)', linewidth=2, color='#2196F3')
    ax.plot(clients, verify_times, 's-', label='Verification Time (ms)', linewidth=2, color='#FF9800')
    
    ax.set_xlabel('Number of Clients')
    ax.set_ylabel('Time (ms)')
    ax.set_title('ZKP-FL Scalability Analysis')
    ax.grid(True, linestyle='--', alpha=0.7)
    ax.legend(loc='upper left')
    
    # Add throughput on secondary y-axis
    ax2 = ax.twinx()
    ax2.plot(clients, throughputs, 'D-', label='Throughput (proofs/s)', linewidth=2, color='#4CAF50')
    ax2.set_ylabel('Throughput (proofs/second)')
    ax2.legend(loc='upper right')
    
    plt.tight_layout()
    plt.savefig('../benchmarks/scalability_analysis.png')
    print("Generated scalability_analysis.png")
    return fig

def generate_dashboard_html(reports):
    """Generate a simple HTML dashboard for the benchmark results."""
    html = """
    <!DOCTYPE html>
    <html>
    <head>
        <title>ZKP-FL Benchmark Results</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 20px; }
            h1 { color: #2196F3; }
            .report-card { border: 1px solid #ddd; padding: 15px; margin: 15px 0; border-radius: 5px; }
            .metric { display: inline-block; margin-right: 20px; text-align: center; }
            .metric-value { font-size: 24px; font-weight: bold; }
            .metric-label { font-size: 12px; color: #666; }
            .success { color: #4CAF50; }
            .warning { color: #FF9800; }
            .error { color: #F44336; }
            .chart-container { margin-top: 30px; }
        </style>
    </head>
    <body>
        <h1>ZKP-FL Benchmark Results</h1>
    """
    
    for report in reports:
        scenario = report.get('scenario', 'Unknown')
        num_clients = report.get('num_clients', 0)
        num_rounds = report.get('num_rounds', 0)
        metrics = report.get('metrics', {})
        
        success_rate = metrics.get('success_rate', 0) * 100
        success_class = "success" if success_rate > 95 else "warning" if success_rate > 80 else "error"
        
        html += f"""
        <div class="report-card">
            <h2>{scenario} Benchmark</h2>
            <p>Clients: {num_clients}, Rounds: {num_rounds}, Timestamp: {report.get('timestamp', 'Unknown')}</p>
            
            <div class="metrics">
                <div class="metric">
                    <div class="metric-value">{metrics.get('avg_proof_time', 0):.2f} ms</div>
                    <div class="metric-label">Avg Proof Time</div>
                </div>
                <div class="metric">
                    <div class="metric-value">{metrics.get('avg_verification_time', 0):.2f} ms</div>
                    <div class="metric-label">Avg Verification Time</div>
                </div>
                <div class="metric">
                    <div class="metric-value">{metrics.get('avg_training_time', 0):.2f} ms</div>
                    <div class="metric-label">Avg Training Time</div>
                </div>
                <div class="metric">
                    <div class="metric-value {success_class}">{success_rate:.1f}%</div>
                    <div class="metric-label">Success Rate</div>
                </div>
                <div class="metric">
                    <div class="metric-value">{metrics.get('throughput', 0):.2f}</div>
                    <div class="metric-label">Throughput (proofs/s)</div>
                </div>
            </div>
        </div>
        """
    
    html += """
        <div class="chart-container">
            <h2>Performance Overview</h2>
            <img src="performance_overview.png" alt="Performance Overview" style="width: 100%; max-width: 1000px;">
        </div>
        
        <div class="chart-container">
            <h2>Scalability Analysis</h2>
            <img src="scalability_analysis.png" alt="Scalability Analysis" style="width: 100%; max-width: 1000px;">
        </div>
    </body>
    </html>
    """
    
    with open('../benchmarks/dashboard.html', 'w') as f:
        f.write(html)
    
    print("Generated dashboard.html")

def main():
    parser = argparse.ArgumentParser(description='ZKP-FL Benchmark Visualizer')
    parser.add_argument('--benchmark_dir', default='../benchmarks', help='Directory containing benchmark files')
    args = parser.parse_args()
    
    reports = load_benchmark_files(args.benchmark_dir)
    
    if not reports:
        print("No benchmark reports found")
        return 1
    
    plot_performance_overview(reports)
    plot_scalability_analysis(reports)
    generate_dashboard_html(reports)
    
    print("Visualization completed successfully!")
    return 0

if __name__ == "__main__":
    sys.exit(main())