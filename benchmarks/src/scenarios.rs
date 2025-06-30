use common::{Config, BenchmarkResult, Result, ZkpFlError};
use crate::multi_client::MultiClientBenchmark;
use crate::single_client::SingleClientBenchmark;
use log::{info, warn};
use std::time::Duration;

pub struct ScenarioRunner {
    config: Config,
    server_url: String,
}

impl ScenarioRunner {
    pub fn new(config: Config, server_url: String) -> Self {
        Self { config, server_url }
    }

    pub async fn run_single_client(&self, rounds: usize) -> Result<BenchmarkResult> {
        info!("Running single client scenario with {} rounds", rounds);
        
        let benchmark = SingleClientBenchmark::new(
            self.config.clone(),
            self.server_url.clone(),
        );
        
        benchmark.run(rounds).await
    }

    pub async fn run_multi_client_sequential(
        &self,
        num_clients: usize,
        rounds: usize,
        client_delay_ms: u64,
    ) -> Result<BenchmarkResult> {
        info!("Running multi-client sequential scenario: {} clients, {} rounds", 
               num_clients, rounds);
        
        let benchmark = MultiClientBenchmark::new(
            self.config.clone(),
            num_clients,
            rounds,
            1, // Sequential: max 1 concurrent
            Duration::from_millis(client_delay_ms),
            self.server_url.clone(),
        );
        
        benchmark.run_sequential().await
    }

    pub async fn run_multi_client_concurrent(
        &self,
        num_clients: usize,
        rounds: usize,
        max_concurrent: usize,
        client_delay_ms: u64,
    ) -> Result<BenchmarkResult> {
        info!("Running multi-client concurrent scenario: {} clients, {} rounds, {} max concurrent", 
               num_clients, rounds, max_concurrent);
        
        let benchmark = MultiClientBenchmark::new(
            self.config.clone(),
            num_clients,
            rounds,
            max_concurrent,
            Duration::from_millis(client_delay_ms),
            self.server_url.clone(),
        );
        
        benchmark.run_concurrent().await
    }

    pub async fn run_stress_test(
        &self,
        num_clients: usize,
        rounds: usize,
        max_concurrent: usize,
    ) -> Result<BenchmarkResult> {
        info!("Running stress test scenario: {} clients, {} rounds, {} max concurrent", 
               num_clients, rounds, max_concurrent);
        
        if num_clients > 100 {
            warn!("Stress test with {} clients may be resource intensive", num_clients);
        }
        
        let benchmark = MultiClientBenchmark::new(
            self.config.clone(),
            num_clients,
            rounds,
            max_concurrent,
            Duration::from_millis(100), // Faster client starts for stress test
            self.server_url.clone(),
        );
        
        benchmark.run_stress_test().await
    }

    pub async fn run_custom_scenario(
        &self,
        scenario_config: CustomScenarioConfig,
    ) -> Result<BenchmarkResult> {
        info!("Running custom scenario: {:?}", scenario_config);
        
        match scenario_config.execution_type {
            ExecutionType::Sequential => {
                self.run_multi_client_sequential(
                    scenario_config.num_clients,
                    scenario_config.rounds,
                    scenario_config.client_delay_ms,
                ).await
            }
            ExecutionType::Concurrent => {
                self.run_multi_client_concurrent(
                    scenario_config.num_clients,
                    scenario_config.rounds,
                    scenario_config.max_concurrent,
                    scenario_config.client_delay_ms,
                ).await
            }
            ExecutionType::Batch => {
                self.run_batch_scenario(scenario_config).await
            }
            ExecutionType::Adaptive => {
                self.run_adaptive_scenario(scenario_config).await
            }
        }
    }

    async fn run_batch_scenario(&self, config: CustomScenarioConfig) -> Result<BenchmarkResult> {
        info!("Running batch scenario with {} batches", config.batch_size.unwrap_or(10));
        
        let batch_size = config.batch_size.unwrap_or(10);
        let num_batches = (config.num_clients + batch_size - 1) / batch_size;
        
        let mut all_results = Vec::new();
        let start_time = std::time::Instant::now();
        
        for batch_id in 0..num_batches {
            let batch_start = batch_id * batch_size;
            let batch_end = ((batch_id + 1) * batch_size).min(config.num_clients);
            let batch_clients = batch_end - batch_start;
            
            info!("Processing batch {} ({} clients)", batch_id, batch_clients);
            
            let batch_benchmark = MultiClientBenchmark::new(
                self.config.clone(),
                batch_clients,
                config.rounds,
                config.max_concurrent,
                Duration::from_millis(config.client_delay_ms),
                self.server_url.clone(),
            );
            
            let batch_result = batch_benchmark.run_concurrent().await?;
            all_results.push(batch_result);
            
            // Delay between batches if specified
            if let Some(batch_delay) = config.batch_delay_ms {
                tokio::time::sleep(Duration::from_millis(batch_delay)).await;
            }
        }
        
        // Combine results from all batches
        self.combine_batch_results(all_results, start_time.elapsed())
    }

