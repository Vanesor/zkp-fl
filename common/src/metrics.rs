use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

/// Benchmark metrics for a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub operation_id: Uuid,
    pub operation_type: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// ZKP-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpMetrics {
    pub setup_time_ms: u64,
    pub witness_generation_time_ms: u64,
    pub proof_generation_time_ms: u64,
    pub proof_verification_time_ms: u64,
    pub proof_size_bytes: usize,
    pub circuit_constraints: usize,
    pub circuit_advice_columns: usize,
    pub circuit_fixed_columns: usize,
    pub folding_iterations: usize,
}

/// Training metrics for ML model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub dataset_size: usize,
    pub num_features: usize,
    pub training_time_ms: u64,
    pub epochs_completed: usize,
    pub final_loss: f64,
    pub initial_loss: f64,
    pub convergence_epoch: Option<usize>,
    pub loss_history: Vec<f64>,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub memory_total_mb: f64,
    pub disk_usage_mb: f64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
}

/// Comprehensive benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub session_id: Uuid,
    pub client_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub total_duration_ms: u64,
    pub zkp_metrics: ZkpMetrics,
    pub training_metrics: TrainingMetrics,
    pub system_metrics: Vec<SystemMetrics>,
    pub operations: Vec<OperationMetrics>,
    pub success: bool,
    pub error_message: Option<String>,
    
    // Additional fields for multi-client benchmarking compatibility
    pub id: Option<Uuid>,
    pub timestamp: Option<DateTime<Utc>>,
    pub scenario: Option<String>,
    pub num_clients: Option<usize>,
    pub num_rounds: Option<usize>,
    pub total_duration: Option<std::time::Duration>,
    pub successful_clients: Option<usize>,
    pub failed_clients: Option<usize>,
    pub avg_training_time: Option<std::time::Duration>,
    pub avg_proof_time: Option<std::time::Duration>,
    pub avg_verification_time: Option<std::time::Duration>,
    pub avg_proof_size: Option<usize>,
    pub client_metrics: Option<Vec<crate::types::ClientMetrics>>,
    pub throughput: Option<f64>,
    pub success_rate: Option<f64>,
}

/// Multi-client benchmark aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiClientBenchmark {
    pub benchmark_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub num_clients: usize,
    pub client_results: Vec<BenchmarkResult>,
    pub aggregate_metrics: AggregateMetrics,
}

/// Aggregated metrics across multiple clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub avg_proof_generation_time_ms: f64,
    pub min_proof_generation_time_ms: u64,
    pub max_proof_generation_time_ms: u64,
    pub avg_proof_verification_time_ms: f64,
    pub avg_training_time_ms: f64,
    pub total_proofs_generated: usize,
    pub total_proofs_verified: usize,
    pub success_rate: f64,
    pub throughput_proofs_per_second: f64,
}

impl OperationMetrics {
    pub fn new(operation_type: String) -> Self {
        let now = Utc::now();
        Self {
            operation_id: Uuid::new_v4(),
            operation_type,
            start_time: now,
            end_time: now,
            duration_ms: 0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            metadata: HashMap::new(),
        }
    }

    pub fn finish(&mut self) {
        self.end_time = Utc::now();
        self.duration_ms = (self.end_time - self.start_time).num_milliseconds() as u64;
    }

    pub fn add_metadata<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.metadata.insert(key.to_string(), json_value);
        }
    }
}

impl SystemMetrics {
    pub fn current() -> Self {
        // In a real implementation, you would collect actual system metrics
        // For now, we'll use placeholder values
        Self {
            timestamp: Utc::now(),
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            memory_total_mb: 0.0,
            disk_usage_mb: 0.0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
        }
    }
}

impl BenchmarkResult {
    pub fn new(session_id: Uuid, client_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            client_id,
            start_time: now,
            end_time: now,
            total_duration_ms: 0,
            zkp_metrics: ZkpMetrics {
                setup_time_ms: 0,
                witness_generation_time_ms: 0,
                proof_generation_time_ms: 0,
                proof_verification_time_ms: 0,
                proof_size_bytes: 0,
                circuit_constraints: 0,
                circuit_advice_columns: 0,
                circuit_fixed_columns: 0,
                folding_iterations: 0,
            },
            training_metrics: TrainingMetrics {
                dataset_size: 0,
                num_features: 0,
                training_time_ms: 0,
                epochs_completed: 0,
                final_loss: 0.0,
                initial_loss: 0.0,
                convergence_epoch: None,
                loss_history: Vec::new(),
            },            system_metrics: Vec::new(),
            operations: Vec::new(),
            success: false,
            error_message: None,
            
            // Initialize optional fields for compatibility
            id: None,
            timestamp: None,
            scenario: None,
            num_clients: None,
            num_rounds: None,
            total_duration: None,
            successful_clients: None,
            failed_clients: None,
            avg_training_time: None,
            avg_proof_time: None,
            avg_verification_time: None,
            avg_proof_size: None,
            client_metrics: None,
            throughput: None,
            success_rate: None,
        }
    }

    pub fn finish(&mut self, success: bool, error_message: Option<String>) {
        self.end_time = Utc::now();
        self.total_duration_ms = (self.end_time - self.start_time).num_milliseconds() as u64;
        self.success = success;
        self.error_message = error_message;
    }
}
