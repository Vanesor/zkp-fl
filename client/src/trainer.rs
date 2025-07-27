use common::{
    HealthcareDataset, Sample, TrainingParams, TrainingMetrics, 
    CircuitConfig, DatasetConfig, Result, ZkpFlError
};
use log::{info, debug};
use std::time::Instant;

pub struct Trainer {
    dataset: Option<HealthcareDataset>,
    training_params: Option<TrainingParams>,
    circuit_config: CircuitConfig,
    dataset_config: DatasetConfig,
}

impl Trainer {
    pub fn new(circuit_config: &CircuitConfig, dataset_config: &DatasetConfig) -> Result<Self> {
        Ok(Self {
            dataset: None,
            training_params: None,
            circuit_config: circuit_config.clone(),
            dataset_config: dataset_config.clone(),
        })
    }

    pub fn set_dataset(&mut self, mut dataset: HealthcareDataset) -> Result<()> {
        info!("Setting dataset with {} samples, {} features", 
              dataset.num_samples, dataset.num_features);
        
        // Normalize if configured
        if self.dataset_config.normalize {
            debug!("Normalizing dataset features");
            dataset.normalize();
        }
        
        // Validate dataset size
        if dataset.num_features != self.circuit_config.num_features {
            return Err(ZkpFlError::Dataset(format!(
                "Dataset features ({}) don't match circuit config ({})",
                dataset.num_features, self.circuit_config.num_features
            )));
        }
        
        self.dataset = Some(dataset);
        Ok(())
    }

    pub async fn train(&mut self, epochs: usize) -> Result<TrainingMetrics> {
        let dataset = self.dataset.as_ref()
            .ok_or_else(|| ZkpFlError::Dataset("No dataset loaded".to_string()))?;
        
        info!("Starting training for {} epochs", epochs);
        let start_time = Instant::now();
        
        // Initialize weights and bias
        let num_features = dataset.num_features;
        let mut weights = vec![0.01; num_features]; // Small random initialization
        let mut bias = 0.0;
        let learning_rate = 0.01;
        
        let mut loss_history = Vec::new();
        let mut initial_loss = None;
        let mut convergence_epoch = None;
        let convergence_threshold = 1e-6;
        
        // Split dataset into train/test
        let (train_dataset, _test_dataset) = dataset.train_test_split(0.8);
        let samples = train_dataset.to_samples();
        
        info!("Training on {} samples", samples.len());
        
        // Training loop
        for epoch in 0..epochs {
            let epoch_start = Instant::now();
            
            // Forward pass and gradient computation
            let (loss, gradients) = self.compute_gradients(&samples, &weights, bias)?;
            loss_history.push(loss);
            
            if initial_loss.is_none() {
                initial_loss = Some(loss);
            }
            
            // Update weights and bias using gradient descent
            for (weight, gradient) in weights.iter_mut().zip(gradients.iter()) {
                *weight -= learning_rate * gradient;
            }
            
            // Update bias (gradient for bias is mean of residuals)
            let bias_gradient: f64 = samples.iter()
                .map(|sample| {
                    let prediction = self.predict(&sample.features, &weights, bias);
                    prediction - sample.target
                })
                .sum::<f64>() / samples.len() as f64;
            
            bias -= learning_rate * bias_gradient;
            
            // Check for convergence
            if epoch > 0 {
                let loss_change = (loss_history[epoch - 1] - loss).abs();
                if loss_change < convergence_threshold && convergence_epoch.is_none() {
                    convergence_epoch = Some(epoch);
                    info!("Converged at epoch {}", epoch);
                }
            }
            
            if epoch % 10 == 0 || epoch == epochs - 1 {
                debug!("Epoch {}: loss = {:.6}, took {}ms", 
                       epoch, loss, epoch_start.elapsed().as_millis());
            }
        }
        
        let training_time = start_time.elapsed();
        let final_loss = loss_history.last().copied().unwrap_or(0.0);
        
        // Store training parameters
        self.training_params = Some(TrainingParams {
            weights: weights.clone(),
            bias,
            loss: final_loss,
            epoch: epochs,
            learning_rate,
        });
        
        let metrics = TrainingMetrics {
            dataset_size: samples.len(),
            num_features,
            training_time_ms: training_time.as_millis() as u64,
            epochs_completed: epochs,
            final_loss,
            initial_loss: initial_loss.unwrap_or(0.0),
            convergence_epoch,
            loss_history,
        };
        
        info!("Training completed: {} epochs, final loss: {:.6}, time: {}ms",
              epochs, final_loss, training_time.as_millis());
        
        Ok(metrics)
    }

    fn compute_gradients(&self, samples: &[Sample], weights: &[f64], bias: f64) -> Result<(f64, Vec<f64>)> {
        let n = samples.len() as f64;
        let mut gradients = vec![0.0; weights.len()];
        let mut total_loss = 0.0;
        
        for sample in samples {
            let prediction = self.predict(&sample.features, weights, bias);
            let residual = prediction - sample.target;
            
            // Squared loss
            total_loss += residual * residual;
            
            // Gradients for weights
            for (i, &feature) in sample.features.iter().enumerate() {
                gradients[i] += 2.0 * residual * feature / n;
            }
        }
        
        let mse_loss = total_loss / n;
        Ok((mse_loss, gradients))
    }

    fn predict(&self, features: &[f64], weights: &[f64], bias: f64) -> f64 {
        let mut prediction = bias;
        for (feature, weight) in features.iter().zip(weights.iter()) {
            prediction += feature * weight;
        }
        prediction
    }

    pub fn get_training_params(&self) -> Result<TrainingParams> {
        self.training_params.clone()
            .ok_or_else(|| ZkpFlError::Dataset("No training completed".to_string()))
    }

    pub fn get_training_samples(&self) -> Result<Vec<Sample>> {
        let dataset = self.dataset.as_ref()
            .ok_or_else(|| ZkpFlError::Dataset("No dataset loaded".to_string()))?;
        
        // Return a subset of samples for circuit (to fit in circuit constraints)
        let max_samples = std::cmp::min(dataset.num_samples, 100); // Limit for circuit
        Ok(dataset.to_samples().into_iter().take(max_samples).collect())
    }

    pub fn get_dataset_size(&self) -> usize {
        self.dataset.as_ref().map(|d| d.num_samples).unwrap_or(0)
    }

    pub fn get_num_features(&self) -> usize {
        self.dataset.as_ref().map(|d| d.num_features).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{CircuitConfig, DatasetConfig};

    #[tokio::test]
    async fn test_trainer_synthetic_data() {
        let circuit_config = CircuitConfig {
            k: 8,
            num_features: 3,
            precision_bits: 32,
            max_iterations: 100,
        };
        
        let dataset_config = DatasetConfig {
            path: "synthetic".to_string(),
            target_column: "target".to_string(),
            feature_columns: vec!["f1".to_string(), "f2".to_string(), "f3".to_string()],
            train_test_split: 0.8,
            normalize: true,
        };
        
        let mut trainer = Trainer::new(&circuit_config, &dataset_config).unwrap();
        
        // Create synthetic dataset
        let dataset = HealthcareDataset::create_synthetic(100, 3);
        trainer.set_dataset(dataset).unwrap();
        
        // Train model
        let metrics = trainer.train(10).await.unwrap();
        
        assert!(metrics.epochs_completed == 10);
        assert!(metrics.final_loss >= 0.0);
        assert!(metrics.training_time_ms > 0);
        
        // Get training parameters
        let params = trainer.get_training_params().unwrap();
        assert_eq!(params.weights.len(), 3);
    }
}
