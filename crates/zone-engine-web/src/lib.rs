pub mod chrome;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event_loop::EventLoopProxy;
use tao::window::Window;
use tracing::{info, warn};
use wry::{PageLoadEvent, Rect, WebView, WebViewBuilder};
use zone_core::{
    BoundingBox, DomSnapshot, EngineMode, GlanceConfig, InteractiveElement, SplitDirection,
    SplitNode, TabId,
};
use zone_ui::{LayoutRect, SplitLayoutEngine};

pub const DOM_EXTRACTOR_JS: &str = r#"
(function() {
    window.__zoneExtractDom = function() {
        const interactive = document.querySelectorAll("a[href], button, input, select, textarea, [role='button'], [role='link']");
        const elements = [];
        let idCounter = 0;

        for (const el of interactive) {
            const rect = el.getBoundingClientRect();
            if (rect.width === 0 || rect.height === 0 || window.getComputedStyle(el).display === 'none') {
                continue;
            }

            elements.push({
                id: idCounter++,
                tag: el.tagName.toLowerCase(),
                role: el.getAttribute('role') || el.tagName.toLowerCase(),
                text: (el.innerText || el.value || el.getAttribute('placeholder') || el.getAttribute('aria-label') || '').trim(),
                placeholder: el.getAttribute('placeholder') || null,
                href: el.getAttribute('href') || null,
                bounds: {
                    x: rect.left,
                    y: rect.top,
                    width: rect.width,
                    height: rect.height
                }
            });
        }

        const payload = {
            type: "dom_extracted",
            url: window.location.href,
            title: document.title,
            elements: elements
        };

        if (window.ipc) {
            window.ipc.postMessage(JSON.stringify(payload));
        }
        return payload;
    };

    // Forward standard browser keyboard shortcuts from content back to host
    window.addEventListener('keydown', function(e) {
        if (!window.ipc) return;
        if (e.ctrlKey && e.key.toLowerCase() === 't') {
            e.preventDefault();
            window.ipc.postMessage(JSON.stringify({ type: 'hotkey', key: 'ctrl_t' }));
        } else if (e.ctrlKey && e.key.toLowerCase() === 'w') {
            e.preventDefault();
            window.ipc.postMessage(JSON.stringify({ type: 'hotkey', key: 'ctrl_w' }));
        } else if (e.ctrlKey && e.key.toLowerCase() === 'l') {
            e.preventDefault();
            window.ipc.postMessage(JSON.stringify({ type: 'hotkey', key: 'ctrl_l' }));
        } else if (e.ctrlKey && e.key.toLowerCase() === 'r') {
            e.preventDefault();
            window.ipc.postMessage(JSON.stringify({ type: 'hotkey', key: 'ctrl_r' }));
        }
    });

    // Report top-level metadata safely without subframe pollution
    if (window === window.top) {
        function reportMeta() {
            if (window.ipc) {
                window.ipc.postMessage(JSON.stringify({
                    type: "page_meta",
                    url: window.location.href,
                    title: document.title || window.location.hostname
                }));
            }
        }
        window.addEventListener("DOMContentLoaded", reportMeta);
        window.addEventListener("load", reportMeta);
        window.addEventListener("popstate", reportMeta);
        window.addEventListener("hashchange", reportMeta);
        setTimeout(reportMeta, 150);
        setTimeout(reportMeta, 1000);
    }
})();
"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ChromeCommand {
    Navigate { url: String },
    Back,
    Forward,
    Reload,
    NewTab,
    CloseTab { id: TabId },
    SwitchTab { id: TabId },
    ToggleSplit,
    SwitchWorkspace { name: String },
    RequestSync,
    Log { text: String },
    // Internal events sent from content webview callbacks
    #[serde(skip)]
    ContentTitleChanged { id: TabId, title: String },
    #[serde(skip)]
    ContentUrlChanged { id: TabId, url: String },
    #[serde(skip)]
    ContentOpenNewTab { url: String },
    #[serde(skip)]
    FocusOmnibox,
}

