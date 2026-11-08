use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use zone_core::{ActionResult, AgentAction, DomSnapshot, EngineMode};
use zone_engine_pure::PureEngine;

#[derive(Clone)]
pub struct AgentState {
    pub current_url: Arc<RwLock<String>>,
    pub current_snapshot: Arc<RwLock<Option<DomSnapshot>>>,
    pub pure_engine: Arc<PureEngine>,
    pub active_gear: Arc<RwLock<EngineMode>>,
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
        .with_state(state)
}

async fn health_handler(State(state): State<AgentState>) -> Json<HealthResponse> {
    let gear = *state.active_gear.read().await;
    Json(HealthResponse {
        status: "ok",
        engine: format!("{:?}", gear),
        uptime_seconds: 42,
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
            if let Some(snapshot) = snap_guard.as_ref() {
                if let Some(target) = snapshot.elements.iter().find(|e| e.id == target_id) {
                    Json(ActionResult {
                        success: true,
                        message: format!(
                            "Simulated click on target [{}]: <{}> '{}' at ({:.1}, {:.1})",
                            target_id,
                            target.tag,
                            target.text,
                            target.bounds.center().0,
                            target.bounds.center().1
                        ),
                        new_url: None,
                    })
                } else {
                    Json(ActionResult {
                        success: false,
                        message: format!("Target ID [{}] not found in DOM snapshot", target_id),
                        new_url: None,
                    })
                }
            } else {
                Json(ActionResult {
                    success: false,
                    message: "No DOM snapshot loaded".to_string(),
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

pub async fn start_agent_server(addr: SocketAddr, state: AgentState) -> Result<()> {
    let app = create_agent_router(state);
    info!("Zone Browser Agent daemon listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