    async fn run_adaptive_scenario(&self, config: CustomScenarioConfig) -> Result<BenchmarkResult> {
        info!("Running adaptive scenario");
        
        // Start with a small number of clients and gradually increase
        let mut current_clients = config.min_clients.unwrap_or(1);
        let max_clients = config.num_clients;
        let step_size = config.step_size.unwrap_or(2);
        
        let mut all_results = Vec::new();
        let start_time = std::time::Instant::now();
        
        while current_clients <= max_clients {
            info!("Running adaptive step with {} clients", current_clients);
            
            let benchmark = MultiClientBenchmark::new(
                self.config.clone(),
                current_clients,
                config.rounds,
                config.max_concurrent,
                Duration::from_millis(config.client_delay_ms),
                self.server_url.clone(),
            );
            
            let result = benchmark.run_concurrent().await?;
            all_results.push(result);
              // Check if we should continue based on success rate
            let last_result = all_results.last().unwrap();
            if let Some(success_rate) = last_result.success_rate {
                if success_rate < 0.8 {
                    warn!("Success rate dropped to {:.2}%, stopping adaptive scenario", 
                          success_rate * 100.0);
                    break;
                }
            }
            
            current_clients = (current_clients + step_size).min(max_clients);
            
            // Brief pause between steps
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        
        // Combine results from all steps
        self.combine_batch_results(all_results, start_time.elapsed())
    }    fn combine_batch_results(
        &self,
        results: Vec<BenchmarkResult>,
        total_duration: Duration,
    ) -> Result<BenchmarkResult> {
        if results.is_empty() {
            return Err(ZkpFlError::Benchmark("No batch results to combine".to_string()));
        }
        
        let total_clients: usize = results.iter().filter_map(|r| r.num_clients).sum();
        let total_successful: usize = results.iter().filter_map(|r| r.successful_clients).sum();
        let total_failed: usize = results.iter().filter_map(|r| r.failed_clients).sum();
          let avg_training_time = Duration::from_nanos(
            (results.iter().filter_map(|r| r.avg_training_time).map(|d| d.as_nanos()).sum::<u128>() / results.len() as u128) as u64
        );
        
        let avg_proof_time = Duration::from_nanos(
            (results.iter().filter_map(|r| r.avg_proof_time).map(|d| d.as_nanos()).sum::<u128>() / results.len() as u128) as u64
        );
        
        let avg_verification_time = Duration::from_nanos(
            (results.iter().filter_map(|r| r.avg_verification_time).map(|d| d.as_nanos()).sum::<u128>() / results.len() as u128) as u64
        );
        
        let avg_proof_size: usize = results.iter().filter_map(|r| r.avg_proof_size).sum::<usize>() / results.len();
        
        let combined_client_metrics: Vec<_> = results.into_iter()
            .filter_map(|r| r.client_metrics)
            .flatten()
            .collect();
        
        let mut result = BenchmarkResult::new(
            uuid::Uuid::new_v4(),
            "combined_batch".to_string(),
        );
        
        // Set required fields
        result.success = total_successful > 0;
        result.total_duration_ms = total_duration.as_millis() as u64;
        
        // Set optional fields
        result.id = Some(uuid::Uuid::new_v4());
        result.timestamp = Some(chrono::Utc::now());
        result.scenario = Some("combined_batch".to_string());
        result.num_clients = Some(total_clients);
        result.num_rounds = Some(combined_client_metrics.first().map(|m| m.training_times.len()).unwrap_or(0));
        result.total_duration = Some(total_duration);
        result.successful_clients = Some(total_successful);
        result.failed_clients = Some(total_failed);
        result.avg_training_time = Some(avg_training_time);
        result.avg_proof_time = Some(avg_proof_time);
        result.avg_verification_time = Some(avg_verification_time);
        result.avg_proof_size = Some(avg_proof_size);
        result.client_metrics = Some(combined_client_metrics);
        result.throughput = Some(total_successful as f64 / total_duration.as_secs_f64());
        result.success_rate = Some(total_successful as f64 / (total_successful + total_failed) as f64);
          Ok(result)
    }
}

/// Run custom scenarios - simplified implementation
pub async fn run_custom_scenarios(
    config: &Config,
    args: &crate::Args,
) -> Result<Vec<BenchmarkResult>> {
    info!("Running custom scenarios");
    
    let runner = ScenarioRunner::new(
        config.clone(),
        args.server_url.clone().unwrap_or_else(|| format!("http://{}:{}", config.server.host, config.server.port)),
    );
    
    // Run a simple single client scenario as a demonstration
    let result = runner.run_single_client(args.rounds).await?;
    Ok(vec![result])
}

#[derive(Debug, Clone)]
pub struct CustomScenarioConfig {
    pub num_clients: usize,
    pub rounds: usize,
    pub max_concurrent: usize,
    pub client_delay_ms: u64,
    pub execution_type: ExecutionType,
    pub batch_size: Option<usize>,
    pub batch_delay_ms: Option<u64>,
    pub min_clients: Option<usize>,
    pub step_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum ExecutionType {
    Sequential,
    Concurrent,
    Batch,
    Adaptive,
}