#[derive(Debug, Serialize)]
struct TabInfo {
    id: TabId,
    title: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct ChromeStatePayload {
    active_tab_id: Option<TabId>,
    active_url: String,
    active_workspace: String,
    tabs: Vec<TabInfo>,
}

pub struct WebTab {
    pub id: TabId,
    pub webview: WebView,
    pub current_url: String,
    pub current_title: String,
    pub workspace: String,
}

pub struct WebViewController {
    pub chrome: Option<WebView>,
    pub tabs: HashMap<TabId, WebTab>,
    pub tab_order: Vec<TabId>,
    pub active_tab_id: Option<TabId>,
    pub split_partner_id: Option<TabId>,
    pub latest_dom: Arc<Mutex<Option<DomSnapshot>>>,
    pub glance: Option<WebView>,
    pub sidebar_width: u32,
    pub window_size: (u32, u32),
    pub proxy: Option<EventLoopProxy<ChromeCommand>>,
    pub active_workspace: String,
}

impl WebViewController {
    pub fn new(sidebar_width: u32) -> Self {
        Self {
            chrome: None,
            tabs: HashMap::new(),
            tab_order: Vec::new(),
            active_tab_id: None,
            split_partner_id: None,
            latest_dom: Arc::new(Mutex::new(None)),
            glance: None,
            sidebar_width,
            window_size: (1280, 800),
            proxy: None,
            active_workspace: "Personal".to_string(),
        }
    }

    pub fn get_latest_dom(&self) -> Option<DomSnapshot> {
        self.latest_dom.lock().unwrap().clone()
    }

    pub fn content_bounds(&self) -> LayoutRect {
        let (w, h) = self.window_size;
        let pad = 6.0f32; // Zen floating card padding
        let content_x = self.sidebar_width as f32 + pad;
        let content_y = pad;
        let content_w = (w as f32 - content_x - pad).max(100.0);
        let content_h = (h as f32 - (pad * 2.0)).max(100.0);
        LayoutRect::new(content_x, content_y, content_w, content_h)
    }

    /// Creates the dedicated Zen Chrome Sidebar WebView at x=0 with trusted IPC
    pub fn init_chrome_sidebar(
        &mut self,
        window: &Window,
        width: u32,
        height: u32,
        proxy: EventLoopProxy<ChromeCommand>,
    ) -> Result<()> {
        self.window_size = (width, height);
        self.proxy = Some(proxy.clone());

        let rect = Rect {
            position: PhysicalPosition::new(0, 0).into(),
            size: PhysicalSize::new(self.sidebar_width, height).into(),
        };

        let proxy_load = proxy.clone();
        let webview = WebViewBuilder::new()
            .with_bounds(rect)
            .with_initialization_script(chrome::CHROME_JS)
            .with_custom_protocol("zone".into(), |_id, _req| {
                wry::http::Response::builder()
                    .header("Content-Type", "text/html; charset=utf-8")
                    .header("Access-Control-Allow-Origin", "*")
                    .body(std::borrow::Cow::Borrowed(chrome::CHROME_HTML.as_bytes()))
                    .unwrap()
            })
            .with_url("http://zone.localhost/index.html")
            .with_on_page_load_handler(move |event, _url| {
                if matches!(event, PageLoadEvent::Finished) {
                    let _ = proxy_load.send_event(ChromeCommand::RequestSync);
                }
            })
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                let msg = req.body();
                match serde_json::from_str::<ChromeCommand>(msg) {
                    Ok(cmd) => {
                        let _ = proxy.send_event(cmd);
                    }
                    Err(e) => {
                        warn!("Rejected untrusted Chrome IPC message: {} ({})", msg, e);
                    }
                }
            })
            .build_as_child(window)?;

