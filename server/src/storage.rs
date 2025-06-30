use common::{
    ZkpProof, ProofBatch, ServerConfig, MultiClientBenchmark, 
    BenchmarkResult, Result, ZkpFlError
};
use dashmap::DashMap;
use parking_lot::RwLock;
use log::{info, debug, warn};
use std::sync::Arc;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde_json;

pub struct ProofStorage {
    // In-memory storage for active proofs
    active_proofs: DashMap<Uuid, ZkpProof>,
    
    // Batch storage for multi-client scenarios
    proof_batches: DashMap<Uuid, ProofBatch>,
    
    // Storage configuration
    storage_path: PathBuf,
    
    // Statistics
    stats: Arc<RwLock<StorageStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct StorageStats {
    pub total_proofs_stored: usize,
    pub total_proofs_verified: usize,
    pub total_batches: usize,
    pub storage_size_bytes: u64,
    pub last_cleanup: Option<DateTime<Utc>>,
}

impl ProofStorage {
    pub fn new(config: &ServerConfig, clear_on_startup: bool) -> Result<Self> {
        let storage_path = PathBuf::from(&config.proof_storage_path);
        
        // Create storage directory if it doesn't exist
        if let Some(parent) = storage_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ZkpFlError::Io(e))?;
        }

        if clear_on_startup {
            info!("Clearing proof storage on startup");
            if storage_path.exists() {
                std::fs::remove_dir_all(&storage_path)
                    .map_err(|e| ZkpFlError::Io(e))?;
            }
            std::fs::create_dir_all(&storage_path)
                .map_err(|e| ZkpFlError::Io(e))?;
        }

        let storage = Self {
            active_proofs: DashMap::new(),
            proof_batches: DashMap::new(),
            storage_path,
            stats: Arc::new(RwLock::new(StorageStats::default())),
        };

        // Load existing proofs if not clearing
        if !clear_on_startup {
            storage.load_existing_proofs()?;
        }

