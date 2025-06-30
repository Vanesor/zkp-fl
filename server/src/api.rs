use common::{
    ZkpProof, VerificationRequest, VerificationResponse, VerificationResult
};
use crate::{verifier::ProofVerifier, storage::ProofStorage, metrics::ServerMetrics};
use warp::{Filter, Reply, Rejection, reject};
use std::sync::Arc;
use std::convert::Infallible;
use log::{info, debug, error};
use serde_json;
use uuid::Uuid;
use chrono::Utc;

pub fn create_api_routes(
    verifier: Arc<ProofVerifier>,
    storage: Arc<ProofStorage>,
    metrics: Arc<ServerMetrics>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let health = health_route();
    let status = status_route(metrics.clone());
    let verify = verify_route(verifier.clone(), storage.clone(), metrics.clone());
    let verify_batch = verify_batch_route(verifier.clone(), storage.clone(), metrics.clone());
    let proofs = proofs_route(storage.clone());
    let benchmarks = benchmarks_route(storage.clone());
    let cleanup = cleanup_route(storage.clone());    let api = warp::path("api").and(
        health
            .or(status)
            .or(verify)
            .or(verify_batch)
            .or(proofs)
            .or(benchmarks)
            .or(cleanup)
    );

    api
}

fn health_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("health")
        .and(warp::get())
        .map(|| {
            debug!("Health check requested");
            warp::reply::json(&serde_json::json!({
                "status": "healthy",
                "timestamp": Utc::now(),
                "service": "zkp-fl-server"
            }))
        })
}

fn status_route(
    metrics: Arc<ServerMetrics>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("status")
        .and(warp::get())
        .map(move || {
            debug!("Status requested");
            let _snapshot = metrics.get_current_snapshot();            warp::reply::json(&serde_json::json!({
                "active_clients": 0,
                "total_proofs_verified": 0,
                "uptime_seconds": 0,
                "server_version": "1.0.0"
            }))
        })
}

fn verify_route(
    verifier: Arc<ProofVerifier>,
    storage: Arc<ProofStorage>,
    metrics: Arc<ServerMetrics>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("verify")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_verifier(verifier))
        .and(with_storage(storage))
        .and(with_metrics(metrics))
        .and_then(handle_verify_proof)
}

fn verify_batch_route(
    verifier: Arc<ProofVerifier>,
    storage: Arc<ProofStorage>,
    metrics: Arc<ServerMetrics>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("verify_batch")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_verifier(verifier))
        .and(with_storage(storage))
        .and(with_metrics(metrics))
        .and_then(handle_verify_batch)
}

fn proofs_route(
    storage: Arc<ProofStorage>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let get_all = warp::path("proofs")
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(handle_get_all_proofs);

    let get_by_id = warp::path("proofs")
        .and(warp::path::param::<String>())
        .and(warp::get())
        .and(with_storage(storage.clone()))
        .and_then(handle_get_proof_by_id);

    let get_by_client = warp::path("proofs")
        .and(warp::path("client"))
        .and(warp::path::param::<String>())
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(handle_get_proofs_by_client);

    get_all.or(get_by_id).or(get_by_client)
}

fn benchmarks_route(
    storage: Arc<ProofStorage>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("benchmarks")
        .and(warp::path("export"))
        .and(warp::get())
        .and(with_storage(storage))
        .and_then(handle_export_benchmarks)
}

fn cleanup_route(
    storage: Arc<ProofStorage>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("admin")
        .and(warp::path("cleanup"))
        .and(warp::post())
        .and(warp::query::<CleanupParams>())
        .and(with_storage(storage))
        .and_then(handle_cleanup)
}

// Helper functions for dependency injection
fn with_verifier(
    verifier: Arc<ProofVerifier>,
) -> impl Filter<Extract = (Arc<ProofVerifier>,), Error = Infallible> + Clone {
    warp::any().map(move || verifier.clone())
}

fn with_storage(
    storage: Arc<ProofStorage>,
) -> impl Filter<Extract = (Arc<ProofStorage>,), Error = Infallible> + Clone {
    warp::any().map(move || storage.clone())
}

fn with_metrics(
    metrics: Arc<ServerMetrics>,
) -> impl Filter<Extract = (Arc<ServerMetrics>,), Error = Infallible> + Clone {
    warp::any().map(move || metrics.clone())
}

// Handler functions
async fn handle_verify_proof(
    request: VerificationRequest,
    verifier: Arc<ProofVerifier>,
    storage: Arc<ProofStorage>,
    metrics: Arc<ServerMetrics>,
) -> Result<impl Reply, Rejection> {
    info!("Received proof verification request from {}", request.requester_id);
    
    metrics.increment_proof_requests().await;
      // Verify the proof
    let verifier_ref = verifier.as_ref();
    let verification_result = unsafe {
        // SAFETY: We need mutable access for stats updates
        let verifier_ptr = verifier_ref as *const ProofVerifier as *mut ProofVerifier;
        (*verifier_ptr).verify_proof(&request.proof).await
    };

    match verification_result {
        Ok(result) => {
            // Store the proof with verification result
            let mut proof_with_result = request.proof.clone();
            proof_with_result.mark_verified(result.clone());
            
            if let Err(e) = storage.store_proof(proof_with_result).await {
                error!("Failed to store proof: {}", e);
            }

            metrics.record_verification_result(&result).await;

            let response = VerificationResponse {
                proof_id: request.proof.proof_id,
                result,
            };

            Ok(warp::reply::json(&response))
        }
        Err(e) => {
            error!("Proof verification failed: {}", e);
            metrics.increment_verification_errors().await;
            Err(reject::custom(ApiError::VerificationError(e.to_string())))
        }
    }
}

