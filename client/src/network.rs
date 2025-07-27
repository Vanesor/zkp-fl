use common::{
    Result, VerificationRequest, VerificationResponse, VerificationResult, ZkpFlError, ZkpProof,
};
use log::{debug, error, info, warn};
use reqwest::Client;
use std::time::Duration;
use uuid::Uuid;

pub struct NetworkClient {
    client: Client,
    server_url: String,
}

impl NetworkClient {
    pub fn new(server_url: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30)) // 30 second timeout for HTTP operations
            .build()
            .map_err(|e| ZkpFlError::Network(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            server_url: server_url.to_string(),
        })
    }

    pub async fn submit_proof(&self, proof: ZkpProof) -> Result<VerificationResult> {
        info!("Submitting proof {} to server", proof.proof_id);

        let request = VerificationRequest {
            proof: proof.clone(),
            requester_id: proof.client_id.clone(),
        };

        let url = format!("{}/api/verify", self.server_url);

        debug!("POST {}", url);
        debug!("Proof size: {} bytes", proof.proof_size());

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to send proof to server: {}", e);
                ZkpFlError::Network(format!("Failed to send request: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Server returned error {}: {}", status, error_text);
            return Err(ZkpFlError::Network(format!(
                "Server error {}: {}",
                status, error_text
            )));
        }

        let verification_response: VerificationResponse = response.json().await.map_err(|e| {
            error!("Failed to parse verification response: {}", e);
            ZkpFlError::Network(format!("Failed to parse response: {}", e))
        })?;

        info!(
            "Received verification result: verified={}, time={}ms",
            verification_response.result.verified,
            verification_response.result.verification_time_ms
        );

        Ok(verification_response.result)
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/health", self.server_url);

        match self.client.get(&url).send().await {
            Ok(response) => {
                let is_healthy = response.status().is_success();
                if is_healthy {
                    debug!("Server health check passed");
                } else {
                    warn!("Server health check failed: {}", response.status());
                }
                Ok(is_healthy)
            }
            Err(e) => {
                warn!("Server health check error: {}", e);
                Ok(false)
            }
        }
    }

    pub async fn get_server_status(&self) -> Result<ServerStatus> {
        let url = format!("{}/api/status", self.server_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ZkpFlError::Network(format!("Failed to get server status: {}", e)))?;

        if !response.status().is_success() {
            return Err(ZkpFlError::Network(format!(
                "Server status error: {}",
                response.status()
            )));
        }

        let status: ServerStatus = response
            .json()
            .await
            .map_err(|e| ZkpFlError::Network(format!("Failed to parse status: {}", e)))?;

        Ok(status)
    }

    pub async fn submit_batch_proofs(
        &self,
        proofs: Vec<ZkpProof>,
    ) -> Result<Vec<VerificationResult>> {
        info!("Submitting batch of {} proofs", proofs.len());

        let batch_request = BatchVerificationRequest {
            proofs,
            requester_id: "batch_client".to_string(),
        };

        let url = format!("{}/api/verify_batch", self.server_url);

        let response = self
            .client
            .post(&url)
            .json(&batch_request)
            .send()
            .await
            .map_err(|e| ZkpFlError::Network(format!("Failed to send batch: {}", e)))?;

        if !response.status().is_success() {
            return Err(ZkpFlError::Network(format!(
                "Batch verification error: {}",
                response.status()
            )));
        }

        let batch_response: BatchVerificationResponse = response
            .json()
            .await
            .map_err(|e| ZkpFlError::Network(format!("Failed to parse batch response: {}", e)))?;

        info!(
            "Batch verification completed: {}/{} proofs verified",
            batch_response.results.iter().filter(|r| r.verified).count(),
            batch_response.results.len()
        );

        Ok(batch_response.results)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ServerStatus {
    pub uptime_seconds: u64,
    pub active_clients: usize,
    pub total_proofs_processed: usize,
    pub total_proofs_verified: usize,
    pub average_verification_time_ms: f64,
    pub server_version: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BatchVerificationRequest {
    pub proofs: Vec<ZkpProof>,
    pub requester_id: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BatchVerificationResponse {
    pub results: Vec<VerificationResult>,
    pub batch_id: Uuid,
    pub total_verification_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_client_creation() {
        let client = NetworkClient::new("http://localhost:8080");
        assert!(client.is_ok());
    }

    // Note: Integration tests would require a running server
}
