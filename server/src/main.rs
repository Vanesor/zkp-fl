mod verifier;
mod storage;
mod api;
mod metrics;
mod network;

use clap::Parser;
use common::{Config, Result, ZkpFlError};
use log::{info, error};
use std::sync::Arc;
use std::time::Instant;
use warp::Filter;

#[derive(Parser, Debug)]
#[command(name = "zkp-fl-server")]
#[command(about = "ZKP Federated Learning Server")]
pub struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    /// Server host
    #[arg(long)]
    host: Option<String>,
    
    /// Server port
    #[arg(short, long)]
    port: Option<u16>,
    
    /// Enable detailed logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Clear proof storage on startup
    #[arg(long)]
    clear_storage: bool,
}

pub struct Server {
    config: Config,
    verifier: Arc<verifier::ProofVerifier>,
    storage: Arc<storage::ProofStorage>,
    metrics: Arc<metrics::ServerMetrics>,
}

impl Server {
    pub async fn new(args: Args) -> Result<Self> {
        // Load configuration
        let mut config = Self::load_config(&args.config)?;
        
        // Override config with command line arguments
        if let Some(host) = args.host {
            config.server.host = host;
        }
        if let Some(port) = args.port {
            config.server.port = port;
        }

        info!("Initializing ZKP-FL server on {}:{}", config.server.host, config.server.port);

        // Initialize components
        let verifier = Arc::new(verifier::ProofVerifier::new(&config.circuit)?);
        let storage = Arc::new(storage::ProofStorage::new(&config.server, args.clear_storage)?);
        let metrics = Arc::new(metrics::ServerMetrics::new());

        Ok(Self {
            config,
            verifier,
            storage,
            metrics,
        })
    }

    fn load_config(path: &str) -> Result<Config> {
        let config_str = std::fs::read_to_string(path)
            .map_err(|e| ZkpFlError::Config(format!("Failed to read config file: {}", e)))?;
        
        let config: Config = toml::from_str(&config_str)
            .map_err(|e| ZkpFlError::Config(format!("Failed to parse config: {}", e)))?;
        
        Ok(config)
    }    pub async fn run(&self) -> Result<()> {
        info!("Starting ZKP-FL server...");
        let start_time = Instant::now();

        // Create API routes
        let routes = self.create_routes();

        // Start metrics collection task
        let metrics_task = self.start_metrics_collection();

        // Start server
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| ZkpFlError::Config(format!("Invalid server address: {}", e)))?;

        info!("Server ready on http://{}", addr);
        info!("Startup time: {}ms", start_time.elapsed().as_millis());

        // Run server and metrics collection concurrently
        tokio::select! {
            result = warp::serve(routes).run(addr) => {
                info!("Server stopped: {:?}", result);
            }
            result = metrics_task => {
                info!("Metrics collection stopped: {:?}", result);
            }
        }

        info!("Server shut down cleanly");
        Ok(())
    }

    fn create_routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let api_routes = api::create_api_routes(
            self.verifier.clone(),
            self.storage.clone(),
            self.metrics.clone(),
        );

        // CORS headers
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

        api_routes.with(cors)
    }    fn start_metrics_collection(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        let storage = self.storage.clone();
        let interval_ms = self.config.benchmarks.metrics_interval_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_millis(interval_ms)
            );

            loop {
                interval.tick().await;
                
                // Collect current metrics
                let current_metrics = metrics.get_current_snapshot();
                
                // Update storage metrics
                storage.update_metrics(&current_metrics).await;
                
                // Log periodic status
                if current_metrics.total_proofs_processed % 10 == 0 && current_metrics.total_proofs_processed > 0 {
                    info!("Server status: {} proofs processed, {} verified, avg verification time: {:.2}ms",
                          current_metrics.total_proofs_processed,
                          current_metrics.total_proofs_verified,
                          current_metrics.average_verification_time_ms);
                }
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();
    
    info!("ZKP-FL Server starting...");
    
    let server = Server::new(args).await?;
    
    match server.run().await {
        Ok(()) => {
            info!("Server shut down cleanly");
            Ok(())
        }
        Err(e) => {
            error!("Server error: {}", e);
            Err(e)
        }
    }
}
