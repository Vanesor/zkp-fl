use chrono::Utc;
use common::types::ProofResult;
use common::{BenchmarkResult, ClientMetrics, Config, Result, ZkpFlError};
use futures::future::try_join_all;
use log::{debug, error, info};
use rand;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use uuid::Uuid;

pub struct MultiClientBenchmark {
    config: Config,
    num_clients: usize,
    rounds: usize,
    max_concurrent: usize,
    client_delay: Duration,
    server_url: String,
}

impl MultiClientBenchmark {
    pub fn new(
        config: Config,
        num_clients: usize,
        rounds: usize,
        max_concurrent: usize,
        client_delay: Duration,
        server_url: String,
    ) -> Self {
        Self {
            config,
            num_clients,
            rounds,
            max_concurrent,
            client_delay,
            server_url,
        }
    }

    pub async fn run_sequential(&self) -> Result<BenchmarkResult> {
        info!("Running sequential multi-client benchmark");

        let start_time = Instant::now();
        let mut all_metrics = Vec::new();
        let mut successful_clients = 0;
        let mut failed_clients = 0;

        for client_id in 0..self.num_clients {
            info!("Starting client {}/{}", client_id + 1, self.num_clients);

            match self.run_single_client(client_id).await {
                Ok(metrics) => {
                    all_metrics.push(metrics);
                    successful_clients += 1;
                    info!("Client {} completed successfully", client_id);
                }
                Err(e) => {
                    error!("Client {} failed: {}", client_id, e);
                    failed_clients += 1;
                }
            }

            // Add delay between clients
            if client_id < self.num_clients - 1 {
                tokio::time::sleep(self.client_delay).await;
            }
        }

        let total_duration = start_time.elapsed();

        Ok(self.create_result(
            "sequential",
            total_duration,
            all_metrics,
            successful_clients,
            failed_clients,
        ))
    }

    pub async fn run_concurrent(&self) -> Result<BenchmarkResult> {
        info!("Running concurrent multi-client benchmark");

        let start_time = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let tasks: Vec<_> = (0..self.num_clients)
            .map(|client_id| {
                let semaphore = semaphore.clone();
                let delay = Duration::from_millis(
                    client_id as u64 * self.client_delay.as_millis() as u64
                        / self.num_clients as u64,
                );
                let benchmark = MultiClientBenchmark::new(
                    self.config.clone(),
                    1, // Single client for this task
                    self.rounds,
                    1, // Single concurrent for individual client
                    self.client_delay,
                    self.server_url.clone(),
                );

                tokio::spawn(async move {
                    // Stagger client starts
                    tokio::time::sleep(delay).await;

                    let _permit = semaphore.acquire().await.unwrap();
                    benchmark.run_single_client(client_id).await
                })
            })
            .collect();

        let results = try_join_all(tasks)
            .await
            .map_err(|e| ZkpFlError::Benchmark(format!("Task join error: {}", e)))?;

        let mut all_metrics = Vec::new();
        let mut successful_clients = 0;
        let mut failed_clients = 0;

        for result in results {
            match result {
                Ok(metrics) => {
                    all_metrics.push(metrics);
                    successful_clients += 1;
                }
                Err(e) => {
                    error!("Client failed: {}", e);
                    failed_clients += 1;
                }
            }
        }

        let total_duration = start_time.elapsed();

        Ok(self.create_result(
            "concurrent",
            total_duration,
            all_metrics,
            successful_clients,
            failed_clients,
        ))
    }