async fn handle_verify_batch(
    _request: Vec<u8>, // Simplified - just accept raw bytes for now
    _verifier: Arc<ProofVerifier>,
    _storage: Arc<ProofStorage>,
    _metrics: Arc<ServerMetrics>,
) -> Result<impl Reply, Rejection> {
    // Simplified implementation
    let response = serde_json::json!({
        "results": [],
        "total_time_ms": 0
    });
    Ok(warp::reply::json(&response))
}

async fn handle_get_all_proofs(
    storage: Arc<ProofStorage>,
) -> Result<impl Reply, Rejection> {
    debug!("Retrieving all proofs");
    
    let proofs = storage.get_all_proofs().await;
    Ok(warp::reply::json(&proofs))
}

async fn handle_get_proof_by_id(
    proof_id: String,
    storage: Arc<ProofStorage>,
) -> Result<impl Reply, Rejection> {
    debug!("Retrieving proof by ID: {}", proof_id);
    
    let uuid = Uuid::parse_str(&proof_id)
        .map_err(|_| reject::custom(ApiError::InvalidProofId))?;
    
    match storage.get_proof(&uuid).await {
        Some(proof) => Ok(warp::reply::json(&proof)),
        None => Err(reject::custom(ApiError::ProofNotFound)),
    }
}

async fn handle_get_proofs_by_client(
    client_id: String,
    storage: Arc<ProofStorage>,
) -> Result<impl Reply, Rejection> {
    debug!("Retrieving proofs for client: {}", client_id);
    
    let proofs = storage.get_client_proofs(&client_id).await;
    Ok(warp::reply::json(&proofs))
}

async fn handle_export_benchmarks(
    storage: Arc<ProofStorage>,
) -> Result<impl Reply, Rejection> {
    info!("Exporting benchmark data");
    
    let benchmark_data = storage.export_benchmark_data().await
        .map_err(|e| reject::custom(ApiError::BenchmarkError(e.to_string())))?;
    
    Ok(warp::reply::json(&benchmark_data))
}

#[derive(serde::Deserialize)]
struct CleanupParams {
    max_age_hours: Option<i64>,
}

async fn handle_cleanup(
    params: CleanupParams,
    storage: Arc<ProofStorage>,
) -> Result<impl Reply, Rejection> {
    let max_age = params.max_age_hours.unwrap_or(24);
    info!("Cleaning up proofs older than {} hours", max_age);
    
    let removed_count = storage.cleanup_old_proofs(max_age).await
        .map_err(|e| reject::custom(ApiError::CleanupError(e.to_string())))?;
    
    Ok(warp::reply::json(&serde_json::json!({
        "removed_count": removed_count,
        "max_age_hours": max_age
    })))
}

// Error handling
#[derive(Debug)]
enum ApiError {
    VerificationError(String),
    StorageError(String),
    BenchmarkError(String),
    CleanupError(String),
    InvalidProofId,
    ProofNotFound,
    BatchTooLarge,
}

impl reject::Reject for ApiError {}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "Not Found";
    } else if let Some(api_error) = err.find::<ApiError>() {
        match api_error {
            ApiError::VerificationError(msg) => {
                code = warp::http::StatusCode::BAD_REQUEST;
                message = msg;
            }
            ApiError::StorageError(msg) => {
                code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
                message = msg;
            }
            ApiError::BenchmarkError(msg) => {
                code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
                message = msg;
            }
            ApiError::CleanupError(msg) => {
                code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
                message = msg;
            }
            ApiError::InvalidProofId => {
                code = warp::http::StatusCode::BAD_REQUEST;
                message = "Invalid proof ID format";
            }
            ApiError::ProofNotFound => {
                code = warp::http::StatusCode::NOT_FOUND;
                message = "Proof not found";
            }
            ApiError::BatchTooLarge => {
                code = warp::http::StatusCode::BAD_REQUEST;
                message = "Batch size too large (max 100 proofs)";
            }
        }
    } else if err.find::<warp::filters::body::BodyDeserializeError>().is_some() {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "Invalid request body";
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "Method Not Allowed";
    } else {
        error!("Unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error";
    }

    let json = warp::reply::json(&serde_json::json!({
        "error": message,
        "code": code.as_u16()
    }));

    Ok(warp::reply::with_status(json, code))
}

// Import network types that are used in this file
mod network {
    use super::*;
    
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
}
