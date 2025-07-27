mod network;
mod prover;
mod trainer;

use chrono::Utc;
use clap::Parser;
use common::{
    BenchmarkResult, Config, HealthcareDataset, OperationMetrics, Result, Session, SessionStatus,
    ZkpFlError,
};
use log::{error, info, warn};
use std::time::Instant;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "zkp-fl-client")]
#[command(about = "ZKP Federated Learning Client")]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Client ID
    #[arg(short = 'i', long)]
    client_id: Option<String>,

    /// Server URL
    #[arg(short, long)]
    server_url: Option<String>,

    /// Dataset path
    #[arg(short, long)]
    dataset_path: Option<String>,

    /// Number of training epochs
    #[arg(short, long)]
    epochs: Option<usize>,

    /// Enable detailed logging
    #[arg(short, long)]
    verbose: bool,

    /// Run in benchmark mode
    #[arg(short, long)]
    benchmark: bool,
}

pub struct Client {
    config: Config,
    client_id: String,
    session: Session,
    benchmark_result: BenchmarkResult,
    trainer: trainer::Trainer,
    prover: prover::ZkpProver,
    network: network::NetworkClient,
}

impl Client {
    pub async fn new(args: Args) -> Result<Self> {
        // Load configuration
        let config = Self::load_config(&args.config)?;

        // Override config with command line arguments
        let mut client_config = config.client.clone();
        if let Some(client_id) = args.client_id {
            client_config.client_id = client_id;
        }
        if let Some(server_url) = args.server_url {
            client_config.server_url = server_url;
        }
        if let Some(epochs) = args.epochs {
            client_config.training_epochs = epochs;
        }

        let session_id = Uuid::new_v4();
        let session = Session {
            id: session_id,
            client_id: client_config.client_id.clone(),
            start_time: Utc::now(),
            end_time: None,
            status: SessionStatus::Starting,
            metrics: common::SessionMetrics {
                training_time_ms: 0,
                proof_generation_time_ms: 0,
                proof_verification_time_ms: 0,
                proof_size_bytes: 0,
                final_loss: 0.0,
                num_epochs: 0,
            },
        };

        let benchmark_result = BenchmarkResult::new(session_id, client_config.client_id.clone());

        // Initialize components
        let trainer = trainer::Trainer::new(&config.circuit, &config.dataset)?;
        let prover = prover::ZkpProver::new(&config.circuit)?;
        let network = network::NetworkClient::new(&client_config.server_url)?;

        Ok(Self {
            config,
            client_id: client_config.client_id,
            session,
            benchmark_result,
            trainer,
            prover,
            network,
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
        info!("Starting ZKP-FL client: {}", self.client_id);

        self.session.status = SessionStatus::Training;

        // Phase 1: Load and prepare dataset
        let dataset_metrics = self.load_dataset().await?;
        self.benchmark_result.operations.push(dataset_metrics);

        // Phase 2: Train model
        let training_metrics = self.train_model().await?;
        self.benchmark_result.operations.push(training_metrics);

        // Phase 3: Generate proof
        self.session.status = SessionStatus::GeneratingProof;
        let proof_metrics = self.generate_proof().await?;
        self.benchmark_result.operations.push(proof_metrics);

        // Phase 4: Send proof to server
        self.session.status = SessionStatus::Verifying;
        let verification_metrics = self.submit_proof().await?;
        self.benchmark_result.operations.push(verification_metrics);

        // Phase 5: Finalize session
        self.session.status = SessionStatus::Completed;
        self.session.end_time = Some(Utc::now());

        self.benchmark_result.finish(true, None);
        self.save_benchmark_results()?;

        info!("Client run completed successfully");
        Ok(())
    }
    async fn load_dataset(&mut self) -> Result<OperationMetrics> {
        let mut metrics = OperationMetrics::new("dataset_loading".to_string());
        info!("Loading dataset...");

        let _start = Instant::now();

        // Try to load from file first, then create synthetic if not available
        let dataset =
            if let Some(path) = self.config.dataset.path.as_str().strip_prefix("synthetic:") {
                let params: Vec<&str> = path.split(',').collect();
                let num_samples = params.get(0).unwrap_or(&"1000").parse().unwrap_or(1000);
                let num_features = params.get(1).unwrap_or(&"5").parse().unwrap_or(5);
                info!(
                    "Creating synthetic dataset with {} samples, {} features",
                    num_samples, num_features
                );
                HealthcareDataset::create_synthetic(num_samples, num_features)
            } else {
                info!("Loading dataset from file: {}", self.config.dataset.path);
                HealthcareDataset::load_from_csv(
                    &self.config.dataset.path,
                    &self.config.dataset.target_column,
                    &self.config.dataset.feature_columns,
                )
                .unwrap_or_else(|_| {
                    warn!("Failed to load dataset from file, creating synthetic dataset");
                    HealthcareDataset::create_synthetic(1000, 5)
                })
            };

        self.trainer.set_dataset(dataset)?;

        metrics.finish();
        metrics.add_metadata("num_samples", self.trainer.get_dataset_size());
        metrics.add_metadata("num_features", self.trainer.get_num_features());

        info!("Dataset loaded in {}ms", metrics.duration_ms);
        Ok(metrics)
    }
    async fn train_model(&mut self) -> Result<OperationMetrics> {
        let mut metrics = OperationMetrics::new("model_training".to_string());
        info!("Starting model training...");

        let _start = Instant::now();
        let training_result = self
            .trainer
            .train(self.config.client.training_epochs)
            .await?;

        metrics.finish();
        metrics.add_metadata("epochs", training_result.epochs_completed);
        metrics.add_metadata("final_loss", training_result.final_loss);
        metrics.add_metadata("convergence_epoch", training_result.convergence_epoch);

        // Update session metrics
        self.session.metrics.training_time_ms = metrics.duration_ms;
        self.session.metrics.final_loss = training_result.final_loss;
        self.session.metrics.num_epochs = training_result.epochs_completed;
        // Update benchmark result
        let final_loss = training_result.final_loss;
        self.benchmark_result.training_metrics = training_result;

        info!(
            "Model training completed in {}ms, final loss: {:.6}",
            metrics.duration_ms, final_loss
        );

        Ok(metrics)
    }

    async fn generate_proof(&mut self) -> Result<OperationMetrics> {
        let mut metrics = OperationMetrics::new("proof_generation".to_string());
        info!("Generating ZKP proof...");
        let training_params = self.trainer.get_training_params()?;
        let samples = self.trainer.get_training_samples()?;

        let _start = Instant::now();
        let proof = self
            .prover
            .generate_proof(samples, &training_params)
            .await?;

        metrics.finish();
        metrics.add_metadata("proof_size_bytes", proof.proof_size());
        metrics.add_metadata(
            "circuit_constraints",
            proof.proof_data.circuit_params.num_constraints,
        );

        // Update session metrics
        self.session.metrics.proof_generation_time_ms = metrics.duration_ms;
        self.session.metrics.proof_size_bytes = proof.proof_size();
        // Update benchmark result with proof metadata
        self.benchmark_result.zkp_metrics.proof_generation_time_ms =
            proof.proof_data.metadata.generation_time_ms;
        self.benchmark_result.zkp_metrics.witness_generation_time_ms =
            proof.proof_data.metadata.witness_generation_time_ms;
        self.benchmark_result.zkp_metrics.setup_time_ms = proof.proof_data.metadata.setup_time_ms;
        self.benchmark_result.zkp_metrics.proof_size_bytes = proof.proof_size();
        self.benchmark_result.zkp_metrics.circuit_constraints =
            proof.proof_data.circuit_params.num_constraints;

        // Store proof size before moving
        let proof_size = proof.proof_size();

        // Store proof for submission
        self.prover.set_current_proof(proof);

        info!(
            "Proof generated in {}ms, size: {} bytes",
            metrics.duration_ms, proof_size
        );

        Ok(metrics)
    }

    async fn submit_proof(&mut self) -> Result<OperationMetrics> {
        let mut metrics = OperationMetrics::new("proof_submission".to_string());
        info!("Submitting proof to server...");
        let proof = self.prover.get_current_proof()?;

        let _start = Instant::now();
        let verification_result = self.network.submit_proof(proof).await?;

        metrics.finish();
        metrics.add_metadata("verified", verification_result.verified);
        metrics.add_metadata(
            "verification_time_ms",
            verification_result.verification_time_ms,
        );

        // Update session metrics
        self.session.metrics.proof_verification_time_ms = verification_result.verification_time_ms;

        // Update benchmark result
        self.benchmark_result.zkp_metrics.proof_verification_time_ms =
            verification_result.verification_time_ms;

        if verification_result.verified {
            info!(
                "Proof verified successfully in {}ms",
                verification_result.verification_time_ms
            );
            self.session.status = SessionStatus::Verified;
        } else {
            error!(
                "Proof verification failed: {:?}",
                verification_result.error_message
            );
            self.session.status = SessionStatus::Failed;
            return Err(ZkpFlError::ProofVerification(
                verification_result
                    .error_message
                    .unwrap_or_else(|| "Unknown verification error".to_string()),
            ));
        }

        Ok(metrics)
    }

    fn save_benchmark_results(&self) -> Result<()> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("benchmark_{}_client_{}.json", timestamp, self.client_id);
        let filepath = std::path::Path::new(&self.config.benchmarks.output_path).join(filename);

        // Ensure directory exists
        if let Some(parent) = filepath.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.benchmark_result)?;
        std::fs::write(&filepath, json)?;

        info!("Benchmark results saved to: {:?}", filepath);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("ZKP-FL Client starting...");

    let mut client = Client::new(args).await?;

    match client.run().await {
        Ok(()) => {
            info!("Client completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Client failed: {}", e);
            client.benchmark_result.finish(false, Some(e.to_string()));
            client.save_benchmark_results()?;
            Err(e)
        }
    }
}
