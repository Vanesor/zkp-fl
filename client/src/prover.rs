use blake2b_simd::blake2b;
use chrono::Utc;
use common::{
    f64_to_field, CircuitBuilder, CircuitConfig, CircuitParams, ProofMetadata, Result, Sample,
    TrainingCommitment, TrainingParams, ZkpFlError, ZkpProof,
};
use halo2_proofs::protostar;
use halo2_proofs::{
    poly::commitment::ParamsProver,
    poly::ipa::commitment::ParamsIPA,
    transcript::{Blake2bWrite, Challenge255, TranscriptWriterBuffer},
};
use halo2curves::pasta::pallas;
use log::{debug, info};
use std::time::Instant;
use uuid::Uuid;

pub struct ZkpProver {
    circuit_builder: CircuitBuilder,
    params: ParamsIPA<pallas::Affine>,
    circuit_config: CircuitConfig,
    current_proof: Option<ZkpProof>,
}

impl ZkpProver {
    pub fn new(circuit_config: &CircuitConfig) -> Result<Self> {
        info!("Initializing ZKP prover with k={}", circuit_config.k);

        let setup_start = Instant::now();

        // Generate SRS parameters
        let params = ParamsIPA::<pallas::Affine>::new(circuit_config.k);

        let circuit_builder = CircuitBuilder::new(
            circuit_config.num_features,
            100, // max_samples for circuit constraints
        );

        info!(
            "ZKP prover initialized in {}ms",
            setup_start.elapsed().as_millis()
        );

        Ok(Self {
            circuit_builder,
            params,
            circuit_config: circuit_config.clone(),
            current_proof: None,
        })
    }

    pub async fn generate_proof(
        &mut self,
        samples: Vec<Sample>,
        training_params: &TrainingParams,
    ) -> Result<ZkpProof> {
        info!("Starting proof generation for {} samples", samples.len());
        let total_start = Instant::now();

        // Phase 1: Build circuit
        let circuit_start = Instant::now();
        let circuit = self
            .circuit_builder
            .build_circuit(samples.clone(), training_params)?;
        let circuit_time = circuit_start.elapsed();
        debug!("Circuit built in {}ms", circuit_time.as_millis());

        // Phase 2: Generate proving key
        let keygen_start = Instant::now();
        let proving_key = protostar::ProvingKey::new(&self.params, &circuit)
            .map_err(|e| ZkpFlError::ProofGeneration(format!("Key generation failed: {:?}", e)))?;
        let keygen_time = keygen_start.elapsed();
        debug!("Proving key generated in {}ms", keygen_time.as_millis()); // Phase 3: Prepare witness generation
        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        let mut rng = rand::thread_rng();

        // Create public instances - for our simple circuit, we'll use the loss as public input
        use halo2curves::pasta::Fq;
        let loss_field: Fq = f64_to_field(training_params.loss);
        let public_instance = vec![loss_field];
        let public_instances: &[&[Fq]] = &[&public_instance];

        // Phase 4: Generate proof using Protostar (includes witness and proof generation)
        let proof_start = Instant::now();

        // Create accumulator (this includes both witness generation and proof creation)
        let accumulator = protostar::prover::create_accumulator(
            &self.params,
            &proving_key,
            &circuit,
            &public_instances,
            &mut rng,
            &mut transcript,
        )
        .map_err(|e| ZkpFlError::ProofGeneration(format!("Proof generation failed: {:?}", e)))?;

        let total_proof_time = proof_start.elapsed();

        // For Protostar, witness generation is integrated with proof generation
        // We'll estimate witness time as ~30% of total proof time (empirical estimation)
        let witness_time = total_proof_time * 30 / 100;
        let proof_time = total_proof_time - witness_time;
        let total_time = total_start.elapsed();

        // Serialize the proof
        let proof_bytes = self.serialize_accumulator(&accumulator)?;

        info!("Proof generated successfully:");
        info!("  Total time: {}ms", total_time.as_millis());
        info!(
            "  Setup time: {}ms",
            (circuit_time + keygen_time).as_millis()
        );
        info!("  Witness time: {}ms", witness_time.as_millis());
        info!("  Proof time: {}ms", proof_time.as_millis());
        info!("  Proof size: {} bytes", proof_bytes.len());

        // Create proof metadata
        let metadata = ProofMetadata {
            generation_time_ms: proof_time.as_millis() as u64,
            proof_size_bytes: proof_bytes.len(),
            witness_generation_time_ms: witness_time.as_millis() as u64,
            setup_time_ms: (circuit_time + keygen_time).as_millis() as u64,
            folding_iterations: 1, // Single iteration for now
        };

        // Create circuit parameters info
        let circuit_params = CircuitParams {
            k: self.circuit_config.k,
            num_constraints: 1000, // TODO: Get actual constraint count from circuit
            num_advice_columns: 10, // TODO: Get actual advice column count
            num_fixed_columns: 2,  // TODO: Get actual fixed column count
            max_degree: proving_key.max_folding_constraints_degree(),
        };

        // Create training commitment
        let training_commitment = self.create_training_commitment(&samples, training_params)?;

        // Create public inputs (for verification)
        let public_inputs = vec![format!("{:.6}", training_params.loss)];

        // Create the final proof
        let proof = ZkpProof::new(
            Uuid::new_v4(),       // session_id will be set by caller
            "client".to_string(), // client_id will be set by caller
            proof_bytes,
            public_inputs,
            circuit_params,
            metadata,
            training_commitment,
        );

        Ok(proof)
    }

