use serde::{Deserialize, Serialize};

/// Server status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub active_clients: usize,
    pub total_proofs_verified: usize,
    pub uptime_seconds: u64,
    pub server_version: String,
}

/// Batch verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerificationRequest {
    pub proofs: Vec<String>, // Proof IDs
    pub priority: Option<u8>,
}

/// Batch verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerificationResponse {
    pub results: Vec<BatchVerificationResult>,
    pub total_time_ms: u64,
}

/// Individual proof verification result in batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerificationResult {
    pub proof_id: String,
    pub verified: bool,
    pub verification_time_ms: u64,
    pub error: Option<String>,
}
