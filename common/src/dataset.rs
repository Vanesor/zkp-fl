use crate::{Result, ZkpFlError, Sample};
use csv::Reader;
use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

/// Healthcare dataset structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcareDataset {
    pub name: String,
    pub description: String,
    pub features: Array2<f64>,
    pub targets: Array1<f64>,
    pub feature_names: Vec<String>,
    pub target_name: String,
    pub num_samples: usize,
    pub num_features: usize,
}

/// Dataset record from CSV
#[derive(Debug, Deserialize)]
pub struct DatasetRecord {
    // Common healthcare features
    pub age: Option<f64>,
    pub sex: Option<f64>,
    pub chest_pain_type: Option<f64>,
    pub resting_bp: Option<f64>,
    pub cholesterol: Option<f64>,
    pub fasting_blood_sugar: Option<f64>,
    pub resting_ecg: Option<f64>,
    pub max_heart_rate: Option<f64>,
    pub exercise_angina: Option<f64>,
    pub st_depression: Option<f64>,
    pub st_slope: Option<f64>,
    pub vessels_colored: Option<f64>,
    pub thalassemia: Option<f64>,
    pub target: Option<f64>,
}

impl HealthcareDataset {
    /// Load dataset from CSV file
    pub fn load_from_csv<P: AsRef<Path>>(
        path: P,
        target_column: &str,
        feature_columns: &[String],
    ) -> Result<Self> {
        let file = File::open(&path)
            .map_err(|e| ZkpFlError::Dataset(format!("Failed to open file: {}", e)))?;
        
        let mut reader = Reader::from_reader(file);
        let mut records = Vec::new();
        
        for result in reader.deserialize() {
            let record: DatasetRecord = result
                .map_err(|e| ZkpFlError::Dataset(format!("Failed to parse record: {}", e)))?;
            records.push(record);
        }

        if records.is_empty() {
            return Err(ZkpFlError::Dataset("No records found in dataset".to_string()));
        }

        let num_samples = records.len();
        let num_features = feature_columns.len();
        
        let mut features = Array2::zeros((num_samples, num_features));
        let mut targets = Array1::zeros(num_samples);
        
        for (i, record) in records.iter().enumerate() {
            // Extract target value
            targets[i] = record.target.unwrap_or(0.0);
            
            // Extract feature values based on feature_columns
            for (j, feature_name) in feature_columns.iter().enumerate() {
                let value = match feature_name.as_str() {
                    "age" => record.age.unwrap_or(0.0),
                    "sex" => record.sex.unwrap_or(0.0),
                    "chest_pain_type" => record.chest_pain_type.unwrap_or(0.0),
                    "resting_bp" => record.resting_bp.unwrap_or(0.0),
                    "cholesterol" => record.cholesterol.unwrap_or(0.0),
                    "fasting_blood_sugar" => record.fasting_blood_sugar.unwrap_or(0.0),
                    "resting_ecg" => record.resting_ecg.unwrap_or(0.0),
                    "max_heart_rate" => record.max_heart_rate.unwrap_or(0.0),
                    "exercise_angina" => record.exercise_angina.unwrap_or(0.0),
                    "st_depression" => record.st_depression.unwrap_or(0.0),
                    "st_slope" => record.st_slope.unwrap_or(0.0),
                    "vessels_colored" => record.vessels_colored.unwrap_or(0.0),
                    "thalassemia" => record.thalassemia.unwrap_or(0.0),
                    _ => 0.0,
                };
                features[(i, j)] = value;
            }
        }

        Ok(Self {
            name: "Healthcare Dataset".to_string(),
            description: "Open source healthcare dataset for ZKP linear regression".to_string(),
            features,
            targets,
            feature_names: feature_columns.to_vec(),
            target_name: target_column.to_string(),
            num_samples,
            num_features,
        })
    }

