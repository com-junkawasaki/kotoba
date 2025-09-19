//! Kubernetes Operator for Kotoba Workflow Engine
//!
//! This operator manages workflow resources in Kubernetes clusters,
//! providing automated deployment, scaling, and lifecycle management.

use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

mod controller;
mod crds;
mod manager;
mod reconciler;

use crate::controller::WorkflowController;
use crate::manager::WorkflowManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Kotoba Workflow Operator");

    // Create Kubernetes client
    let client = kube::Client::try_default().await?;
    info!("Connected to Kubernetes cluster");

    // Create workflow manager
    let manager = Arc::new(WorkflowManager::new(client.clone()));

    // Create and start controller
    let controller = WorkflowController::new(client, manager);
    let controller_handle = tokio::spawn(async move {
        if let Err(e) = controller.run().await {
            error!("Controller error: {}", e);
        }
    });

    // Wait for shutdown signal
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM, shutting down");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT, shutting down");
        }
    }

    // Graceful shutdown
    controller_handle.abort();
    info!("Workflow operator shutdown complete");

    Ok(())
}
