use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Configuration for the ZKP-FL system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub client: ClientConfig,
    pub circuit: CircuitConfig,
    pub dataset: DatasetConfig,
    pub benchmarks: BenchmarkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_clients: usize,
    pub proof_storage_path: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server_url: String,
    pub client_id: String,
    pub training_epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitConfig {
    pub k: u32, // Circuit size parameter (2^k rows)
    pub num_features: usize,
    pub precision_bits: usize,
    pub max_iterations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub path: String,
    pub target_column: String,
    pub feature_columns: Vec<String>,
    pub train_test_split: f64,
    pub normalize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub output_path: String,
    pub metrics_interval_ms: u64,
    pub detailed_logging: bool,
}

/// Training parameters for linear regression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingParams {
    pub weights: Vec<f64>,
    pub bias: f64,
    pub loss: f64,
    pub epoch: usize,
    pub learning_rate: f64,
}

/// Dataset sample for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub features: Vec<f64>,
    pub target: f64,
}

/// Training batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingBatch {
    pub samples: Vec<Sample>,
    pub batch_id: usize,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub client_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub status: SessionStatus,
    pub metrics: SessionMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Starting,
    Training,
    GeneratingProof,
    ProofGenerated,
    Verifying,
    Verified,
    Failed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub training_time_ms: u64,
    pub proof_generation_time_ms: u64,
    pub proof_verification_time_ms: u64,
    pub proof_size_bytes: usize,
    pub final_loss: f64,
    pub num_epochs: usize,
}

/// Client metrics for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMetrics {
    pub client_id: String,
    pub training_times: Vec<Duration>,
    pub proof_times: Vec<Duration>,
    pub witness_times: Vec<Duration>,
    pub verification_times: Vec<Duration>,
    pub proof_sizes: Vec<usize>,
    pub success_count: usize,
    pub failure_count: usize,
    pub avg_training_time: Duration,
    pub avg_proof_time: Duration,
    pub avg_witness_time: Duration,
    pub avg_verification_time: Duration,
    pub avg_proof_size: usize,
    pub total_time: Duration,
}

/// Result of proof operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResult {
    pub success: bool,
    pub proof_id: Option<String>,
    pub training_time: Duration,
    pub proof_time: Duration,
    pub verification_time: Duration,
    pub proof_size: usize,
    pub error: Option<String>,
}

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum ZkpFlError {
    #[error("Circuit error: {0}")]
    Circuit(String),

    #[error("Proof generation error: {0}")]
    ProofGeneration(String),
    #[error("Proof verification error: {0}")]
    ProofVerification(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Benchmark error: {0}")]
    Benchmark(String),

    #[error("Dataset error: {0}")]
    Dataset(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ZkpFlError>;