    /// Create a synthetic healthcare dataset for testing
    pub fn create_synthetic(num_samples: usize, num_features: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let mut features = Array2::zeros((num_samples, num_features));
        let mut targets = Array1::zeros(num_samples);
        
        // Generate synthetic healthcare-like data
        for i in 0..num_samples {
            for j in 0..num_features {
                match j {
                    0 => features[(i, j)] = rng.gen_range(20.0..80.0), // age
                    1 => features[(i, j)] = rng.gen_range(0.0..2.0), // sex
                    2 => features[(i, j)] = rng.gen_range(90.0..200.0), // blood pressure
                    3 => features[(i, j)] = rng.gen_range(100.0..400.0), // cholesterol
                    4 => features[(i, j)] = rng.gen_range(60.0..200.0), // heart rate
                    _ => features[(i, j)] = rng.gen_range(0.0..10.0),
                }
            }
            
            // Generate target based on features with some noise
            targets[i] = 0.5 * features[(i, 0)] / 80.0 + 
                        0.3 * features[(i, 2)] / 200.0 + 
                        0.2 * features[(i, 3)] / 400.0 + 
                        rng.gen_range(-0.1..0.1); // noise
        }

        let feature_names = (0..num_features)
            .map(|i| match i {
                0 => "age".to_string(),
                1 => "sex".to_string(),
                2 => "blood_pressure".to_string(),
                3 => "cholesterol".to_string(),
                4 => "heart_rate".to_string(),
                _ => format!("feature_{}", i),
            })
            .collect();

        Self {
            name: "Synthetic Healthcare Dataset".to_string(),
            description: "Synthetically generated healthcare dataset for testing".to_string(),
            features,
            targets,
            feature_names,
            target_name: "risk_score".to_string(),
            num_samples,
            num_features,
        }
    }

    /// Normalize features to [0, 1] range
    pub fn normalize(&mut self) {
        for j in 0..self.num_features {
            let column = self.features.column(j);
            let min_val = column.iter().copied().fold(f64::INFINITY, f64::min);
            let max_val = column.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            
            if max_val > min_val {
                for i in 0..self.num_samples {
                    self.features[(i, j)] = (self.features[(i, j)] - min_val) / (max_val - min_val);
                }
            }
        }
    }

    /// Split dataset into training and testing sets
    pub fn train_test_split(&self, train_ratio: f64) -> (Self, Self) {
        assert!(train_ratio > 0.0 && train_ratio < 1.0);
        
        let train_size = (self.num_samples as f64 * train_ratio) as usize;
        
        let train_features = self.features.slice(s![0..train_size, ..]).to_owned();
        let train_targets = self.targets.slice(s![0..train_size]).to_owned();
        
        let test_features = self.features.slice(s![train_size.., ..]).to_owned();
        let test_targets = self.targets.slice(s![train_size..]).to_owned();
        
        let train_dataset = Self {
            name: format!("{} (Train)", self.name),
            description: format!("{} - Training set", self.description),
            features: train_features,
            targets: train_targets,
            feature_names: self.feature_names.clone(),
            target_name: self.target_name.clone(),
            num_samples: train_size,
            num_features: self.num_features,
        };
        
        let test_dataset = Self {
            name: format!("{} (Test)", self.name),
            description: format!("{} - Test set", self.description),
            features: test_features,
            targets: test_targets,
            feature_names: self.feature_names.clone(),
            target_name: self.target_name.clone(),
            num_samples: self.num_samples - train_size,
            num_features: self.num_features,
        };
        
        (train_dataset, test_dataset)
    }

    /// Convert to training samples
    pub fn to_samples(&self) -> Vec<Sample> {
        (0..self.num_samples)
            .map(|i| Sample {
                features: self.features.row(i).to_vec(),
                target: self.targets[i],
            })
            .collect()
    }

    /// Get batches for training
    pub fn get_batches(&self, batch_size: usize) -> Vec<Vec<Sample>> {
        let samples = self.to_samples();
        samples
            .chunks(batch_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }
}

use ndarray::s;
