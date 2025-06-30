mod multi_client;
mod scenarios;
mod single_client;

use chrono::Utc;
use clap::Parser;
use common::{BenchmarkResult, Config, MultiClientBenchmark, Result, ZkpFlError};
use log::{error, info, warn};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Parser, Debug, Clone)]
#[command(name = "zkp-fl-benchmarks")]
#[command(about = "ZKP Federated Learning Benchmarking Tool")]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Benchmark scenario to run
    #[arg(short, long, value_enum)]
    scenario: BenchmarkScenario,

    /// Number of clients to simulate
    #[arg(short, long, default_value = "5")]
    num_clients: usize,

    /// Number of rounds/iterations
    #[arg(short, long, default_value = "3")]
    rounds: usize,

    /// Output directory for results
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Server URL
    #[arg(long)]
    server_url: Option<String>,

    /// Enable detailed logging
    #[arg(short, long)]
    verbose: bool,

    /// Delay between client starts (ms)
    #[arg(long, default_value = "1000")]
    client_delay_ms: u64,

    /// Maximum concurrent clients
    #[arg(long, default_value = "10")]
    max_concurrent: usize,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum BenchmarkScenario {
    /// Single client performance test
    SingleClient,
    /// Multiple clients with sequential execution
    MultiClientSequential,
    /// Multiple clients with concurrent execution
    MultiClientConcurrent,
    /// Stress test with many clients
    StressTest,
    /// Custom scenario
    Custom,
}

pub struct BenchmarkRunner {
    config: Config,
    args: Args,
    results: Vec<BenchmarkResult>,
}

impl BenchmarkRunner {
    pub async fn new(args: Args) -> Result<Self> {
        // Load configuration
        let config = Self::load_config(&args.config)?;

        Ok(Self {
            config,
            args,
            results: Vec::new(),
        })
    }

    fn load_config(path: &str) -> Result<Config> {
        let config_str = std::fs::read_to_string(path)
            .map_err(|e| ZkpFlError::Config(format!("Failed to read config file: {}", e)))?;

        let config: Config = toml::from_str(&config_str)
            .map_err(|e| ZkpFlError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting benchmark scenario: {:?}", self.args.scenario);
        info!(
            "Clients: {}, Rounds: {}",
            self.args.num_clients, self.args.rounds
        );

        // Ensure output directory exists
        let output_dir = self
            .args
            .output_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.config.benchmarks.output_path));
        std::fs::create_dir_all(&output_dir)?;

        // Check server availability
        if let Some(ref server_url) = self.args.server_url {
            self.check_server_availability(server_url).await?;
        }

        // Run the specific benchmark scenario
        match self.args.scenario {
            BenchmarkScenario::SingleClient => {
                self.run_single_client_benchmark().await?;
            }
            BenchmarkScenario::MultiClientSequential => {
                self.run_multi_client_sequential().await?;
            }
            BenchmarkScenario::MultiClientConcurrent => {
                self.run_multi_client_concurrent().await?;
            }
            BenchmarkScenario::StressTest => {
                self.run_stress_test().await?;
            }
            BenchmarkScenario::Custom => {
                self.run_custom_scenario().await?;
            }
        }

        // Generate and save final report
        self.generate_final_report(&output_dir).await?;