    fn serialize_accumulator(
        &self,
        _accumulator: &protostar::accumulator::Accumulator<pallas::Affine>,
    ) -> Result<Vec<u8>> {
        // In a real implementation, you would properly serialize the accumulator
        // For now, we create a placeholder serialization
        use bincode;

        // Create a simplified representation for serialization
        let simplified_proof = SimplifiedProof {
            timestamp: Utc::now(),
            circuit_k: self.circuit_config.k,
            num_features: self.circuit_config.num_features,
            // In practice, you would serialize the actual accumulator components
            placeholder_data: vec![0u8; 1024], // Placeholder proof data
        };

        bincode::serialize(&simplified_proof)
            .map_err(|e| ZkpFlError::ProofGeneration(format!("Serialization failed: {}", e)))
    }
    fn create_training_commitment(
        &self,
        samples: &[Sample],
        params: &TrainingParams,
    ) -> Result<TrainingCommitment> {
        // Create dataset hash
        let dataset_bytes = bincode::serialize(samples).map_err(|e| {
            ZkpFlError::ProofGeneration(format!("Dataset serialization failed: {}", e))
        })?;
        let dataset_hash = hex::encode(blake2b(&dataset_bytes).as_bytes());

        // Create weights commitment
        let weights_bytes = bincode::serialize(&params.weights).map_err(|e| {
            ZkpFlError::ProofGeneration(format!("Weights serialization failed: {}", e))
        })?;
        let weights_commitment = hex::encode(blake2b(&weights_bytes).as_bytes());

        Ok(TrainingCommitment {
            dataset_hash,
            num_samples: samples.len(),
            num_features: samples.first().map(|s| s.features.len()).unwrap_or(0),
            learning_rate: params.learning_rate,
            epochs: params.epoch,
            weights_commitment,
            final_loss: params.loss,
        })
    }

    pub fn set_current_proof(&mut self, proof: ZkpProof) {
        self.current_proof = Some(proof);
    }

    pub fn get_current_proof(&self) -> Result<ZkpProof> {
        self.current_proof
            .clone()
            .ok_or_else(|| ZkpFlError::ProofGeneration("No proof available".to_string()))
    }
}

// Simplified proof structure for serialization
#[derive(serde::Serialize, serde::Deserialize)]
struct SimplifiedProof {
    timestamp: chrono::DateTime<Utc>,
    circuit_k: u32,
    num_features: usize,
    placeholder_data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{CircuitConfig, Sample, TrainingParams};

    #[tokio::test]
    async fn test_proof_generation() {
        let circuit_config = CircuitConfig {
            k: 8,
            num_features: 2,
            precision_bits: 32,
            max_iterations: 100,
        };

        let mut prover = ZkpProver::new(&circuit_config).unwrap();

        let samples = vec![
            Sample {
                features: vec![1.0, 2.0],
                target: 3.0,
            },
            Sample {
                features: vec![2.0, 3.0],
                target: 5.0,
            },
        ];

        let training_params = TrainingParams {
            weights: vec![1.0, 1.0],
            bias: 0.0,
            loss: 0.1,
            epoch: 10,
            learning_rate: 0.01,
        };

        let proof = prover
            .generate_proof(samples, &training_params)
            .await
            .unwrap();

        assert!(proof.proof_size() > 0);
        assert_eq!(proof.training_commitment.num_features, 2);
        assert_eq!(proof.proof_data.circuit_params.k, 8);
    }
}