        self.chrome = Some(webview);
        Ok(())
    }

    /// Creates and attaches a content WebView with live title/URL tracking and isolated IPC
    pub fn create_tab(
        &mut self,
        window: &Window,
        tab_id: TabId,
        url: &str,
        bounds: LayoutRect,
    ) -> Result<()> {
        let latest_dom_ref = self.latest_dom.clone();
        let proxy_ipc = self.proxy.clone();
        let proxy_title = self.proxy.clone();
        let proxy_new_win = self.proxy.clone();

        let rect = Rect {
            position: PhysicalPosition::new(bounds.x as i32, bounds.y as i32).into(),
            size: PhysicalSize::new(bounds.width as u32, bounds.height as u32).into(),
        };

        // Hide previous tabs unless in split view
        for existing in self.tabs.values_mut() {
            if Some(existing.id) != self.split_partner_id {
                let _ = existing.webview.set_visible(false);
            }
        }

        let webview = WebViewBuilder::new()
            .with_bounds(rect)
            .with_initialization_script(DOM_EXTRACTOR_JS)
            // ISOLATED CONTENT IPC: Accepts ONLY "dom_extracted" and allowlisted hotkeys
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                let msg = req.body();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
                    match val.get("type").and_then(|t| t.as_str()) {
                        Some("dom_extracted") => {
                            if let (Some(url_str), Some(title_str), Some(raw_elements)) = (
                                val.get("url").and_then(|u| u.as_str()),
                                val.get("title").and_then(|t| t.as_str()),
                                val.get("elements").and_then(|e| e.as_array()),
                            ) {
                                let mut elements = Vec::new();
                                for el in raw_elements {
                                    let id = el.get("id").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                    let tag = el.get("tag").and_then(|t| t.as_str()).unwrap_or("div").to_string();
                                    let role = el.get("role").and_then(|r| r.as_str()).unwrap_or("generic").to_string();
                                    let text = el.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
                                    let placeholder = el.get("placeholder").and_then(|p| p.as_str()).map(|s| s.to_string());
                                    let href = el.get("href").and_then(|h| h.as_str()).map(|s| s.to_string());
                                    let b = el.get("bounds");
                                    let bounds = BoundingBox {
                                        x: b.and_then(|v| v.get("x")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                                        y: b.and_then(|v| v.get("y")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                                        width: b.and_then(|v| v.get("width")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                                        height: b.and_then(|v| v.get("height")).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                                    };

                                    elements.push(InteractiveElement {
                                        id,
                                        tag,
                                        role,
                                        text,
                                        placeholder,
                                        href,
                                        bounds,
                                    });
                                }

                                let snapshot = DomSnapshot {
                                    url: url_str.to_string(),
                                    title: title_str.to_string(),
                                    engine_used: EngineMode::WebView2,
                                    elements,
                                };

                                *latest_dom_ref.lock().unwrap() = Some(snapshot);
                                info!("Gear 2: Extracted {} interactive elements via isolated IPC", raw_elements.len());
                            }
                        }
                        Some("hotkey") => {
                            if let Some(p) = &proxy_ipc {
                                match val.get("key").and_then(|k| k.as_str()) {
                                    Some("ctrl_t") => { let _ = p.send_event(ChromeCommand::NewTab); }
                                    Some("ctrl_w") => { let _ = p.send_event(ChromeCommand::CloseTab { id: tab_id }); }
                                    Some("ctrl_l") => { let _ = p.send_event(ChromeCommand::FocusOmnibox); }
                                    Some("ctrl_r") => { let _ = p.send_event(ChromeCommand::Reload); }
                                    _ => {}
                                }
                            }
                        }
                        Some("page_meta") => {
                            if let Some(p) = &proxy_ipc {
                                if let Some(u) = val.get("url").and_then(|v| v.as_str()) {
                                    let _ = p.send_event(ChromeCommand::ContentUrlChanged {
                                        id: tab_id,
                                        url: u.to_string(),
                                    });
                                }
                                if let Some(t) = val.get("title").and_then(|v| v.as_str()) {
                                    let _ = p.send_event(ChromeCommand::ContentTitleChanged {
                                        id: tab_id,
                                        title: t.to_string(),
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            })
            .with_document_title_changed_handler(move |title: String| {
                if let Some(p) = &proxy_title {
                    let _ = p.send_event(ChromeCommand::ContentTitleChanged {
                        id: tab_id,
                        title,
                    });
                }
            })
            .with_new_window_req_handler(move |target_url: String| {
                if let Some(p) = &proxy_new_win {
                    let _ = p.send_event(ChromeCommand::ContentOpenNewTab { url: target_url });
                }
                false
            })
            .with_url(url)
            .build_as_child(window)?;

        self.tabs.insert(
            tab_id,
            WebTab {
                id: tab_id,
                webview,
                current_url: url.to_string(),
                current_title: url.to_string(),
                workspace: self.active_workspace.clone(),
            },
        );

        self.tab_order.push(tab_id);
        self.active_tab_id = Some(tab_id);

        if let Some(p) = &self.proxy {
            let _ = p.send_event(ChromeCommand::RequestSync);
        }

        self.relayout_active_tabs();
        self.sync_chrome_state();
        Ok(())
    }

    pub fn update_tab_title(&mut self, tab_id: TabId, title: String) {
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            if !title.trim().is_empty() {
                tab.current_title = title;
                self.sync_chrome_state();
            }
        }
    }

    pub fn update_tab_url(&mut self, tab_id: TabId, url: String) {
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            if url.starts_with("http://") || url.starts_with("https://") {
                tab.current_url = url;
                self.sync_chrome_state();
            }
        }
    }

    pub fn focus_omnibox(&self) {
        if let Some(chrome) = &self.chrome {
            let _ = chrome.focus();
            let _ = chrome.evaluate_script(
                "const el = document.getElementById('omnibox'); if (el) { el.focus(); el.select(); }",
            );
        }
    }

    pub fn switch_workspace(&mut self, window: &Window, name: String) {
        self.active_workspace = name;
        let existing_tab = self
            .tab_order
            .iter()
            .rev()
            .copied()
            .find(|id| self.tabs.get(id).map_or(false, |t| t.workspace == self.active_workspace));

        match existing_tab {
            Some(target_id) => {
                self.switch_tab(target_id);
            }
            None => {
                let new_id = TabId::new();
                let bounds = self.content_bounds();
                let _ = self.create_tab(window, new_id, "https://www.google.com", bounds);
            }
        }
        self.sync_chrome_state();
    }

    /// Toggles 2-way side-by-side Split View using SplitLayoutEngine
    pub fn toggle_split_view(&mut self) {
        if self.split_partner_id.is_some() {
            self.split_partner_id = None;
        } else if self.tab_order.len() >= 2 {
            let active = self.active_tab_id;
            self.split_partner_id = self
                .tab_order
                .iter()
                .rev()
                .copied()
                .find(|id| Some(*id) != active);
        }
        self.relayout_active_tabs();
    }

    pub fn relayout_active_tabs(&mut self) {
        let area = self.content_bounds();

        match (self.active_tab_id, self.split_partner_id) {
            (Some(primary), Some(partner)) if primary != partner && self.tabs.contains_key(&partner) => {
                let tree = SplitNode::Split {
                    direction: SplitDirection::Horizontal,
                    ratio: 0.5,
                    first: Box::new(SplitNode::Leaf(primary)),
                    second: Box::new(SplitNode::Leaf(partner)),
                };
                let placements = SplitLayoutEngine::compute_layout(&tree, area);
                for (id, tab) in &mut self.tabs {
                    if let Some((_, r)) = placements.iter().find(|(tid, _)| tid == id) {
                        let rect = Rect {
                            position: PhysicalPosition::new(r.x as i32, r.y as i32).into(),
                            size: PhysicalSize::new((r.width - 3.0).max(50.0) as u32, r.height as u32).into(),
                        };
                        let _ = tab.webview.set_bounds(rect);
                        let _ = tab.webview.set_visible(true);
                    } else {
                        let _ = tab.webview.set_visible(false);
                    }
                }
            }
            (Some(primary), _) => {
                self.split_partner_id = None;
                let rect = Rect {
                    position: PhysicalPosition::new(area.x as i32, area.y as i32).into(),
                    size: PhysicalSize::new(area.width as u32, area.height as u32).into(),
                };
                for (id, tab) in &mut self.tabs {
                    if *id == primary {
                        let _ = tab.webview.set_bounds(rect);
                        let _ = tab.webview.set_visible(true);
                    } else {
                        let _ = tab.webview.set_visible(false);
                    }
                }
            }
            _ => {}
        }
    }

    /// Switches visible tab, hiding others to preserve scroll & session state
    pub fn switch_tab(&mut self, tab_id: TabId) {
        if !self.tabs.contains_key(&tab_id) {
            return;
        }

        self.active_tab_id = Some(tab_id);
        self.relayout_active_tabs();

        if let Some(tab) = self.tabs.get(&tab_id) {
            let _ = tab.webview.focus();
        }

        self.sync_chrome_state();
    }

    /// Closes a tab and selects its neighbor
    pub fn close_tab(&mut self, tab_id: TabId) {
        if self.tab_order.len() <= 1 {
            // Keep at least one tab open like Zen Browser
            return;
        }

        self.tabs.remove(&tab_id);
        self.tab_order.retain(|id| *id != tab_id);

        if self.split_partner_id == Some(tab_id) {
            self.split_partner_id = None;
        }

        if self.active_tab_id == Some(tab_id) {
            self.active_tab_id = self.tab_order.last().copied();
        }

        self.relayout_active_tabs();
        self.sync_chrome_state();
    }

    /// Synchronizes browser state to the Chrome WebView securely via serde_json
    pub fn sync_chrome_state(&self) {
        if let Some(chrome) = &self.chrome {
            let active_url = self
                .active_tab_id
                .and_then(|id| self.tabs.get(&id))
                .map(|t| t.current_url.clone())
                .unwrap_or_default();

            let tabs_info: Vec<TabInfo> = self
                .tab_order
                .iter()
                .filter_map(|id| self.tabs.get(id))
                .filter(|t| t.workspace == self.active_workspace)
                .map(|t| TabInfo {
                    id: t.id,
                    title: t.current_title.clone(),
                    url: t.current_url.clone(),
                })
                .collect();

            let payload = ChromeStatePayload {
                active_tab_id: self.active_tab_id,
                active_url,
                active_workspace: self.active_workspace.clone(),
                tabs: tabs_info,
            };

            if let Ok(json_str) = serde_json::to_string(&payload) {
                let script = format!(
                    "if (typeof window.__zoneUpdateState === 'function') {{ window.__zoneUpdateState({}); }} else {{ window.__pendingState = {}; }}",
                    json_str, json_str
                );
                let _ = chrome.evaluate_script(&script);
            }
        }
    }

    /// Repositions all active webviews on window resize
    pub fn handle_resize(&mut self, width: u32, height: u32) {
        self.window_size = (width, height);

        if let Some(chrome) = &mut self.chrome {
            let rect = Rect {
                position: PhysicalPosition::new(0, 0).into(),
                size: PhysicalSize::new(self.sidebar_width, height).into(),
            };
            let _ = chrome.set_bounds(rect);
        }

        self.relayout_active_tabs();
    }

    pub fn navigate_active(&mut self, url: &str) {
        if let Some(id) = self.active_tab_id {
            if let Some(tab) = self.tabs.get_mut(&id) {
                tab.current_url = url.to_string();
                let _ = tab.webview.load_url(url);
                self.sync_chrome_state();
            }
        }
    }

    pub fn back_active(&self) {
        if let Some(id) = self.active_tab_id {
            if let Some(tab) = self.tabs.get(&id) {
                let _ = tab.webview.evaluate_script("window.history.back();");
            }
        }
    }

    pub fn forward_active(&self) {
        if let Some(id) = self.active_tab_id {
            if let Some(tab) = self.tabs.get(&id) {
                let _ = tab.webview.evaluate_script("window.history.forward();");
            }
        }
    }

    pub fn reload_active(&self) {
        if let Some(id) = self.active_tab_id {
            if let Some(tab) = self.tabs.get(&id) {
                let _ = tab.webview.evaluate_script("window.location.reload();");
            }
        }
    }

    pub fn open_glance(
        &mut self,
        window: &Window,
        url: &str,
        config: Option<GlanceConfig>,
    ) -> Result<()> {
        let (pw, ph) = self.window_size;
        let cfg = config.unwrap_or_default();
        let glance_w = (pw as f32 * cfg.scale).min(cfg.width) as u32;
        let glance_h = (ph as f32 * cfg.scale).min(cfg.height) as u32;

        let pos_x = (pw.saturating_sub(glance_w)) / 2;
        let pos_y = (ph.saturating_sub(glance_h)) / 2;

        let rect = Rect {
            position: PhysicalPosition::new(pos_x as i32, pos_y as i32).into(),
            size: PhysicalSize::new(glance_w, glance_h).into(),
        };

        let webview = WebViewBuilder::new()
            .with_bounds(rect)
            .with_url(url)
            .build_as_child(window)?;

        self.glance = Some(webview);
        Ok(())
    }

    pub fn close_glance(&mut self) {
        self.glance = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrome_command_deserialization() {
        let nav: ChromeCommand =
            serde_json::from_str(r#"{"action":"navigate","url":"https://example.com"}"#).unwrap();
        match nav {
            ChromeCommand::Navigate { url } => assert_eq!(url, "https://example.com"),
            _ => panic!("Expected Navigate"),
        }

        let back: ChromeCommand = serde_json::from_str(r#"{"action":"back"}"#).unwrap();
        assert!(matches!(back, ChromeCommand::Back));

        let fwd: ChromeCommand = serde_json::from_str(r#"{"action":"forward"}"#).unwrap();
        assert!(matches!(fwd, ChromeCommand::Forward));

        let reload: ChromeCommand = serde_json::from_str(r#"{"action":"reload"}"#).unwrap();
        assert!(matches!(reload, ChromeCommand::Reload));

        let new_tab: ChromeCommand = serde_json::from_str(r#"{"action":"new_tab"}"#).unwrap();
        assert!(matches!(new_tab, ChromeCommand::NewTab));

        let split: ChromeCommand = serde_json::from_str(r#"{"action":"toggle_split"}"#).unwrap();
        assert!(matches!(split, ChromeCommand::ToggleSplit));

        let sync: ChromeCommand = serde_json::from_str(r#"{"action":"request_sync"}"#).unwrap();
        assert!(matches!(sync, ChromeCommand::RequestSync));

        let ws: ChromeCommand = serde_json::from_str(r#"{"action":"switch_workspace","name":"Work"}"#).unwrap();
        match ws {
            ChromeCommand::SwitchWorkspace { name } => assert_eq!(name, "Work"),
            _ => panic!("Expected SwitchWorkspace"),
        }

        let switch: ChromeCommand = serde_json::from_str(
            r#"{"action":"switch_tab","id":"550e8400-e29b-41d4-a716-446655440000"}"#,
        )
        .unwrap();
        match switch {
            ChromeCommand::SwitchTab { id } => {
                assert_eq!(
                    id.0.to_string(),
                    "550e8400-e29b-41d4-a716-446655440000"
                );
            }
            _ => panic!("Expected SwitchTab"),
        }

        let close: ChromeCommand = serde_json::from_str(
            r#"{"action":"close_tab","id":"550e8400-e29b-41d4-a716-446655440000"}"#,
        )
        .unwrap();
        match close {
            ChromeCommand::CloseTab { id } => {
                assert_eq!(
                    id.0.to_string(),
                    "550e8400-e29b-41d4-a716-446655440000"
                );
            }
            _ => panic!("Expected CloseTab"),
        }
    }
}
