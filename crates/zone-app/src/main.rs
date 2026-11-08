use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use tokio::sync::RwLock;
use tracing::info;
use zone_agent::{start_agent_server, AgentState};
use zone_core::{EngineMode, SplitDirection, SplitNode, TabId, TaskRequest};
use zone_engine_pure::PureEngine;
use zone_engine_web::WebViewController;
use zone_ui::LayoutRect;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Starting Zone Browser (Path B+ Dual-Gear Pure Rust)");

    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless" || a == "--agent-only");

    // Initialize Gear 1 Pure-Rust Engine
    let pure_engine = Arc::new(PureEngine::new(1280, 800));

    // Shared Agent State
    let agent_state = AgentState {
        current_url: Arc::new(RwLock::new("about:blank".to_string())),
        current_snapshot: Arc::new(RwLock::new(None)),
        pure_engine: pure_engine.clone(),
        active_gear: Arc::new(RwLock::new(EngineMode::PureRust)),
        shared_cookies: Arc::new(RwLock::new(Vec::new())),
        tasks: Arc::new(RwLock::new(HashMap::new())),
    };

    // Spin up background Tokio runtime for on-board AI agent daemon
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let daemon_state = agent_state.clone();
    rt.spawn(async move {
        let agent_addr: SocketAddr = "127.0.0.1:4567".parse().unwrap();
        if let Err(e) = start_agent_server(agent_addr, daemon_state).await {
            tracing::error!("Agent daemon encountered error: {}", e);
        }
    });

    info!("On-board AI agent daemon is ACTIVE by default at http://127.0.0.1:4567");

    // CLI autonomous task runner: --task "prompt"
    if let Some(task_idx) = args.iter().position(|a| a == "--task") {
        if let Some(prompt) = args.get(task_idx + 1) {
            info!("Executing CLI autonomous task: '{}'", prompt);
            let prompt_str = prompt.clone();
            rt.block_on(async move {
                // Wait briefly for daemon startup
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let client = reqwest::Client::new();
                let task_req = TaskRequest {
                    prompt: prompt_str,
                    max_steps: 5,
                    llm_endpoint: None,
                    model: None,
                };
                match client
                    .post("http://127.0.0.1:4567/v1/task")
                    .json(&task_req)
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if let Ok(task_res) = resp.json::<zone_core::TaskResponse>().await {
                            println!("\n=== TASK COMPLETED ===");
                            println!("ID: {}", task_res.task_id);
                            println!("Status: {}", task_res.status);
                            println!("Summary: {}\n", task_res.summary);
                            for step in task_res.steps {
                                println!("Step {}: {}", step.step, step.thought);
                                println!("  -> Result: {}", step.result.message);
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to execute task: {}", e),
                }
            });
            return Ok(());
        }
    }

    if is_headless {
        info!("Running in headless mode. Press Ctrl+C to terminate.");
        rt.block_on(async {
            let _ = tokio::signal::ctrl_c().await;
        });
        return Ok(());
    }

    // Phase 2: Interactive Desktop Window with Zen Split View & Vertical Tabs
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Zone Browser")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .build(&event_loop)?;

    let mut controller = WebViewController::new();

    // Demo Zen ergonomics: 2-way horizontal split view
    let tab_left = TabId::new();
    let tab_right = TabId::new();

    let split_tree = SplitNode::Split {
        direction: SplitDirection::Horizontal,
        ratio: 0.50,
        first: Box::new(SplitNode::Leaf(tab_left)),
        second: Box::new(SplitNode::Leaf(tab_right)),
    };

    let sidebar_width = 56.0f32; // Zen collapsible vertical tabs rail
    let initial_size = window.inner_size();
    let content_area = LayoutRect::new(
        sidebar_width,
        0.0,
        initial_size.width as f32 - sidebar_width,
        initial_size.height as f32,
    );

    // Initial tabs: left tab Hacker News, right tab GitHub
    let initial_placements = zone_ui::SplitLayoutEngine::compute_layout(&split_tree, content_area);
    if let Some((_, r_left)) = initial_placements.iter().find(|(id, _)| *id == tab_left) {
        let _ = controller.create_tab(&window, tab_left, "https://news.ycombinator.com", *r_left);
    }
    if let Some((_, r_right)) = initial_placements.iter().find(|(id, _)| *id == tab_right) {
        let _ = controller.create_tab(&window, tab_right, "https://github.com", *r_right);
    }

    info!("Zone UI Window initialized. Running native Tao message loop.");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let updated_content = LayoutRect::new(
                    sidebar_width,
                    0.0,
                    size.width as f32 - sidebar_width,
                    size.height as f32,
                );
                controller.apply_split_layout(&split_tree, updated_content);
            }
            _ => (),
        }
    });
}
