#![windows_subsystem = "windows"]

use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use tokio::sync::RwLock;
use tracing::info;
use zone_agent::{start_agent_server, AgentState};
use zone_core::{EngineMode, TabId, TaskRequest};
use zone_engine_pure::PureEngine;
use zone_engine_web::{ChromeCommand, WebViewController};

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
        started_at: std::time::Instant::now(),
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

    // Zen Browser UI: Left Chrome Sidebar (230px) + Framed Content WebViews
    let event_loop = EventLoopBuilder::<ChromeCommand>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Zen")
        .with_theme(Some(tao::window::Theme::Dark))
        .with_inner_size(LogicalSize::new(1280.0, 820.0))
        .build(&event_loop)?;

    let sidebar_width = 230u32;
    let mut controller = WebViewController::new(sidebar_width);

    let initial_size = window.inner_size();
    controller.init_chrome_sidebar(&window, initial_size.width, initial_size.height, proxy)?;

    // Initial tabs matching real browsing workflow
    let tab_1 = TabId::new();
    let tab_2 = TabId::new();
    let content_bounds = controller.content_bounds();
    controller.create_tab(&window, tab_1, "https://github.com/amirulaliffcherahim/zone", content_bounds)?;
    controller.create_tab(&window, tab_2, "https://news.ycombinator.com", content_bounds)?;

    info!("Zone UI Window initialized with Zen sidebar. Running native Tao message loop.");

    let mut startup_sync_ticks = 20u32;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(cmd) => {
                match cmd {
                    ChromeCommand::Navigate { url } => {
                        controller.navigate_active(&url);
                    }
                    ChromeCommand::Back => {
                        controller.back_active();
                    }
                    ChromeCommand::Forward => {
                        controller.forward_active();
                    }
                    ChromeCommand::Reload => {
                        controller.reload_active();
                    }
                    ChromeCommand::NewTab => {
                        let new_id = TabId::new();
                        let bounds = controller.content_bounds();
                        let _ = controller.create_tab(&window, new_id, "https://www.google.com", bounds);
                    }
                    ChromeCommand::CloseTab { id } => {
                        controller.close_tab(id);
                    }
                    ChromeCommand::SwitchTab { id } => {
                        controller.switch_tab(id);
                    }
                    ChromeCommand::ToggleSplit => {
                        controller.toggle_split_view();
                    }
                    ChromeCommand::SwitchWorkspace { name } => {
                        controller.switch_workspace(&window, name);
                    }
                    ChromeCommand::RequestSync => {
                        controller.sync_chrome_state();
                    }
                    ChromeCommand::Log { text } => {
                        info!("Chrome JS Log: {}", text);
                    }
                    ChromeCommand::ContentTitleChanged { id, title } => {
                        controller.update_tab_title(id, title);
                    }
                    ChromeCommand::ContentUrlChanged { id, url } => {
                        controller.update_tab_url(id, url);
                    }
                    ChromeCommand::ContentOpenNewTab { url } => {
                        let new_id = TabId::new();
                        let bounds = controller.content_bounds();
                        let _ = controller.create_tab(&window, new_id, &url, bounds);
                    }
                    ChromeCommand::FocusOmnibox => {
                        controller.focus_omnibox();
                    }
                }
            }
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
                controller.handle_resize(size.width, size.height);
            }
            Event::MainEventsCleared => {
                if startup_sync_ticks > 0 {
                    startup_sync_ticks -= 1;
                    controller.sync_chrome_state();
                }
            }
            _ => (),
        }
    });
}