        info!("Proof storage initialized at: {:?}", storage.storage_path);
        Ok(storage)
    }

    pub async fn store_proof(&self, proof: ZkpProof) -> Result<()> {
        debug!("Storing proof {} from client {}", proof.proof_id, proof.client_id);

        // Store in memory
        self.active_proofs.insert(proof.proof_id, proof.clone());

        // Persist to disk
        self.persist_proof(&proof).await?;

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_proofs_stored += 1;
            if proof.is_verified() {
                stats.total_proofs_verified += 1;
            }
        }

        debug!("Proof {} stored successfully", proof.proof_id);
        Ok(())
    }

    pub async fn get_proof(&self, proof_id: &Uuid) -> Option<ZkpProof> {
        self.active_proofs.get(proof_id).map(|entry| entry.clone())
    }    pub async fn update_proof_verification(&self, proof_id: &Uuid, verified: bool) -> Result<()> {
        if let Some(_proof_entry) = self.active_proofs.get_mut(proof_id) {
            // Update verification status would go here
            // For now, we just update stats
            if verified {
                let mut stats = self.stats.write();
                stats.total_proofs_verified += 1;
            }
        }
        Ok(())
    }

    pub async fn create_batch(&self, client_proofs: Vec<ZkpProof>) -> Result<Uuid> {
        let batch = ProofBatch::new(client_proofs);
        let batch_id = batch.batch_id;
        
        info!("Creating proof batch {} with {} proofs", batch_id, batch.proofs.len());

        // Store batch in memory
        self.proof_batches.insert(batch_id, batch.clone());

        // Persist batch to disk
        self.persist_batch(&batch).await?;

        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_batches += 1;
        }

        Ok(batch_id)
    }

    pub async fn get_batch(&self, batch_id: &Uuid) -> Option<ProofBatch> {
        self.proof_batches.get(batch_id).map(|entry| entry.clone())
    }

    pub async fn get_all_proofs(&self) -> Vec<ZkpProof> {
        self.active_proofs.iter().map(|entry| entry.clone()).collect()
    }

    pub async fn get_verified_proofs(&self) -> Vec<ZkpProof> {
        self.active_proofs
            .iter()
            .filter(|entry| entry.is_verified())
            .map(|entry| entry.clone())
            .collect()
    }

    pub async fn get_client_proofs(&self, client_id: &str) -> Vec<ZkpProof> {
        self.active_proofs
            .iter()
            .filter(|entry| entry.client_id == client_id)
            .map(|entry| entry.clone())
            .collect()
    }

    pub async fn cleanup_old_proofs(&self, max_age_hours: i64) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut removed_count = 0;

        // Remove old proofs from memory
        self.active_proofs.retain(|_, proof| {
            if proof.timestamp < cutoff_time {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        // Clean up old batch data
        self.proof_batches.retain(|_, batch| {
            batch.timestamp >= cutoff_time
        });

        // Update cleanup timestamp
        {
            let mut stats = self.stats.write();
            stats.last_cleanup = Some(Utc::now());
        }

        if removed_count > 0 {
            info!("Cleaned up {} old proofs (older than {} hours)", removed_count, max_age_hours);
        }

        Ok(removed_count)
    }

    pub async fn update_metrics(&self, _current_metrics: &crate::metrics::ServerMetricsSnapshot) {
        // Update storage size calculation
        let storage_size = self.calculate_storage_size().await;
        
        {
            let mut stats = self.stats.write();
            stats.storage_size_bytes = storage_size;
        }
    }

    pub fn get_stats(&self) -> StorageStats {
        self.stats.read().clone()
    }

    async fn persist_proof(&self, proof: &ZkpProof) -> Result<()> {
        let filename = format!("proof_{}.json", proof.proof_id);
        let filepath = self.storage_path.join("proofs").join(filename);
        
        // Ensure proofs directory exists
        if let Some(parent) = filepath.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ZkpFlError::Io(e))?;
        }

        let json_data = serde_json::to_string_pretty(proof)
            .map_err(|e| ZkpFlError::Serialization(e))?;

        tokio::fs::write(&filepath, json_data).await
            .map_err(|e| ZkpFlError::Io(e))?;

        Ok(())
    }

    async fn persist_batch(&self, batch: &ProofBatch) -> Result<()> {
        let filename = format!("batch_{}.json", batch.batch_id);
        let filepath = self.storage_path.join("batches").join(filename);
        
        // Ensure batches directory exists
        if let Some(parent) = filepath.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ZkpFlError::Io(e))?;
        }

        let json_data = serde_json::to_string_pretty(batch)
            .map_err(|e| ZkpFlError::Serialization(e))?;

        tokio::fs::write(&filepath, json_data).await
            .map_err(|e| ZkpFlError::Io(e))?;

        Ok(())
    }

    fn load_existing_proofs(&self) -> Result<()> {
        let proofs_dir = self.storage_path.join("proofs");
        if !proofs_dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&proofs_dir)
            .map_err(|e| ZkpFlError::Io(e))?;

        let mut loaded_count = 0;
        for entry in entries {
            let entry = entry.map_err(|e| ZkpFlError::Io(e))?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_proof_from_file(&path) {
                    Ok(proof) => {
                        self.active_proofs.insert(proof.proof_id, proof);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load proof from {:?}: {}", path, e);
                    }
                }
            }
        }

        if loaded_count > 0 {
            info!("Loaded {} existing proofs from storage", loaded_count);
        }

        Ok(())
    }

    fn load_proof_from_file(&self, path: &std::path::Path) -> Result<ZkpProof> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ZkpFlError::Io(e))?;
        
        let proof: ZkpProof = serde_json::from_str(&content)
            .map_err(|e| ZkpFlError::Serialization(e))?;
        
        Ok(proof)
    }

    async fn calculate_storage_size(&self) -> u64 {
        // Calculate the total size of stored data
        let mut total_size = 0u64;
        
        // Size of in-memory proofs (approximate)
        for proof in self.active_proofs.iter() {
            total_size += proof.proof_size() as u64;
        }
        
        // Add storage overhead estimate
        total_size += (self.active_proofs.len() * 1024) as u64; // ~1KB overhead per proof

        total_size
    }

    pub async fn export_benchmark_data(&self) -> Result<MultiClientBenchmark> {
        info!("Exporting benchmark data for {} proofs", self.active_proofs.len());

        // Collect all client results
        let mut client_results = Vec::new();
        
        for proof in self.active_proofs.iter() {
            // Convert proof to benchmark result (simplified)
            let benchmark_result = BenchmarkResult::new(
                proof.session_id,
                proof.client_id.clone(),
            );
            
            client_results.push(benchmark_result);
        }

        // Calculate aggregate metrics
        let num_clients = client_results.len();
        let stats = self.get_stats();
        
        let aggregate_metrics = common::AggregateMetrics {
            avg_proof_generation_time_ms: 1000.0, // Would calculate from actual data
            min_proof_generation_time_ms: 500,
            max_proof_generation_time_ms: 2000,
            avg_proof_verification_time_ms: 200.0,
            avg_training_time_ms: 5000.0,
            total_proofs_generated: stats.total_proofs_stored,
            total_proofs_verified: stats.total_proofs_verified,
            success_rate: if stats.total_proofs_stored > 0 {
                stats.total_proofs_verified as f64 / stats.total_proofs_stored as f64
            } else {
                0.0
            },
            throughput_proofs_per_second: 0.5, // Would calculate from timing data
        };

        let benchmark = MultiClientBenchmark {
            benchmark_id: Uuid::new_v4(),
            start_time: Utc::now() - chrono::Duration::hours(1), // Estimate
            end_time: Utc::now(),
            num_clients,
            client_results,
            aggregate_metrics,
        };

        Ok(benchmark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{ServerConfig, TrainingCommitment, ProofData, CircuitParams, ProofMetadata};

    #[tokio::test]
    async fn test_proof_storage() {
        let config = ServerConfig {
            host: "localhost".to_string(),
            port: 8080,
            max_clients: 10,
            proof_storage_path: "/tmp/test_proofs".to_string(),
            log_level: "info".to_string(),
        };

        let storage = ProofStorage::new(&config, true).unwrap();
        
        // Create test proof
        let proof = create_test_proof();
        
        // Store proof
        storage.store_proof(proof.clone()).await.unwrap();
        
        // Retrieve proof
        let retrieved = storage.get_proof(&proof.proof_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().proof_id, proof.proof_id);
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

        ZkpProof::new(
            Uuid::new_v4(),
            "test_client".to_string(),
            vec![0u8; 1024],
            vec!["0.1".to_string()],
            circuit_params,
            metadata,
            training_commitment,
        )
    }
}
