use crate::Args;
use common::{BenchmarkResult, Config, Result, ZkpFlError};
use log::{debug, info};
use std::process::Stdio;
use tokio::process::Command;
use uuid::Uuid;

pub async fn run_single_client_benchmark(
    _config: &Config,
    args: &Args,
    round: usize,
) -> Result<BenchmarkResult> {
    info!("Running single client benchmark - Round {}", round + 1);

    let session_id = Uuid::new_v4();
    let client_id = format!("benchmark_client_single_{}", round);

    let mut benchmark_result = BenchmarkResult::new(session_id, client_id.clone());

    // Prepare client command
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--bin", "client"])
        .arg("--")
        .arg("--config")
        .arg(&args.config)
        .arg("--client-id")
        .arg(&client_id)
        .arg("--epochs")
        .arg("10")
        .arg("--dataset-path")
        .arg("synthetic:100,5")
        .arg("--benchmark");

    if let Some(ref server_url) = args.server_url {
        cmd.arg("--server-url").arg(server_url);
    }

    if args.verbose {
        cmd.arg("--verbose");
    } // The benchmark should be run from the workspace root, no need to change directory
      // cmd.current_dir("../../");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    debug!("Executing client command: {:?}", cmd);

    // Execute the client
    let start_time = std::time::Instant::now();
    let output = cmd
        .output()
        .await
        .map_err(|e| ZkpFlError::Config(format!("Failed to execute client: {}", e)))?;

    let execution_time = start_time.elapsed();

    // Parse the output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if args.verbose {
        debug!("Client stdout: {}", stdout);
        if !stderr.is_empty() {
            debug!("Client stderr: {}", stderr);
        }
    }
    // Determine success based on exit code and output
    let success =
        output.status.success() && !stdout.contains("ERROR") && !stdout.contains("FAILED");
    // Try to parse metrics from client output (prefer stdout, fallback to stderr)
    let output_to_parse = if !stdout.trim().is_empty() {
        &stdout
    } else {
        &stderr
    };
    let (training_time, proof_time, verification_time, proof_size) =
        parse_client_metrics(output_to_parse);

    // Update benchmark result
    benchmark_result.zkp_metrics.proof_generation_time_ms = proof_time;
    benchmark_result.zkp_metrics.proof_verification_time_ms = verification_time;
    benchmark_result.zkp_metrics.proof_size_bytes = proof_size as usize;
    benchmark_result.training_metrics.training_time_ms = training_time;
    benchmark_result.training_metrics.epochs_completed = 10;
    benchmark_result.training_metrics.dataset_size = 100;
    benchmark_result.training_metrics.num_features = 5;

    let error_message = if success {
        None
    } else {
        Some(format!(
            "Client execution failed. Exit code: {:?}. Stderr: {}",
            output.status.code(),
            stderr
        ))
    };

    benchmark_result.finish(success, error_message);

    info!(
        "Single client benchmark round {} completed: success={}, time={}ms",
        round + 1,
        success,
        execution_time.as_millis()
    );

    Ok(benchmark_result)
}

fn parse_client_metrics(output: &str) -> (u64, u64, u64, u64) {
    let mut training_time = 0u64;
    let mut proof_time = 0u64;
    let mut verification_time = 0u64;
    let mut proof_size = 0u64;

    for line in output.lines() {
        // Look for training completion - client outputs "Model training completed"
        if line.contains("Model training completed")
            || line.contains("training completed")
            || line.contains("Training completed")
        {
            if let Some(time_str) = extract_time_from_line(line) {
                training_time = time_str;
            }
        }
        // Look for proof generation - client outputs "Proof generated"
        else if line.contains("proof generated") || line.contains("Proof generated") {
            if let Some(time_str) = extract_time_from_line(line) {
                proof_time = time_str;
            }
            // Also extract proof size from the same line
            if let Some(size_str) = extract_proof_size_from_line(line) {
                proof_size = size_str;
            }
        }
        // Look for proof verification - client outputs "Proof verified successfully"
        else if line.contains("proof verified") || line.contains("Proof verified") {
            if let Some(time_str) = extract_time_from_line(line) {
                verification_time = time_str;
            }
        }
    }

    (training_time, proof_time, verification_time, proof_size)
}

fn extract_time_from_line(line: &str) -> Option<u64> {
    // Look for patterns like "in 1234ms" or "time: 1234ms"
    let patterns = [" in ", "time: ", "took "];

    for pattern in &patterns {
        if let Some(pos) = line.find(pattern) {
            let time_part = &line[pos + pattern.len()..];
            if let Some(ms_pos) = time_part.find("ms") {
                let number_part = &time_part[..ms_pos];
                if let Ok(time) = number_part.trim().parse::<u64>() {
                    return Some(time);
                }
            }
        }
    }

    None
}

fn extract_proof_size_from_line(line: &str) -> Option<u64> {
    // Look for patterns like "size: 1082 bytes"
    if let Some(size_pos) = line.find("size: ") {
        let size_part = &line[size_pos + 6..]; // "size: ".len() = 6
        if let Some(bytes_pos) = size_part.find(" bytes") {
            let number_part = &size_part[..bytes_pos];
            if let Ok(size) = number_part.trim().parse::<u64>() {
                return Some(size);
            }
        }
    }

    None
}

/// Single client benchmark - simplified implementation
pub struct SingleClientBenchmark {
    pub config: Config,
    pub server_url: String,
}

impl SingleClientBenchmark {
    pub fn new(config: Config, server_url: String) -> Self {
        Self { config, server_url }
    }

    pub async fn run(&self, rounds: usize) -> Result<BenchmarkResult> {
        info!("Running single client benchmark with {} rounds", rounds);

        // Use the existing run_single_client_benchmark function
        let mut results = Vec::new();
        for round in 0..rounds {
            let args = crate::Args {
                config: "config.toml".to_string(),
                scenario: crate::BenchmarkScenario::SingleClient,
                num_clients: 1,
                rounds,
                output_dir: None,
                server_url: Some(self.server_url.clone()),
                verbose: true,
                client_delay_ms: 1000,
                max_concurrent: 1,
            };

            let result = run_single_client_benchmark(&self.config, &args, round).await?;
            results.push(result);
        }

        // Return the first result as the main benchmark result
        Ok(results.into_iter().next().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_client_metrics() {
        let output = r#"
        INFO: Model training completed in 1500ms, final loss: 0.123456
        INFO: Proof generated in 2500ms, size: 1024 bytes
        INFO: Proof verified successfully in 150ms
        "#;

        let (training, proof, verification, proof_size) = parse_client_metrics(output);
        assert_eq!(training, 1500);
        assert_eq!(proof, 2500);
        assert_eq!(verification, 150);
        assert_eq!(proof_size, 1024);
    }

    #[test]
    fn test_extract_time_from_line() {
        assert_eq!(
            extract_time_from_line("Training completed in 1234ms"),
            Some(1234)
        );
        assert_eq!(
            extract_time_from_line("Verification time: 567ms"),
            Some(567)
        );
        assert_eq!(
            extract_time_from_line("Process took 890ms to complete"),
            Some(890)
        );
        assert_eq!(extract_time_from_line("No time information here"), None);
    }
}
