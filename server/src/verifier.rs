use common::{
    ZkpProof, VerificationResult, TrainingCommitment, ProofData,
    CircuitConfig, Result
};
use halo2_proofs::{
    poly::ipa::{
        commitment::{ParamsIPA},
    },
    poly::commitment::ParamsProver,
};
use halo2curves::pasta::pallas;
use log::{info, debug};
use std::time::Instant;
use chrono::Utc;

pub struct ProofVerifier {
    params: ParamsIPA<pallas::Affine>,
    circuit_config: CircuitConfig,
    stats: VerificationStats,
}

#[derive(Debug, Default)]
struct VerificationStats {
    total_verifications: usize,
    successful_verifications: usize,
    total_verification_time_ms: u64,
}

impl ProofVerifier {
    pub fn new(circuit_config: &CircuitConfig) -> Result<Self> {
        info!("Initializing proof verifier with k={}", circuit_config.k);
        
        let setup_start = Instant::now();
        
        // Generate the same SRS parameters as the client
        let params = ParamsIPA::<pallas::Affine>::new(circuit_config.k);
        
        info!("Proof verifier initialized in {}ms", setup_start.elapsed().as_millis());
        
        Ok(Self {
            params,
            circuit_config: circuit_config.clone(),
            stats: VerificationStats::default(),
        })
    }

    pub async fn verify_proof(&mut self, proof: &ZkpProof) -> Result<VerificationResult> {
        info!("Verifying proof {} from client {}", proof.proof_id, proof.client_id);
        let start_time = Instant::now();
        
        // Phase 1: Validate proof structure
        let validation_result = self.validate_proof_structure(proof)?;
        if !validation_result.is_valid {
            return Ok(VerificationResult {
                verified: false,
                verification_time_ms: start_time.elapsed().as_millis() as u64,
                verifier_id: "server".to_string(),
                verification_timestamp: Utc::now(),
                error_message: Some(validation_result.error_message),
            });
        }

        // Phase 2: Verify training commitment
        let commitment_result = self.verify_training_commitment(&proof.training_commitment)?;
        if !commitment_result.is_valid {
            return Ok(VerificationResult {
                verified: false,
                verification_time_ms: start_time.elapsed().as_millis() as u64,
                verifier_id: "server".to_string(),
                verification_timestamp: Utc::now(),
                error_message: Some(commitment_result.error_message),
            });
        }

        // Phase 3: Verify the actual ZKP
        let zkp_result = self.verify_zkp(&proof.proof_data).await?;
        
        let verification_time = start_time.elapsed().as_millis() as u64;
        
        // Update stats
        self.stats.total_verifications += 1;
        self.stats.total_verification_time_ms += verification_time;
        if zkp_result.is_valid {
            self.stats.successful_verifications += 1;
        }

        let verification_result = VerificationResult {
            verified: zkp_result.is_valid,
            verification_time_ms: verification_time,
            verifier_id: "server".to_string(),
            verification_timestamp: Utc::now(),
            error_message: if zkp_result.is_valid { None } else { Some(zkp_result.error_message) },
        };

        info!("Proof verification completed: verified={}, time={}ms", 
              verification_result.verified, verification_time);
        
        Ok(verification_result)
    }

