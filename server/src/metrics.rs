use common::{VerificationResult, SystemMetrics};
use parking_lot::RwLock;
use log::{debug, info};
use std::time::Instant;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;

pub struct ServerMetrics {
    start_time: Instant,
    stats: RwLock<ServerStats>,
    verification_history: RwLock<VecDeque<VerificationRecord>>,
}

#[derive(Debug, Clone)]
pub struct ServerStats {
    pub total_proof_requests: usize,
    pub total_proofs_processed: usize,
    pub total_proofs_verified: usize,
    pub total_verification_errors: usize,
    pub active_clients: usize,
    pub total_verification_time_ms: u64,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ServerMetricsSnapshot {
    pub uptime_seconds: u64,
    pub active_clients: usize,
    pub total_proofs_processed: usize,
    pub total_proofs_verified: usize,
    pub total_verification_errors: usize,
    pub average_verification_time_ms: f64,
    pub verification_success_rate: f64,
    pub current_load: f64,
    pub system_metrics: SystemMetrics,
}

#[derive(Debug, Clone)]
struct VerificationRecord {
    timestamp: DateTime<Utc>,
    verification_time_ms: u64,
    success: bool,
    client_id: String,
}

impl ServerMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            stats: RwLock::new(ServerStats {
                total_proof_requests: 0,
                total_proofs_processed: 0,
                total_proofs_verified: 0,
                total_verification_errors: 0,
                active_clients: 0,
                total_verification_time_ms: 0,
                last_update: Utc::now(),
            }),
            verification_history: RwLock::new(VecDeque::new()),
        }
    }

    pub async fn increment_proof_requests(&self) {
        let mut stats = self.stats.write();
        stats.total_proof_requests += 1;
        stats.last_update = Utc::now();
        
        debug!("Total proof requests: {}", stats.total_proof_requests);
    }

    pub async fn record_verification_result(&self, result: &VerificationResult) {
        let mut stats = self.stats.write();
        stats.total_proofs_processed += 1;
        stats.total_verification_time_ms += result.verification_time_ms;
        
        if result.verified {
            stats.total_proofs_verified += 1;
        }
        
        stats.last_update = Utc::now();

        // Add to verification history
        let mut history = self.verification_history.write();
        history.push_back(VerificationRecord {
            timestamp: result.verification_timestamp,
            verification_time_ms: result.verification_time_ms,
            success: result.verified,
            client_id: result.verifier_id.clone(),
        });

        // Keep only last 1000 records
        if history.len() > 1000 {
            history.pop_front();
        }

        debug!("Recorded verification: verified={}, time={}ms", 
               result.verified, result.verification_time_ms);
    }

    pub async fn increment_verification_errors(&self) {
        let mut stats = self.stats.write();
        stats.total_verification_errors += 1;
        stats.last_update = Utc::now();
        
        debug!("Total verification errors: {}", stats.total_verification_errors);
    }

    pub async fn set_active_clients(&self, count: usize) {
        let mut stats = self.stats.write();
        stats.active_clients = count;
        stats.last_update = Utc::now();
        
        debug!("Active clients: {}", count);
    }

    pub fn get_current_snapshot(&self) -> ServerMetricsSnapshot {
        let stats = self.stats.read();
        let uptime = self.start_time.elapsed().as_secs();

        let average_verification_time_ms = if stats.total_proofs_processed > 0 {
            stats.total_verification_time_ms as f64 / stats.total_proofs_processed as f64
        } else {
            0.0
        };

        let verification_success_rate = if stats.total_proofs_processed > 0 {
            stats.total_proofs_verified as f64 / stats.total_proofs_processed as f64
        } else {
            0.0
        };

        // Calculate current load based on recent activity
        let current_load = self.calculate_current_load();

        ServerMetricsSnapshot {
            uptime_seconds: uptime,
            active_clients: stats.active_clients,
            total_proofs_processed: stats.total_proofs_processed,
            total_proofs_verified: stats.total_proofs_verified,
            total_verification_errors: stats.total_verification_errors,
            average_verification_time_ms,
            verification_success_rate,
            current_load,
            system_metrics: SystemMetrics::current(), // This would collect actual system metrics
        }
    }

    fn calculate_current_load(&self) -> f64 {
        let history = self.verification_history.read();
        let now = Utc::now();
        let one_minute_ago = now - chrono::Duration::minutes(1);

        // Count verifications in the last minute
        let recent_verifications = history
            .iter()
            .filter(|record| record.timestamp >= one_minute_ago)
            .count();

        // Calculate load as verifications per second
        recent_verifications as f64 / 60.0
    }

    pub fn get_verification_history(&self, minutes: i64) -> Vec<VerificationRecord> {
        let history = self.verification_history.read();
        let cutoff = Utc::now() - chrono::Duration::minutes(minutes);

        history
            .iter()
            .filter(|record| record.timestamp >= cutoff)
            .cloned()
            .collect()
    }

    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        let stats = self.stats.read();
        let history = self.verification_history.read();

        // Calculate percentiles from recent verification times
        let mut recent_times: Vec<u64> = history
            .iter()
            .take(100) // Last 100 verifications
            .map(|r| r.verification_time_ms)
            .collect();
        
        recent_times.sort_unstable();

        let p50 = percentile(&recent_times, 0.5);
        let p95 = percentile(&recent_times, 0.95);
        let p99 = percentile(&recent_times, 0.99);

        PerformanceMetrics {
            average_verification_time_ms: if stats.total_proofs_processed > 0 {
                stats.total_verification_time_ms as f64 / stats.total_proofs_processed as f64
            } else {
                0.0
            },
            p50_verification_time_ms: p50,
            p95_verification_time_ms: p95,
            p99_verification_time_ms: p99,
            throughput_proofs_per_second: self.calculate_current_load(),
            error_rate: if stats.total_proofs_processed > 0 {
                stats.total_verification_errors as f64 / stats.total_proofs_processed as f64
            } else {
                0.0
            },
        }
    }

    pub async fn log_periodic_status(&self) {
        let snapshot = self.get_current_snapshot();
        
        info!("=== Server Status ===");
        info!("Uptime: {}s", snapshot.uptime_seconds);
        info!("Active clients: {}", snapshot.active_clients);
        info!("Proofs processed: {} (verified: {})", 
              snapshot.total_proofs_processed, snapshot.total_proofs_verified);
        info!("Success rate: {:.1}%", snapshot.verification_success_rate * 100.0);
        info!("Avg verification time: {:.2}ms", snapshot.average_verification_time_ms);
        info!("Current load: {:.2} proofs/sec", snapshot.current_load);
        info!("====================");
    }

    pub fn reset_metrics(&self) {
        let mut stats = self.stats.write();
        *stats = ServerStats {
            total_proof_requests: 0,
            total_proofs_processed: 0,
            total_proofs_verified: 0,
            total_verification_errors: 0,
            active_clients: 0,
            total_verification_time_ms: 0,
            last_update: Utc::now(),
        };

        let mut history = self.verification_history.write();
        history.clear();

        info!("Server metrics reset");
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PerformanceMetrics {
    pub average_verification_time_ms: f64,
    pub p50_verification_time_ms: f64,
    pub p95_verification_time_ms: f64,
    pub p99_verification_time_ms: f64,
    pub throughput_proofs_per_second: f64,
    pub error_rate: f64,
}

fn percentile(sorted_data: &[u64], p: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }
    
    let index = (sorted_data.len() as f64 * p) as usize;
    let index = index.min(sorted_data.len() - 1);
    sorted_data[index] as f64
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_metrics() {
        let metrics = ServerMetrics::new();
        
        // Test initial state
        let snapshot = metrics.get_current_snapshot();
        assert_eq!(snapshot.total_proofs_processed, 0);
        assert_eq!(snapshot.total_proofs_verified, 0);
        
        // Test incrementing requests
        metrics.increment_proof_requests().await;
        let snapshot = metrics.get_current_snapshot();
        assert_eq!(snapshot.total_proofs_processed, 0); // Should still be 0, only requests incremented
        
        // Test recording verification result
        let verification_result = VerificationResult {
            verified: true,
            verification_time_ms: 100,
            verifier_id: "test".to_string(),
            verification_timestamp: Utc::now(),
            error_message: None,
        };
        
        metrics.record_verification_result(&verification_result).await;
        let snapshot = metrics.get_current_snapshot();
        assert_eq!(snapshot.total_proofs_processed, 1);
        assert_eq!(snapshot.total_proofs_verified, 1);
        assert_eq!(snapshot.average_verification_time_ms, 100.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        assert_eq!(percentile(&data, 0.5), 5.0); // 50th percentile
        assert_eq!(percentile(&data, 0.9), 9.0); // 90th percentile
        assert_eq!(percentile(&[], 0.5), 0.0);  // Empty data
    }
}
