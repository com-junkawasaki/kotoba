use std::sync::Arc;
use clap::Parser;
use axum::routing::get;
use kotoba_server_core::{HttpServer, AppRouter, handlers::*};
#[cfg(feature = "workflow")]
use kotoba_server_workflow::{WorkflowRouter, WorkflowServerExt};
#[cfg(feature = "workflow")]
use kotoba_workflow_core::WorkflowEngine;

/// Command line arguments for kotoba-server
#[derive(Parser)]
#[command(name = "kotoba-server")]
#[command(about = "Kotoba HTTP Server")]
struct Args {
    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port number to bind to
    #[arg(long, default_value = "8100")]
    port: u16,

    /// Enable development mode
    #[arg(long)]
    dev: bool,

    /// Enable workflow features
    #[arg(long)]
    workflow: bool,
}

// Setup logging
fn setup_logging() {
    use tracing_subscriber::{prelude::*, EnvFilter, fmt};

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kotoba_server=debug,tower_http=debug".into()),
        )
        .with(fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    setup_logging();

    // Create router with core functionality
    let mut router = AppRouter::new();

    // Add health check
    router = router.route("/health", get(health_check));

    // Add workflow features if enabled
    #[cfg(feature = "workflow")]
    if args.workflow {
        router = router.with_workflow_routes();
        tracing::info!("🔄 Workflow features enabled");
    }
    #[cfg(not(feature = "workflow"))]
    if args.workflow {
        tracing::warn!("⚠️  Workflow features requested but not available (compiled without workflow support)");
    }

    // Build server
    let server = HttpServer::builder()
        .host(args.host)
        .port(args.port)
        .router(router.build())
        .build()?;

    if args.dev {
        tracing::info!("🚀 Development mode enabled");
    }

    // Start server
    server.serve().await?;

    Ok(())
}
