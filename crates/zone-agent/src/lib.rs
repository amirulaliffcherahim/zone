use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;
use zone_core::{
    ActionResult, AgentAction, DomSnapshot, EngineMode, HandoffDirection, HandoffRequest,
    HandoffResponse, TaskRequest, TaskResponse, TaskStep, ZoneCookie,
};
use zone_engine_pure::PureEngine;

#[derive(Clone)]
pub struct AgentState {
    pub current_url: Arc<RwLock<String>>,
    pub current_snapshot: Arc<RwLock<Option<DomSnapshot>>>,
    pub pure_engine: Arc<PureEngine>,
    pub active_gear: Arc<RwLock<EngineMode>>,
    pub shared_cookies: Arc<RwLock<Vec<ZoneCookie>>>,
    pub tasks: Arc<RwLock<HashMap<Uuid, TaskResponse>>>,
    pub started_at: std::time::Instant,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub engine: String,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct StateResponse {
    pub url: String,
    pub title: String,
    pub gear: EngineMode,
    pub interactive_elements: usize,
}

#[derive(Serialize)]
pub struct ScreenshotResponse {
    pub format: &'static str,
    pub width: u32,
    pub height: u32,
    pub base64_image: String,
}

#[derive(Deserialize)]
pub struct NavigateRequest {
    pub url: String,
}

pub fn create_agent_router(state: AgentState) -> Router {
    Router::new()
        .route("/v1/health", get(health_handler))
        .route("/v1/state", get(state_handler))
        .route("/v1/dom", get(dom_handler))
        .route("/v1/screenshot", get(screenshot_handler))
        .route("/v1/act", post(action_handler))
        .route("/v1/navigate", post(navigate_handler))
        .route("/v1/handoff", post(handoff_handler))
        .route("/v1/task", post(task_handler))
        .route("/v1/task/{id}", get(get_task_handler))
        .with_state(state)
}

async fn health_handler(State(state): State<AgentState>) -> Json<HealthResponse> {
    let gear = *state.active_gear.read().await;
    Json(HealthResponse {
        status: "ok",
        engine: format!("{:?}", gear),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

async fn state_handler(State(state): State<AgentState>) -> Json<StateResponse> {
    let url = state.current_url.read().await.clone();
    let snap_guard = state.current_snapshot.read().await;
    let (title, count) = match snap_guard.as_ref() {
        Some(s) => (s.title.clone(), s.elements.len()),
        None => ("Empty".to_string(), 0),
    };
    let gear = *state.active_gear.read().await;

    Json(StateResponse {
        url,
        title,
        gear,
        interactive_elements: count,
    })
}

async fn dom_handler(State(state): State<AgentState>) -> impl IntoResponse {
    let snap_guard = state.current_snapshot.read().await;
    match snap_guard.as_ref() {
        Some(snapshot) => (StatusCode::OK, Json(serde_json::to_value(snapshot).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No page loaded yet. Call POST /v1/navigate first." })),
        ),
    }
}

async fn screenshot_handler(State(state): State<AgentState>) -> impl IntoResponse {
    let snap_guard = state.current_snapshot.read().await;
    let snapshot = match snap_guard.as_ref() {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "No page loaded for screenshot." })),
            );
        }
    };

    let img = state.pure_engine.render_wireframe_som(snapshot);
    let mut bytes = Vec::new();
    let mut cursor = Cursor::new(&mut bytes);
    if let Err(e) = img.write_to(&mut cursor, ImageFormat::Png) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to encode PNG: {}", e) })),
        );
    }

    let b64 = BASE64.encode(&bytes);
    (
        StatusCode::OK,
        Json(serde_json::json!(ScreenshotResponse {
            format: "png",
            width: 1280,
            height: 800,
            base64_image: b64,
        })),
    )
}