    fn validate_proof_structure(&self, proof: &ZkpProof) -> Result<ValidationResult> {
        debug!("Validating proof structure");

        // Check circuit parameters
        if proof.proof_data.circuit_params.k != self.circuit_config.k {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: format!(
                    "Circuit parameter k mismatch: expected {}, got {}",
                    self.circuit_config.k, proof.proof_data.circuit_params.k
                ),
            });
        }

        // Check proof size is reasonable
        if proof.proof_data.proof_bytes.is_empty() {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Proof data is empty".to_string(),
            });
        }

        if proof.proof_data.proof_bytes.len() > 10_000_000 { // 10MB max
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Proof data too large".to_string(),
            });
        }

        // Check training commitment has required fields
        if proof.training_commitment.num_features != self.circuit_config.num_features {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: format!(
                    "Feature count mismatch: expected {}, got {}",
                    self.circuit_config.num_features, proof.training_commitment.num_features
                ),
            });
        }

        // Check timestamps are reasonable
        let now = Utc::now();
        if proof.timestamp > now || (now - proof.timestamp).num_hours() > 24 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Proof timestamp is invalid".to_string(),
            });
        }

        debug!("Proof structure validation passed");
        Ok(ValidationResult {
            is_valid: true,
            error_message: String::new(),
        })
    }

    fn verify_training_commitment(&self, commitment: &TrainingCommitment) -> Result<ValidationResult> {
        debug!("Verifying training commitment");

        // Check reasonable bounds on training parameters
        if commitment.learning_rate <= 0.0 || commitment.learning_rate > 1.0 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: format!("Invalid learning rate: {}", commitment.learning_rate),
            });
        }

        if commitment.epochs == 0 || commitment.epochs > 10000 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: format!("Invalid epoch count: {}", commitment.epochs),
            });
        }

        if commitment.num_samples == 0 || commitment.num_samples > 1_000_000 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: format!("Invalid sample count: {}", commitment.num_samples),
            });
        }        // Check hash formats
        if commitment.dataset_hash.len() != 128 { // Blake2b hash is 64 bytes = 128 hex chars
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Invalid dataset hash format".to_string(),
            });
        }

        if commitment.weights_commitment.len() != 128 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Invalid weights commitment format".to_string(),
            });
        }

        debug!("Training commitment verification passed");
        Ok(ValidationResult {
            is_valid: true,
            error_message: String::new(),
        })
    }

    async fn verify_zkp(&self, proof_data: &ProofData) -> Result<ValidationResult> {
        debug!("Verifying ZKP using Protostar");

        // In a real implementation, this would:
        // 1. Deserialize the proof (accumulator)
        // 2. Create the verifier circuit
        // 3. Run the Protostar verification algorithm
        
        // For now, we simulate the verification process
        let verification_start = Instant::now();
        
        // Simulate some verification work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Parse proof data (in real implementation, deserialize the accumulator)
        let proof_result = self.simulate_proof_verification(proof_data);
        
        let verification_time = verification_start.elapsed();
        debug!("ZKP verification completed in {}ms", verification_time.as_millis());

        proof_result
    }

    fn simulate_proof_verification(&self, proof_data: &ProofData) -> Result<ValidationResult> {
        // This is a simulation of the actual proof verification
        // In a real implementation, you would:
        // 1. Deserialize the Protostar accumulator from proof_data.proof_bytes
        // 2. Create a verification circuit with the same parameters
        // 3. Run the Protostar verifier algorithm
        
        // Basic checks on proof structure
        if proof_data.proof_bytes.len() < 100 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Proof data too small".to_string(),
            });
        }

        // Check circuit parameters are consistent
        if proof_data.circuit_params.num_advice_columns == 0 {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Invalid circuit parameters".to_string(),
            });
        }

        // Check public inputs format
        if proof_data.public_inputs.is_empty() {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "No public inputs provided".to_string(),
            });
        }

        // Parse and validate public inputs (loss value)
        if let Ok(loss) = proof_data.public_inputs[0].parse::<f64>() {
            if loss < 0.0 || loss > 1000.0 {
                return Ok(ValidationResult {
                    is_valid: false,
                    error_message: format!("Invalid loss value: {}", loss),
                });
            }
        } else {
            return Ok(ValidationResult {
                is_valid: false,
                error_message: "Invalid public input format".to_string(),
            });
        }

        // Simulate successful verification (in practice, this would be the actual Protostar verification)
        Ok(ValidationResult {
            is_valid: true,
            error_message: String::new(),
        })
    }

    pub fn get_stats(&self) -> VerificationStats {
        VerificationStats {
            total_verifications: self.stats.total_verifications,
            successful_verifications: self.stats.successful_verifications,
            total_verification_time_ms: self.stats.total_verification_time_ms,
        }
    }

    pub fn get_average_verification_time(&self) -> f64 {
        if self.stats.total_verifications == 0 {
            0.0
        } else {
            self.stats.total_verification_time_ms as f64 / self.stats.total_verifications as f64
        }
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.stats.total_verifications == 0 {
            0.0
        } else {
            self.stats.successful_verifications as f64 / self.stats.total_verifications as f64
        }
    }
}

#[derive(Debug)]
struct ValidationResult {
    is_valid: bool,
    error_message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{TrainingParams, Sample, CircuitParams, ProofMetadata, ProofData};

    #[tokio::test]
    async fn test_proof_verifier_creation() {
        let circuit_config = CircuitConfig {
            k: 8,
            num_features: 5,
            precision_bits: 32,
            max_iterations: 100,
        };

        let verifier = ProofVerifier::new(&circuit_config);
        assert!(verifier.is_ok());
    }

    #[tokio::test]
    async fn test_proof_structure_validation() {
        let circuit_config = CircuitConfig {
            k: 8,
            num_features: 5,
            precision_bits: 32,
            max_iterations: 100,
        };

        let verifier = ProofVerifier::new(&circuit_config).unwrap();
        
        // Create a valid proof structure
        let proof = create_test_proof();
        let result = verifier.validate_proof_structure(&proof).unwrap();
        assert!(result.is_valid);
    }

    fn create_test_proof() -> ZkpProof {
        let training_commitment = TrainingCommitment {
            dataset_hash: "a".repeat(128),
            num_samples: 100,
            num_features: 5,
            learning_rate: 0.01,
            epochs: 10,
            weights_commitment: "b".repeat(64),
            final_loss: 0.1,
        };

        let circuit_params = CircuitParams {
            k: 8,
            num_constraints: 100,
            num_advice_columns: 10,
            num_fixed_columns: 5,
            max_degree: 3,
        };

        let metadata = ProofMetadata {
            generation_time_ms: 1000,
            proof_size_bytes: 1024,
            witness_generation_time_ms: 500,
            setup_time_ms: 200,
            folding_iterations: 1,
        };

        let proof_data = ProofData {
            proof_bytes: vec![0u8; 1024],
            public_inputs: vec!["0.1".to_string()],
            circuit_params,
            metadata,
        };

        ZkpProof::new(
            Uuid::new_v4(),
            "test_client".to_string(),
            proof_data.proof_bytes.clone(),
            proof_data.public_inputs.clone(),
            proof_data.circuit_params.clone(),
            proof_data.metadata.clone(),
            training_commitment,
        )
    }
}