    pub async fn run_stress_test(&self) -> Result<BenchmarkResult> {
        info!("Running stress test with {} clients", self.num_clients);

        let start_time = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        // Create batches of clients
        let batch_size = self.max_concurrent;
        let mut all_metrics = Vec::new();
        let mut successful_clients = 0;
        let mut failed_clients = 0;

        for batch_start in (0..self.num_clients).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(self.num_clients);

            info!("Processing batch {}-{}", batch_start, batch_end - 1);
            let batch_tasks: Vec<_> = (batch_start..batch_end)
                .map(|client_id| {
                    let semaphore = semaphore.clone();
                    let benchmark = MultiClientBenchmark::new(
                        self.config.clone(),
                        1, // Single client for this task
                        self.rounds,
                        1, // Single concurrent for individual client
                        self.client_delay,
                        self.server_url.clone(),
                    );

                    tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.unwrap();
                        benchmark.run_single_client(client_id).await
                    })
                })
                .collect();

            let batch_results = try_join_all(batch_tasks)
                .await
                .map_err(|e| ZkpFlError::Benchmark(format!("Batch task join error: {}", e)))?;

            for result in batch_results {
                match result {
                    Ok(metrics) => {
                        all_metrics.push(metrics);
                        successful_clients += 1;
                    }
                    Err(e) => {
                        error!("Client in batch failed: {}", e);
                        failed_clients += 1;
                    }
                }
            }

            // Brief pause between batches
            if batch_end < self.num_clients {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        let total_duration = start_time.elapsed();

        Ok(self.create_result(
            "stress_test",
            total_duration,
            all_metrics,
            successful_clients,
            failed_clients,
        ))
    }
    async fn run_single_client(&self, client_id: usize) -> Result<ClientMetrics> {
        debug!("Running actual client {}", client_id);

        let start_time = Instant::now();
        let client_name = format!("benchmark_client_{}", client_id);

        // Ensure server URL has proper http:// prefix
        let server_url =
            if self.server_url.starts_with("http://") || self.server_url.starts_with("https://") {
                self.server_url.clone()
            } else {
                format!("http://{}", self.server_url)
            };

        // Prepare client command
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(&["run", "--bin", "client"])
            .arg("--")
            .arg("--config")
            .arg("config.toml")
            .arg("--client-id")
            .arg(&client_name)
            .arg("--epochs")
            .arg("10")
            .arg("--server-url")
            .arg(&server_url)
            .arg("--benchmark")
            .arg("--verbose");

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        debug!(
            "Executing client command for client {}: {:?}",
            client_id, cmd
        );

        // Execute the client
        let output = cmd.output().await.map_err(|e| {
            ZkpFlError::Config(format!("Failed to execute client {}: {}", client_id, e))
        })?;

        let total_time = start_time.elapsed();

        // Try to parse JSON file first, fallback to stdout parsing
        let json_metrics = self.parse_client_json_file(&client_name).await;

        match json_metrics {
            Ok(metrics) => {
                info!("Successfully parsed JSON metrics for client {}", client_id);
                Ok(metrics)
            }
            Err(_) => {
                // Fallback to stdout parsing if JSON file not found
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("Client {} failed. Stderr: {}", client_id, stderr);
                    return Ok(ClientMetrics {
                        client_id: client_name,
                        training_times: vec![],
                        proof_times: vec![],
                        witness_times: vec![],
                        verification_times: vec![],
                        proof_sizes: vec![],
                        success_count: 0,
                        failure_count: 1,
                        avg_training_time: Duration::ZERO,
                        avg_proof_time: Duration::ZERO,
                        avg_witness_time: Duration::ZERO,
                        avg_verification_time: Duration::ZERO,
                        avg_proof_size: 0,
                        total_time,
                    });
                }

                // Parse client output for performance metrics
                let stdout = String::from_utf8_lossy(&output.stdout);
                let performance = self.parse_client_output(&stdout);
                Ok(ClientMetrics {
                    client_id: client_name,
                    training_times: vec![performance.training_time],
                    proof_times: vec![performance.proof_time],
                    witness_times: vec![performance.witness_time],
                    verification_times: vec![performance.verification_time],
                    proof_sizes: vec![performance.proof_size],
                    success_count: 1,
                    failure_count: 0,
                    avg_training_time: performance.training_time,
                    avg_proof_time: performance.proof_time,
                    avg_witness_time: performance.witness_time,
                    avg_verification_time: performance.verification_time,
                    avg_proof_size: performance.proof_size,
                    total_time,
                })
            }
        }
    }

    async fn parse_client_json_file(&self, client_name: &str) -> Result<ClientMetrics> {
        use serde_json::Value;
        use std::fs;
        use std::path::Path;

        // Look for the most recent JSON file for this client
        let benchmark_dir = Path::new("./benchmarks");
        let mut latest_file = None;
        let mut latest_time = std::time::SystemTime::UNIX_EPOCH;

        if benchmark_dir.exists() {
            for entry in fs::read_dir(benchmark_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.contains(client_name)
                        && filename.ends_with(".json")
                        && !filename.contains("_report_")
                    {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if modified > latest_time {
                                    latest_time = modified;
                                    latest_file = Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(json_path) = latest_file {
            let json_content = fs::read_to_string(&json_path).map_err(|e| {
                ZkpFlError::Benchmark(format!("Failed to read JSON file {:?}: {}", json_path, e))
            })?;

            let json_data: Value = serde_json::from_str(&json_content)
                .map_err(|e| ZkpFlError::Benchmark(format!("Failed to parse JSON: {}", e)))?;
            // Extract metrics from JSON
            let zkp_metrics = &json_data["zkp_metrics"];
            let training_metrics = &json_data["training_metrics"];

            let training_time =
                Duration::from_millis(training_metrics["training_time_ms"].as_u64().unwrap_or(0));
            let proof_time = Duration::from_millis(
                zkp_metrics["proof_generation_time_ms"]
                    .as_u64()
                    .unwrap_or(0),
            );
            let witness_time = Duration::from_millis(
                zkp_metrics["witness_generation_time_ms"]
                    .as_u64()
                    .unwrap_or(0),
            );
            let verification_time = Duration::from_millis(
                zkp_metrics["proof_verification_time_ms"]
                    .as_u64()
                    .unwrap_or(0),
            );
            let proof_size = zkp_metrics["proof_size_bytes"].as_u64().unwrap_or(0) as usize;
            let success = json_data["success"].as_bool().unwrap_or(false);

            let total_duration =
                Duration::from_millis(json_data["total_duration_ms"].as_u64().unwrap_or(0));
            Ok(ClientMetrics {
                client_id: client_name.to_string(),
                training_times: vec![training_time],
                proof_times: vec![proof_time],
                witness_times: vec![witness_time],
                verification_times: vec![verification_time],
                proof_sizes: vec![proof_size],
                success_count: if success { 1 } else { 0 },
                failure_count: if success { 0 } else { 1 },
                avg_training_time: training_time,
                avg_proof_time: proof_time,
                avg_witness_time: witness_time,
                avg_verification_time: verification_time,
                avg_proof_size: proof_size,
                total_time: total_duration,
            })
        } else {
            Err(ZkpFlError::Benchmark("JSON file not found".to_string()))
        }
    }
    fn parse_client_output(&self, output: &str) -> ClientPerformance {
        // Extract performance metrics from client logs
        let mut training_time = Duration::from_millis(10);
        let mut proof_time = Duration::from_millis(100);
        let mut witness_time = Duration::from_millis(30); // Default estimated witness time
        let mut verification_time = Duration::from_millis(50);
        let mut proof_size = 1082;

        for line in output.lines() {
            if line.contains("Model training completed in") {
                if let Some(time_str) = line.split("in ").nth(1) {
                    if let Some(ms_str) = time_str.split("ms").next() {
                        if let Ok(ms) = ms_str.parse::<u64>() {
                            training_time = Duration::from_millis(ms);
                        }
                    }
                }
            } else if line.contains("Proof generated in") {
                if let Some(time_str) = line.split("in ").nth(1) {
                    if let Some(ms_str) = time_str.split("ms").next() {
                        if let Ok(ms) = ms_str.parse::<u64>() {
                            proof_time = Duration::from_millis(ms);
                        }
                    }
                }
            } else if line.contains("Witness time:") {
                if let Some(time_str) = line.split("Witness time: ").nth(1) {
                    if let Some(ms_str) = time_str.split("ms").next() {
                        if let Ok(ms) = ms_str.parse::<u64>() {
                            witness_time = Duration::from_millis(ms);
                        }
                    }
                }
            } else if line.contains("Proof verified successfully in") {
                if let Some(time_str) = line.split("in ").nth(1) {
                    if let Some(ms_str) = time_str.split("ms").next() {
                        if let Ok(ms) = ms_str.parse::<u64>() {
                            verification_time = Duration::from_millis(ms);
                        }
                    }
                }
            } else if line.contains("size: ") && line.contains("bytes") {
                if let Some(size_str) = line.split("size: ").nth(1) {
                    if let Some(bytes_str) = size_str.split(" bytes").next() {
                        if let Ok(size) = bytes_str.parse::<usize>() {
                            proof_size = size;
                        }
                    }
                }
            }
        }

        ClientPerformance {
            training_time,
            proof_time,
            witness_time,
            verification_time,
            proof_size,
        }
    }

    async fn simulate_training(
        &self,
        client_id: usize,
        round: usize,
        dataset_size: usize,
    ) -> Result<TrainingResult> {
        // Simulate actual training work
        let work_duration = Duration::from_millis(100 + (dataset_size / 10) as u64);
        tokio::time::sleep(work_duration).await;

        Ok(TrainingResult {
            client_id,
            round,
            model_weights: vec![0.5 + (client_id as f64 * 0.1), 0.3 + (round as f64 * 0.05)],
            loss: 0.1 / (round + 1) as f64,
            dataset_size,
        })
    }
    async fn simulate_proof_generation(
        &self,
        _client_id: usize,
        _round: usize,
        training_result: &TrainingResult,
    ) -> Result<ProofResult> {
        // Simulate proof generation work (this is computationally intensive)
        let work_duration = Duration::from_millis(500 + (training_result.dataset_size / 5) as u64);
        tokio::time::sleep(work_duration).await;

        let proof_size = 1000 + training_result.dataset_size / 10;

        Ok(ProofResult {
            success: true,
            proof_id: Some(Uuid::new_v4().to_string()),
            training_time: Duration::from_millis(100),
            proof_time: work_duration,
            verification_time: Duration::from_millis(50),
            proof_size,
            error: None,
        })
    }

    async fn submit_and_verify_proof(
        &self,
        client_id: usize,
        round: usize,
        proof_result: ProofResult,
    ) -> Result<bool> {
        // Simulate network submission and server verification
        let network_delay = Duration::from_millis(50 + (client_id * 10) as u64);
        let verification_work = Duration::from_millis(200 + (proof_result.proof_size / 100) as u64);

        tokio::time::sleep(network_delay + verification_work).await;

        // Simulate occasional failures
        let success_rate = 0.95;
        let success = rand::random::<f64>() < success_rate;

        if !success {
            return Err(ZkpFlError::Verification(
                "Simulated verification failure".to_string(),
            ));
        }

        debug!(
            "Client {} round {} proof verified successfully",
            client_id, round
        );
        Ok(true)
    }
    fn create_result(
        &self,
        scenario_type: &str,
        total_duration: Duration,
        client_metrics: Vec<ClientMetrics>,
        successful_clients: usize,
        failed_clients: usize,
    ) -> BenchmarkResult {
        let avg_training_time = if !client_metrics.is_empty() {
            let total_training_count = client_metrics
                .iter()
                .map(|m| m.training_times.len())
                .sum::<usize>();
            if total_training_count > 0 {
                client_metrics
                    .iter()
                    .flat_map(|m| &m.training_times)
                    .sum::<Duration>()
                    / total_training_count as u32
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        };

        let avg_proof_time = if !client_metrics.is_empty() {
            let total_proof_count = client_metrics
                .iter()
                .map(|m| m.proof_times.len())
                .sum::<usize>();
            if total_proof_count > 0 {
                client_metrics
                    .iter()
                    .flat_map(|m| &m.proof_times)
                    .sum::<Duration>()
                    / total_proof_count as u32
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        };
        let avg_verification_time = if !client_metrics.is_empty() {
            let total_verification_count = client_metrics
                .iter()
                .map(|m| m.verification_times.len())
                .sum::<usize>();
            if total_verification_count > 0 {
                client_metrics
                    .iter()
                    .flat_map(|m| &m.verification_times)
                    .sum::<Duration>()
                    / total_verification_count as u32
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        };

        let avg_witness_time = if !client_metrics.is_empty() {
            let total_witness_count = client_metrics
                .iter()
                .map(|m| m.witness_times.len())
                .sum::<usize>();
            if total_witness_count > 0 {
                client_metrics
                    .iter()
                    .flat_map(|m| &m.witness_times)
                    .sum::<Duration>()
                    / total_witness_count as u32
            } else {
                Duration::ZERO
            }
        } else {
            Duration::ZERO
        };
        let avg_proof_size = if !client_metrics.is_empty() {
            let total_proof_sizes_count = client_metrics
                .iter()
                .map(|m| m.proof_sizes.len())
                .sum::<usize>();
            if total_proof_sizes_count > 0 {
                client_metrics
                    .iter()
                    .flat_map(|m| &m.proof_sizes)
                    .sum::<usize>()
                    / total_proof_sizes_count
            } else {
                0
            }
        } else {
            0
        }; // Calculate aggregated ZKP and training metrics
        let zkp_metrics = common::ZkpMetrics {
            setup_time_ms: 0, // Not tracked individually
            witness_generation_time_ms: avg_witness_time.as_millis() as u64,
            proof_generation_time_ms: avg_proof_time.as_millis() as u64,
            proof_verification_time_ms: avg_verification_time.as_millis() as u64,
            proof_size_bytes: avg_proof_size,
            circuit_constraints: 0,    // Not tracked in benchmarks
            circuit_advice_columns: 0, // Not tracked in benchmarks
            circuit_fixed_columns: 0,  // Not tracked in benchmarks
            folding_iterations: 0,     // Not tracked in benchmarks
        };

        let training_metrics = common::TrainingMetrics {
            dataset_size: 100, // Default from config
            num_features: 5,   // Default from config
            training_time_ms: avg_training_time.as_millis() as u64,
            epochs_completed: 10, // Default from config
            final_loss: 0.0,      // Not aggregated
            initial_loss: 0.0,    // Not aggregated
            convergence_epoch: None,
            loss_history: vec![],
        };

        let mut result =
            BenchmarkResult::new(Uuid::new_v4(), format!("multi_client_{}", scenario_type));

        // Set required fields
        result.success = successful_clients > 0;
        result.total_duration_ms = total_duration.as_millis() as u64;
        result.zkp_metrics = zkp_metrics;
        result.training_metrics = training_metrics;

        // Set optional fields for backward compatibility
        result.id = Some(Uuid::new_v4());
        result.timestamp = Some(Utc::now());
        result.scenario = Some(scenario_type.to_string());
        result.num_clients = Some(self.num_clients);
        result.num_rounds = Some(self.rounds);
        result.total_duration = Some(total_duration);
        result.successful_clients = Some(successful_clients);
        result.failed_clients = Some(failed_clients);
        result.avg_training_time = Some(avg_training_time);
        result.avg_proof_time = Some(avg_proof_time);
        result.avg_verification_time = Some(avg_verification_time);
        result.avg_proof_size = Some(avg_proof_size);
        result.client_metrics = Some(client_metrics);
        result.throughput = Some(successful_clients as f64 / total_duration.as_secs_f64());
        result.success_rate =
            Some(successful_clients as f64 / (successful_clients + failed_clients) as f64);
        result
    }
}

/// Run sequential benchmark - simplified implementation
pub async fn run_sequential_benchmark(
    config: &Config,
    args: &crate::Args,
    round: usize,
) -> Result<Vec<BenchmarkResult>> {
    info!(
        "Running sequential multi-client benchmark - Round {}",
        round + 1
    );

    let benchmark = MultiClientBenchmark::new(
        config.clone(),
        args.num_clients,
        1, // Single round
        1, // Sequential: max 1 concurrent
        Duration::from_millis(args.client_delay_ms),
        args.server_url
            .clone()
            .unwrap_or_else(|| config.server.host.clone() + ":" + &config.server.port.to_string()),
    );

    let result = benchmark.run_sequential().await?;
    Ok(vec![result])
}

/// Run concurrent benchmark - simplified implementation  
pub async fn run_concurrent_benchmark(
    config: &Config,
    args: &crate::Args,
    round: usize,
) -> Result<Vec<BenchmarkResult>> {
    info!(
        "Running concurrent multi-client benchmark - Round {}",
        round + 1
    );

    let benchmark = MultiClientBenchmark::new(
        config.clone(),
        args.num_clients,
        1, // Single round
        args.max_concurrent,
        Duration::from_millis(args.client_delay_ms),
        args.server_url
            .clone()
            .unwrap_or_else(|| config.server.host.clone() + ":" + &config.server.port.to_string()),
    );

    let result = benchmark.run_concurrent().await?;
    Ok(vec![result])
}

#[derive(Debug)]
struct TrainingResult {
    client_id: usize,
    round: usize,
    model_weights: Vec<f64>,
    loss: f64,
    dataset_size: usize,
}

#[derive(Debug)]
struct ClientPerformance {
    training_time: Duration,
    proof_time: Duration,
    witness_time: Duration,
    verification_time: Duration,
    proof_size: usize,
}