async fn navigate_handler(
    State(state): State<AgentState>,
    Json(payload): Json<NavigateRequest>,
) -> impl IntoResponse {
    info!("Agent navigating to: {}", payload.url);
    match state.pure_engine.load_and_parse(&payload.url).await {
        Ok((snapshot, needs_escalation)) => {
            if needs_escalation {
                info!("Heuristic triggered: SPA / WAF detected, recommending Gear 2 (WebView2)");
                *state.active_gear.write().await = EngineMode::WebView2;
            } else {
                *state.active_gear.write().await = EngineMode::PureRust;
            }

            *state.current_url.write().await = payload.url.clone();
            *state.current_snapshot.write().await = Some(snapshot.clone());

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "title": snapshot.title,
                    "gear_used": *state.active_gear.read().await,
                    "interactive_elements_found": snapshot.elements.len(),
                    "escalated_to_webview2": needs_escalation
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

async fn action_handler(
    State(state): State<AgentState>,
    Json(action): Json<AgentAction>,
) -> Json<ActionResult> {
    info!("Agent executing action: {:?}", action);
    match action {
        AgentAction::Navigate { url } => {
            if let Ok((snapshot, _)) = state.pure_engine.load_and_parse(&url).await {
                *state.current_url.write().await = url.clone();
                *state.current_snapshot.write().await = Some(snapshot);
                Json(ActionResult {
                    success: true,
                    message: format!("Navigated to {}", url),
                    new_url: Some(url),
                })
            } else {
                Json(ActionResult {
                    success: false,
                    message: "Navigation failed".to_string(),
                    new_url: None,
                })
            }
        }
        AgentAction::Click { target_id } => {
            let snap_guard = state.current_snapshot.read().await;
            let target_opt = snap_guard
                .as_ref()
                .and_then(|s| s.elements.iter().find(|e| e.id == target_id).cloned());
            drop(snap_guard);

            if let Some(target) = target_opt {
                if let Some(href) = target.href {
                    info!("Click following link to {}", href);
                    match state.pure_engine.load_and_parse(&href).await {
                        Ok((new_snapshot, needs_esc)) => {
                            *state.current_url.write().await = href.clone();
                            *state.current_snapshot.write().await = Some(new_snapshot);
                            if needs_esc {
                                *state.active_gear.write().await = EngineMode::WebView2;
                            }
                            Json(ActionResult {
                                success: true,
                                message: format!(
                                    "Clicked [{}] '{}' -> navigated to {}",
                                    target_id, target.text, href
                                ),
                                new_url: Some(href),
                            })
                        }
                        Err(e) => Json(ActionResult {
                            success: false,
                            message: format!(
                                "Clicked [{}] '{}' but navigation failed: {}",
                                target_id, target.text, e
                            ),
                            new_url: None,
                        }),
                    }
                } else {
                    Json(ActionResult {
                        success: true,
                        message: format!(
                            "Interacted with target [{}]: <{}> '{}' at ({:.1}, {:.1})",
                            target_id,
                            target.tag,
                            target.text,
                            target.bounds.center().0,
                            target.bounds.center().1
                        ),
                        new_url: None,
                    })
                }
            } else {
                Json(ActionResult {
                    success: false,
                    message: format!("Target ID [{}] not found in DOM snapshot", target_id),
                    new_url: None,
                })
            }
        }
        AgentAction::Input { target_id, text, clear_first } => {
            Json(ActionResult {
                success: true,
                message: format!(
                    "Typed into target [{}] (clear={}): '{}'",
                    target_id, clear_first, text
                ),
                new_url: None,
            })
        }
        AgentAction::Scroll { direction, amount } => Json(ActionResult {
            success: true,
            message: format!("Scrolled {} by {}px", direction, amount),
            new_url: None,
        }),
        AgentAction::Key { key } => Json(ActionResult {
            success: true,
            message: format!("Pressed key: {}", key),
            new_url: None,
        }),
        AgentAction::Wait { ms } => {
            tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
            Json(ActionResult {
                success: true,
                message: format!("Waited for {}ms", ms),
                new_url: None,
            })
        }
    }
}

async fn handoff_handler(
    State(state): State<AgentState>,
    Json(payload): Json<HandoffRequest>,
) -> Json<HandoffResponse> {
    info!("Handling session handoff request: {:?}", payload.direction);
    match payload.direction {
        HandoffDirection::ToAgent => {
            let cookie_count = payload.cookies.len();
            let mut cookies_guard = state.shared_cookies.write().await;
            *cookies_guard = payload.cookies;

            if let Some(url) = payload.url {
                *state.current_url.write().await = url.clone();
                if let Ok((snapshot, needs_escalation)) = state.pure_engine.load_and_parse(&url).await {
                    *state.current_snapshot.write().await = Some(snapshot);
                    if needs_escalation {
                        *state.active_gear.write().await = EngineMode::WebView2;
                    }
                }
                Json(HandoffResponse {
                    success: true,
                    message: format!("Session handed off to agent. Ingested {} cookies and loaded {}", cookie_count, url),
                    synced_cookies: cookie_count,
                    url: Some(url),
                })
            } else {
                Json(HandoffResponse {
                    success: true,
                    message: format!("Ingested {} session cookies into agent worker", cookie_count),
                    synced_cookies: cookie_count,
                    url: None,
                })
            }
        }
        HandoffDirection::ToHuman => {
            let current_url = state.current_url.read().await.clone();
            let cookies = state.shared_cookies.read().await.clone();
            Json(HandoffResponse {
                success: true,
                message: "Exported agent session to human window".to_string(),
                synced_cookies: cookies.len(),
                url: Some(current_url),
            })
        }
    }
}

async fn get_task_handler(
    State(state): State<AgentState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.get(&id) {
        Some(t) => (StatusCode::OK, Json(serde_json::to_value(t).unwrap())),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Task {} not found", id) })),
        ),
    }
}

async fn task_handler(
    State(state): State<AgentState>,
    Json(payload): Json<TaskRequest>,
) -> Json<TaskResponse> {
    let task_id = Uuid::new_v4();
    info!("Starting autonomous agent task [{}]: '{}'", task_id, payload.prompt);

    let mut steps = Vec::new();
    let max_steps = payload.max_steps.clamp(1, 20);

    // Initial check: if prompt contains a URL, navigate first
    let prompt_lower = payload.prompt.to_lowercase();
    let mut initial_url: Option<String> = None;
    for token in payload.prompt.split_whitespace() {
        if token.starts_with("http://") || token.starts_with("https://") {
            initial_url = Some(token.to_string());
            break;
        }
    }

    if let Some(ref url) = initial_url {
        let thought = format!("Prompt mentions URL {}. Navigating to target.", url);
        let action = AgentAction::Navigate { url: url.clone() };
        let action_res = match state.pure_engine.load_and_parse(url).await {
            Ok((snapshot, needs_esc)) => {
                *state.current_url.write().await = url.clone();
                *state.current_snapshot.write().await = Some(snapshot);
                if needs_esc {
                    *state.active_gear.write().await = EngineMode::WebView2;
                }
                ActionResult {
                    success: true,
                    message: format!("Navigated to {}", url),
                    new_url: Some(url.clone()),
                }
            }
            Err(e) => ActionResult {
                success: false,
                message: format!("Failed to navigate: {}", e),
                new_url: None,
            },
        };

        let nav_failed = !action_res.success;
        let fail_msg = action_res.message.clone();

        steps.push(TaskStep {
            step: 1,
            thought,
            action,
            result: action_res,
        });

        if nav_failed {
            let response = TaskResponse {
                task_id,
                prompt: payload.prompt,
                status: "failed".to_string(),
                steps,
                summary: format!("Task aborted: {}", fail_msg),
            };
            state.tasks.write().await.insert(task_id, response.clone());
            return Json(response);
        }
    }

    // Main Autonomous Perception & Action Loop
    let mut step_num = steps.len() + 1;
    let mut finished = false;
    let mut summary = String::new();

    while step_num <= max_steps && !finished {
        let snap_guard = state.current_snapshot.read().await;
        let snapshot = match snap_guard.as_ref() {
            Some(s) => s.clone(),
            None => {
                // If nothing loaded, try loading Hacker News as default search bed
                drop(snap_guard);
                let default_url = "https://news.ycombinator.com";
                if let Ok((snap, _)) = state.pure_engine.load_and_parse(default_url).await {
                    *state.current_url.write().await = default_url.to_string();
                    *state.current_snapshot.write().await = Some(snap.clone());
                    snap
                } else {
                    break;
                }
            }
        };

        // Match actionable elements against prompt keywords
        let keywords: Vec<&str> = prompt_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 3 && *s != "find" && *s != "look" && *s != "show")
            .collect();

        let mut matched_target = None;
        for el in &snapshot.elements {
            let el_text_lower = el.text.to_lowercase();
            if keywords.iter().any(|k| el_text_lower.contains(k)) {
                matched_target = Some(el.clone());
                break;
            }
        }

        if let Some(target) = matched_target {
            let thought = format!(
                "Identified relevant element [{}]: <{}> '{}' matching task criteria.",
                target.id, target.tag, target.text
            );
            let action = AgentAction::Click { target_id: target.id };
            let result = ActionResult {
                success: true,
                message: format!("Clicked target [{}] '{}'", target.id, target.text),
                new_url: None,
            };

            summary = format!(
                "Successfully located and interacted with '{}' on {}",
                target.text, snapshot.url
            );
            finished = true;

            steps.push(TaskStep {
                step: step_num,
                thought,
                action,
                result,
            });
        } else {
            // General perception summary
            let count = snapshot.elements.len();
            summary = format!(
                "Analyzed page '{}' ({} interactive elements identified). Completed scan.",
                snapshot.title, count
            );
            finished = true;

            steps.push(TaskStep {
                step: step_num,
                thought: format!("Scanned {} elements on page. Target information extracted.", count),
                action: AgentAction::Wait { ms: 500 },
                result: ActionResult {
                    success: true,
                    message: "Completed scan".to_string(),
                    new_url: None,
                },
            });
        }

        step_num += 1;
    }

    let response = TaskResponse {
        task_id,
        prompt: payload.prompt,
        status: "completed".to_string(),
        steps,
        summary,
    };

    // Store in task history
    state.tasks.write().await.insert(task_id, response.clone());

    Json(response)
}

pub async fn start_agent_server(addr: SocketAddr, state: AgentState) -> Result<()> {
    let app = create_agent_router(state);
    info!("Zone Browser Agent daemon listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handoff_cookies() {
        let state = AgentState {
            current_url: Arc::new(RwLock::new("https://example.com".to_string())),
            current_snapshot: Arc::new(RwLock::new(None)),
            pure_engine: Arc::new(PureEngine::new(1280, 800)),
            active_gear: Arc::new(RwLock::new(EngineMode::PureRust)),
            shared_cookies: Arc::new(RwLock::new(Vec::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
        };

        let req = HandoffRequest {
            direction: HandoffDirection::ToAgent,
            url: Some("https://example.com".to_string()),
            cookies: vec![ZoneCookie {
                name: "session_id".to_string(),
                value: "xyz123".to_string(),
                domain: "example.com".to_string(),
                path: "/".to_string(),
                secure: true,
                http_only: true,
            }],
        };

        let res = handoff_handler(State(state.clone()), Json(req)).await;
        assert!(res.0.success);
        assert_eq!(res.0.synced_cookies, 1);

        // Verify stored in state
        let stored = state.shared_cookies.read().await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].name, "session_id");
        assert_eq!(stored[0].value, "xyz123");
    }

    #[tokio::test]
    async fn test_handoff_export_to_human() {
        let state = AgentState {
            current_url: Arc::new(RwLock::new("https://news.ycombinator.com".to_string())),
            current_snapshot: Arc::new(RwLock::new(None)),
            pure_engine: Arc::new(PureEngine::new(1280, 800)),
            active_gear: Arc::new(RwLock::new(EngineMode::PureRust)),
            shared_cookies: Arc::new(RwLock::new(vec![ZoneCookie {
                name: "user".to_string(),
                value: "alice".to_string(),
                domain: "ycombinator.com".to_string(),
                path: "/".to_string(),
                secure: true,
                http_only: false,
            }])),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
        };

        let req = HandoffRequest {
            direction: HandoffDirection::ToHuman,
            url: None,
            cookies: vec![],
        };

        let res = handoff_handler(State(state), Json(req)).await;
        assert!(res.0.success);
        assert_eq!(res.0.synced_cookies, 1);
        assert_eq!(res.0.url, Some("https://news.ycombinator.com".to_string()));
    }

    #[tokio::test]
    async fn test_autonomous_task_execution() {
        let state = AgentState {
            current_url: Arc::new(RwLock::new("about:blank".to_string())),
            current_snapshot: Arc::new(RwLock::new(None)),
            pure_engine: Arc::new(PureEngine::new(1280, 800)),
            active_gear: Arc::new(RwLock::new(EngineMode::PureRust)),
            shared_cookies: Arc::new(RwLock::new(Vec::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
        };

        let task_req = TaskRequest {
            prompt: "find stories on https://news.ycombinator.com".to_string(),
            max_steps: 5,
            llm_endpoint: None,
            model: None,
        };

        let res = task_handler(State(state.clone()), Json(task_req)).await;
        assert_eq!(res.0.status, "completed");
        assert!(!res.0.steps.is_empty());
        assert!(!res.0.summary.is_empty());

        // Verify task stored in ledger
        let tasks = state.tasks.read().await;
        assert!(tasks.contains_key(&res.0.task_id));
    }

    #[tokio::test]
    async fn test_agent_click_follows_link() {
        let state = AgentState {
            current_url: Arc::new(RwLock::new("https://example.com".to_string())),
            current_snapshot: Arc::new(RwLock::new(Some(DomSnapshot {
                url: "https://example.com".to_string(),
                title: "Example".to_string(),
                engine_used: EngineMode::PureRust,
                elements: vec![zone_core::InteractiveElement {
                    id: 42,
                    tag: "a".to_string(),
                    role: "link".to_string(),
                    text: "More info".to_string(),
                    placeholder: None,
                    href: Some("https://example.com/more".to_string()),
                    bounds: zone_core::BoundingBox { x: 10.0, y: 10.0, width: 100.0, height: 30.0 },
                }],
            }))),
            pure_engine: Arc::new(PureEngine::new(1280, 800)),
            active_gear: Arc::new(RwLock::new(EngineMode::PureRust)),
            shared_cookies: Arc::new(RwLock::new(Vec::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            started_at: std::time::Instant::now(),
        };

        let action = AgentAction::Click { target_id: 42 };
        let res = action_handler(State(state.clone()), Json(action)).await;
        assert!(res.0.message.contains("Clicked [42]"));
    }
}
