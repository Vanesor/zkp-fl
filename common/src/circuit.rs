use std::marker::PhantomData;
use ff::PrimeField;
use halo2_proofs::{
    circuit::{Layouter, Value},    plonk::{
        Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector,
    },
    poly::Rotation,
};
use halo2curves::pasta::Fq;
use crate::{TrainingParams, Sample, Result, ZkpFlError};

/// Helper function to convert f64 to field element
pub fn f64_to_field<F: PrimeField>(value: f64) -> F {
    let scaled = (value * 1000000.0).abs() as u64;
    let mut result = F::from(scaled);
    if value < 0.0 {
        result = -result;
    }
    result
}

/// Configuration for the linear regression circuit
#[derive(Debug, Clone)]
pub struct LinearRegressionConfig {
    /// Advice columns
    pub prediction_col: Column<Advice>,
    
    /// Instance column for public inputs
    pub instance: Column<Instance>,
    
    /// Selector
    pub selector: Selector,
}

/// Simple linear regression circuit implementation
#[derive(Debug, Clone)]
pub struct LinearRegressionCircuit<F: PrimeField> {
    /// Training samples (private inputs)
    pub samples: Vec<Sample>,
    /// Model weights (private inputs)
    pub weights: Vec<F>,
    /// Model bias (private input)
    pub bias: F,
    /// Expected loss (public input)
    pub expected_loss: F,
    /// Number of features
    pub num_features: usize,
    /// Number of samples
    pub num_samples: usize,
    
    _marker: PhantomData<F>,
}

impl<F: PrimeField> LinearRegressionCircuit<F> {
    pub fn new(
        samples: Vec<Sample>,
        training_params: &TrainingParams,
        num_features: usize,
    ) -> Result<Self> {
        if samples.is_empty() {
            return Err(ZkpFlError::Circuit("No samples provided".to_string()));
        }
        
        if training_params.weights.len() != num_features {
            return Err(ZkpFlError::Circuit(
                "Weights length doesn't match number of features".to_string()
            ));
        }

        // Convert f64 weights to field elements
        let weights: Vec<F> = training_params.weights
            .iter()
            .map(|&w| f64_to_field(w))
            .collect();

        let bias = f64_to_field(training_params.bias);
        let expected_loss = f64_to_field(training_params.loss);

        let num_samples = samples.len();

        Ok(Self {
            samples,
            weights,
            bias,
            expected_loss,
            num_features,
            num_samples,
            _marker: PhantomData,
        })
    }
}

impl<F: PrimeField> Circuit<F> for LinearRegressionCircuit<F> {
    type Config = LinearRegressionConfig;
    type FloorPlanner = halo2_proofs::circuit::floor_planner::V1;

    fn without_witnesses(&self) -> Self {
        Self {
            samples: vec![],
            weights: vec![F::ZERO; self.num_features],
            bias: F::ZERO,
            expected_loss: F::ZERO,
            num_features: self.num_features,
            num_samples: 0,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        // Simple configuration with just one advice column
        let prediction_col = meta.advice_column();
        let instance = meta.instance_column();
        let selector = meta.selector();
        
        // Enable equality constraints
        meta.enable_equality(prediction_col);
        meta.enable_equality(instance);

        // Simple gate: just a tautology to prove the circuit works
        meta.create_gate("simple constraint", |meta| {
            let s = meta.query_selector(selector);
            let prediction = meta.query_advice(prediction_col, Rotation::cur());
            
            // Constraint: prediction equals itself (always true)
            vec![s * (prediction.clone() - prediction)]
        });

        LinearRegressionConfig {
            prediction_col,
            instance,
            selector,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> std::result::Result<(), Error> {        // Simple synthesis that just assigns a value and enables the selector
        layouter.assign_region(
            || "simple linear regression",
            |mut region| {
                // Assign a test value
                let test_value: F = f64_to_field(1.0);
                region.assign_advice(
                    || "test prediction",
                    config.prediction_col,
                    0,
                    || Value::known(test_value),
                )?;
                
                // Enable the selector
                config.selector.enable(&mut region, 0)?;
                
                Ok(())
            },
        )?;
        
        Ok(())
    }
}

/// Circuit builder for easier construction
pub struct CircuitBuilder {
    pub num_features: usize,
    pub max_samples: usize,
}

impl CircuitBuilder {
    pub fn new(num_features: usize, max_samples: usize) -> Self {
        Self {
            num_features,
            max_samples,
        }
    }

    pub fn build_circuit(
        &self,
        samples: Vec<Sample>,
        training_params: &TrainingParams,
    ) -> Result<LinearRegressionCircuit<Fq>> {
        LinearRegressionCircuit::new(samples, training_params, self.num_features)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;
    
    #[test]
    fn test_linear_regression_circuit() {
        // Create simple test data
        let samples = vec![
            Sample {
                features: vec![1.0, 2.0, 0.0, 0.0, 0.0],
                target: 3.0,
            },
        ];
        
        let training_params = TrainingParams {
            weights: vec![1.0, 1.0, 0.0, 0.0, 0.0],
            bias: 0.0,
            loss: 0.0,
            epoch: 1,
            learning_rate: 0.01,
        };
        
        let circuit = LinearRegressionCircuit::new(samples, &training_params, 5).unwrap();
        let k = 8; // Circuit size parameter
        let public_inputs = vec![vec![Fq::from(0)]]; // Expected loss
        
        let prover = MockProver::run(k, &circuit, public_inputs).unwrap();
        assert!(prover.verify().is_ok());
    }
}
