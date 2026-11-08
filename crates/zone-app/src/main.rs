use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use zone_agent::{start_agent_server, AgentState};
use zone_core::EngineMode;
use zone_engine_pure::PureEngine;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Zone Browser...");
    info!("Mode: Path B+ Dual-Gear Pure Rust Engine");

    // Initialize Gear 1 Pure-Rust Engine
    let pure_engine = Arc::new(PureEngine::new(1280, 800));

    // Shared Agent State
    let state = AgentState {
        current_url: Arc::new(RwLock::new("about:blank".to_string())),
        current_snapshot: Arc::new(RwLock::new(None)),
        pure_engine: pure_engine.clone(),
        active_gear: Arc::new(RwLock::new(EngineMode::PureRust)),
    };

    // Spin up On-Board AI Agent Daemon on 127.0.0.1:4567
    let agent_addr: SocketAddr = "127.0.0.1:4567".parse()?;
    let server_state = state.clone();

    tokio::spawn(async move {
        if let Err(e) = start_agent_server(agent_addr, server_state).await {
            tracing::error!("Agent daemon encountered error: {}", e);
        }
    });

    info!("On-board AI agent daemon is ON by default at http://127.0.0.1:4567");
    info!("Ready for commands. Press Ctrl+C to exit.");

    // Keep running
    tokio::signal::ctrl_c().await?;
    info!("Zone Browser shutting down.");

    Ok(())
}
