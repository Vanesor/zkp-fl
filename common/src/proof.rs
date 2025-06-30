use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// ZKP proof structure that gets sent between client and server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkpProof {
    pub proof_id: Uuid,
    pub session_id: Uuid,
    pub client_id: String,
    pub timestamp: DateTime<Utc>,
    pub proof_data: ProofData,
    pub training_commitment: TrainingCommitment,
    pub verification_result: Option<VerificationResult>,
}

/// The actual proof data from the circuit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofData {
    /// Serialized proof bytes
    pub proof_bytes: Vec<u8>,
    /// Public inputs to the circuit
    pub public_inputs: Vec<String>,
    /// Circuit parameters used
    pub circuit_params: CircuitParams,
    /// Proof generation metadata
    pub metadata: ProofMetadata,
}

/// Commitment to the training process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCommitment {
    /// Hash of the dataset used
    pub dataset_hash: String,
    /// Number of training samples
    pub num_samples: usize,
    /// Number of features
    pub num_features: usize,
    /// Training parameters used
    pub learning_rate: f64,
    /// Number of epochs
    pub epochs: usize,
    /// Final model weights (committed)
    pub weights_commitment: String,
    /// Final loss value
    pub final_loss: f64,
}

/// Circuit parameters used for proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitParams {
    pub k: u32,
    pub num_constraints: usize,
    pub num_advice_columns: usize,
    pub num_fixed_columns: usize,
    pub max_degree: usize,
}

/// Metadata about proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    pub generation_time_ms: u64,
    pub proof_size_bytes: usize,
    pub witness_generation_time_ms: u64,
    pub setup_time_ms: u64,
    pub folding_iterations: usize,
}

/// Result of proof verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub verification_time_ms: u64,
    pub verifier_id: String,
    pub verification_timestamp: DateTime<Utc>,
    pub error_message: Option<String>,
}

/// Request to verify a proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    pub proof: ZkpProof,
    pub requester_id: String,
}

/// Response from proof verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResponse {
    pub proof_id: Uuid,
    pub result: VerificationResult,
}

/// Batch of proofs for multi-client scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofBatch {
    pub batch_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub proofs: Vec<ZkpProof>,
    pub batch_metadata: BatchMetadata,
}

/// Metadata for proof batches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    pub num_proofs: usize,
    pub total_size_bytes: usize,
    pub submission_order: Vec<Uuid>, // Order of proof submission
}

impl ZkpProof {
    pub fn new(
        session_id: Uuid,
        client_id: String,
        proof_bytes: Vec<u8>,
        public_inputs: Vec<String>,
        circuit_params: CircuitParams,
        metadata: ProofMetadata,
        training_commitment: TrainingCommitment,
    ) -> Self {
        Self {
            proof_id: Uuid::new_v4(),
            session_id,
            client_id,
            timestamp: Utc::now(),
            proof_data: ProofData {
                proof_bytes,
                public_inputs,
                circuit_params,
                metadata,
            },
            training_commitment,
            verification_result: None,
        }
    }

    pub fn mark_verified(&mut self, result: VerificationResult) {
        self.verification_result = Some(result);
    }

    pub fn is_verified(&self) -> bool {
        self.verification_result
            .as_ref()
            .map(|r| r.verified)
            .unwrap_or(false)
    }

    pub fn proof_size(&self) -> usize {
        self.proof_data.proof_bytes.len()
    }
}

impl ProofBatch {
    pub fn new(proofs: Vec<ZkpProof>) -> Self {
        let num_proofs = proofs.len();
        let total_size_bytes = proofs.iter().map(|p| p.proof_size()).sum();
        let submission_order = proofs.iter().map(|p| p.proof_id).collect();

        Self {
            batch_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            proofs,
            batch_metadata: BatchMetadata {
                num_proofs,
                total_size_bytes,
                submission_order,
            },
        }
    }

    pub fn add_proof(&mut self, proof: ZkpProof) {
        self.batch_metadata.submission_order.push(proof.proof_id);
        self.batch_metadata.total_size_bytes += proof.proof_size();
        self.batch_metadata.num_proofs += 1;
        self.proofs.push(proof);
    }

    pub fn verified_proofs(&self) -> Vec<&ZkpProof> {
        self.proofs.iter().filter(|p| p.is_verified()).collect()
    }

    pub fn verification_rate(&self) -> f64 {
        if self.proofs.is_empty() {
            0.0
        } else {
            self.verified_proofs().len() as f64 / self.proofs.len() as f64
        }
    }
}