        info!("Benchmark completed successfully");
        Ok(())
    }

    async fn check_server_availability(&self, server_url: &str) -> Result<()> {
        info!("Checking server availability: {}", server_url);

        let client = reqwest::Client::new();
        let health_url = format!("{}/api/health", server_url);

        match client.get(&health_url).send().await {
            Ok(response) if response.status().is_success() => {
                info!("Server is available and healthy");
                Ok(())
            }
            Ok(response) => {
                warn!("Server responded with status: {}", response.status());
                Err(ZkpFlError::Network(format!(
                    "Server health check failed: {}",
                    response.status()
                )))
            }
            Err(e) => {
                error!("Failed to connect to server: {}", e);
                Err(ZkpFlError::Network(format!(
                    "Server connection failed: {}",
                    e
                )))
            }
        }
    }

    async fn run_single_client_benchmark(&mut self) -> Result<()> {
        info!("Running single client benchmark");

        for round in 0..self.args.rounds {
            info!("Round {}/{}", round + 1, self.args.rounds);

            let result =
                single_client::run_single_client_benchmark(&self.config, &self.args, round).await?;

            self.results.push(result);

            info!("Round {} completed", round + 1);

            // Small delay between rounds
            if round < self.args.rounds - 1 {
                tokio::time::sleep(Duration::from_millis(2000)).await;
            }
        }

        Ok(())
    }

    async fn run_multi_client_sequential(&mut self) -> Result<()> {
        info!("Running multi-client sequential benchmark");

        for round in 0..self.args.rounds {
            info!("Round {}/{}", round + 1, self.args.rounds);

            let results =
                multi_client::run_sequential_benchmark(&self.config, &self.args, round).await?;

            self.results.extend(results);

            info!(
                "Round {} completed with {} clients",
                round + 1,
                self.args.num_clients
            );
        }

        Ok(())
    }

    async fn run_multi_client_concurrent(&mut self) -> Result<()> {
        info!("Running multi-client concurrent benchmark");

        for round in 0..self.args.rounds {
            info!("Round {}/{}", round + 1, self.args.rounds);

            let results =
                multi_client::run_concurrent_benchmark(&self.config, &self.args, round).await?;

            self.results.extend(results);

            info!(
                "Round {} completed with {} clients",
                round + 1,
                self.args.num_clients
            );
        }

        Ok(())
    }

    async fn run_stress_test(&mut self) -> Result<()> {
        info!("Running stress test");

        // Gradually increase the number of clients
        let stress_levels = vec![5, 10, 20, 50];

        for level in stress_levels {
            if level > self.args.num_clients {
                continue;
            }

            info!("Stress test level: {} clients", level);

            let mut args = self.args.clone();
            args.num_clients = level;
            args.rounds = 1; // Single round per stress level

            let results = multi_client::run_concurrent_benchmark(&self.config, &args, 0).await?;

            self.results.extend(results);

            // Longer delay between stress levels
            tokio::time::sleep(Duration::from_millis(5000)).await;
        }

        Ok(())
    }

    async fn run_custom_scenario(&mut self) -> Result<()> {
        info!("Running custom scenario");

        // Load custom scenario configuration
        let scenario_results = scenarios::run_custom_scenarios(&self.config, &self.args).await?;

        self.results.extend(scenario_results);

        Ok(())
    }

    async fn generate_final_report(&self, output_dir: &PathBuf) -> Result<()> {
        info!("Generating final benchmark report");

        // Create aggregate benchmark data
        let aggregate_benchmark = MultiClientBenchmark {
            benchmark_id: Uuid::new_v4(),
            start_time: self
                .results
                .first()
                .map(|r| r.start_time)
                .unwrap_or_else(Utc::now),
            end_time: self
                .results
                .last()
                .map(|r| r.end_time)
                .unwrap_or_else(Utc::now),
            num_clients: self.args.num_clients,
            client_results: self.results.clone(),
            aggregate_metrics: self.calculate_aggregate_metrics(),
        };

        // Save detailed JSON report
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let report_file = output_dir.join(format!("benchmark_report_{}.json", timestamp));

        let json_report = serde_json::to_string_pretty(&aggregate_benchmark)?;
        std::fs::write(&report_file, json_report)?;

        // Save summary report
        let summary_file = output_dir.join(format!("benchmark_summary_{}.txt", timestamp));
        let summary = self.generate_summary_report(&aggregate_benchmark);
        std::fs::write(&summary_file, summary)?;

        info!("Reports saved:");
        info!("  Detailed: {:?}", report_file);
        info!("  Summary: {:?}", summary_file);

        // Print summary to console
        self.print_summary(&aggregate_benchmark);

        Ok(())
    }
    fn calculate_aggregate_metrics(&self) -> common::AggregateMetrics {
        if self.results.is_empty() {
            return common::AggregateMetrics {
                avg_proof_generation_time_ms: 0.0,
                min_proof_generation_time_ms: 0,
                max_proof_generation_time_ms: 0,
                avg_proof_verification_time_ms: 0.0,
                avg_training_time_ms: 0.0,
                total_proofs_generated: 0,
                total_proofs_verified: 0,
                success_rate: 0.0,
                throughput_proofs_per_second: 0.0,
            };
        } // For multi-client benchmarks, use the data from the aggregated result
        let first_result = &self.results[0];

        let successful_results = if first_result.success {
            first_result.successful_clients.unwrap_or(1)
        } else {
            0
        };

        let total_clients =
            first_result.successful_clients.unwrap_or(1) + first_result.failed_clients.unwrap_or(0);

        // Calculate throughput using the total duration from the multi-client benchmark
        let total_duration_ms = first_result.total_duration_ms;
        let throughput = if total_duration_ms > 0 {
            (total_clients as f64) / (total_duration_ms as f64 / 1000.0) // Convert ms to seconds
        } else {
            0.0
        };

        common::AggregateMetrics {
            avg_proof_generation_time_ms: first_result.zkp_metrics.proof_generation_time_ms as f64,
            min_proof_generation_time_ms: first_result.zkp_metrics.proof_generation_time_ms,
            max_proof_generation_time_ms: first_result.zkp_metrics.proof_generation_time_ms,
            avg_proof_verification_time_ms: first_result.zkp_metrics.proof_verification_time_ms
                as f64,
            avg_training_time_ms: first_result.training_metrics.training_time_ms as f64,
            total_proofs_generated: total_clients,
            total_proofs_verified: successful_results,
            success_rate: successful_results as f64 / total_clients as f64,
            throughput_proofs_per_second: throughput,
        }
    }

    fn generate_summary_report(&self, benchmark: &MultiClientBenchmark) -> String {
        format!(
            r#"
ZKP-FL Benchmark Summary Report
===============================

Benchmark ID: {}
Scenario: {:?}
Date: {}
Duration: {:.2} seconds

Test Configuration:
- Number of clients: {}
- Number of rounds: {}
- Total executions: {}

Performance Metrics:
- Average proof generation time: {:.2} ms
- Min proof generation time: {} ms
- Max proof generation time: {} ms
- Average verification time: {:.2} ms
- Average training time: {:.2} ms
- Success rate: {:.1}%
- Throughput: {:.2} proofs/second

Results:
- Total proofs generated: {}
- Total proofs verified: {}
- Failed executions: {}

Time Breakdown:
- Total benchmark time: {:.2} seconds
- Average per client: {:.2} seconds

System Performance:
- Memory usage: Varied
- CPU usage: Varied
- Network latency: Varied

Notes:
- All times are in milliseconds unless specified
- Success rate is based on successful proof verification
- Throughput is calculated as total proofs / total time
"#,
            benchmark.benchmark_id,
            self.args.scenario,
            benchmark.start_time.format("%Y-%m-%d %H:%M:%S UTC"),
            (benchmark.end_time - benchmark.start_time).num_seconds() as f64,
            benchmark.num_clients,
            self.args.rounds,
            benchmark.client_results.len(),
            benchmark.aggregate_metrics.avg_proof_generation_time_ms,
            benchmark.aggregate_metrics.min_proof_generation_time_ms,
            benchmark.aggregate_metrics.max_proof_generation_time_ms,
            benchmark.aggregate_metrics.avg_proof_verification_time_ms,
            benchmark.aggregate_metrics.avg_training_time_ms,
            benchmark.aggregate_metrics.success_rate * 100.0,
            benchmark.aggregate_metrics.throughput_proofs_per_second,
            benchmark.aggregate_metrics.total_proofs_generated,
            benchmark.aggregate_metrics.total_proofs_verified,
            benchmark.aggregate_metrics.total_proofs_generated
                - benchmark.aggregate_metrics.total_proofs_verified,
            (benchmark.end_time - benchmark.start_time).num_seconds() as f64,
            (benchmark.end_time - benchmark.start_time).num_seconds() as f64
                / benchmark.num_clients as f64,
        )
    }

    fn print_summary(&self, benchmark: &MultiClientBenchmark) {
        println!("\n=== Benchmark Results Summary ===");
        println!("Scenario: {:?}", self.args.scenario);
        println!(
            "Clients: {}, Rounds: {}",
            self.args.num_clients, self.args.rounds
        );
        println!("Total executions: {}", benchmark.client_results.len());
        println!(
            "Success rate: {:.1}%",
            benchmark.aggregate_metrics.success_rate * 100.0
        );
        println!(
            "Average proof generation: {:.2} ms",
            benchmark.aggregate_metrics.avg_proof_generation_time_ms
        );
        println!(
            "Average verification: {:.2} ms",
            benchmark.aggregate_metrics.avg_proof_verification_time_ms
        );
        println!(
            "Throughput: {:.2} proofs/second",
            benchmark.aggregate_metrics.throughput_proofs_per_second
        );
        println!("==================================\n");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("ZKP-FL Benchmarking Tool starting...");

    let mut runner = BenchmarkRunner::new(args).await?;

    match runner.run().await {
        Ok(()) => {
            info!("Benchmarking completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Benchmarking failed: {}", e);
            Err(e)
        }
    }
}
