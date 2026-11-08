use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::window::Window;
use tracing::info;
use wry::{Rect, WebView, WebViewBuilder};
use zone_core::{BoundingBox, DomSnapshot, EngineMode, InteractiveElement, SplitNode, TabId};
use zone_ui::{LayoutRect, SplitLayoutEngine};

pub const DOM_EXTRACTOR_JS: &str = r#"
(function() {
    window.__zoneExtractDom = function() {
        const interactive = document.querySelectorAll("a[href], button, input, select, textarea, [role='button'], [role='link']");
        const elements = [];
        let idCounter = 0;

        for (const el of interactive) {
            const rect = el.getBoundingClientRect();
            // Filter non-visible
            if (rect.width === 0 || rect.height === 0 || window.getComputedStyle(el).display === 'none') {
                continue;
            }

            elements.push({
                id: idCounter++,
                tag: el.tagName.toLowerCase(),
                role: el.getAttribute('role') || el.tagName.toLowerCase(),
                text: (el.innerText || el.value || el.getAttribute('placeholder') || el.getAttribute('aria-label') || '').trim(),
                placeholder: el.getAttribute('placeholder') || null,
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
})();
"#;

pub struct WebTab {
    pub id: TabId,
    pub webview: WebView,
    pub current_url: String,
    pub current_title: String,
}

pub struct WebViewController {
    tabs: HashMap<TabId, WebTab>,
    latest_dom: Arc<Mutex<Option<DomSnapshot>>>,
}

impl WebViewController {
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            latest_dom: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_latest_dom(&self) -> Option<DomSnapshot> {
        self.latest_dom.lock().unwrap().clone()
    }

    /// Creates and attaches a child WebView2 into the host Tao window
    pub fn create_tab(
        &mut self,
        window: &Window,
        tab_id: TabId,
        url: &str,
        bounds: LayoutRect,
    ) -> Result<()> {
        let latest_dom_ref = self.latest_dom.clone();

        let rect = Rect {
            position: PhysicalPosition::new(bounds.x as i32, bounds.y as i32).into(),
            size: PhysicalSize::new(bounds.width as u32, bounds.height as u32).into(),
        };

        let webview = WebViewBuilder::new()
            .with_bounds(rect)
            .with_initialization_script(DOM_EXTRACTOR_JS)
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                let msg = req.body();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(msg) {
                    if val.get("type").and_then(|t| t.as_str()) == Some("dom_extracted") {
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
                            info!("Gear 2: Extracted {} interactive elements via native IPC", raw_elements.len());
                        }
                    }
                }
            })
            .with_url(url)
            .build_as_child(window)?;

        self.tabs.insert(
            tab_id,
            WebTab {
                id: tab_id,
                webview,
                current_url: url.to_string(),
                current_title: String::new(),
            },
        );

        Ok(())
    }

    /// Repositions all active webviews based on a SplitNode tree calculation
    pub fn apply_split_layout(&mut self, tree: &SplitNode, content_area: LayoutRect) {
        let placements = SplitLayoutEngine::compute_layout(tree, content_area);
        for (tab_id, rect) in placements {
            if let Some(tab) = self.tabs.get_mut(&tab_id) {
                let wry_rect = Rect {
                    position: PhysicalPosition::new(rect.x as i32, rect.y as i32).into(),
                    size: PhysicalSize::new(rect.width as u32, rect.height as u32).into(),
                };
                let _ = tab.webview.set_bounds(wry_rect);
            }
        }
    }

    /// Evaluates DOM extraction on a specific tab
    pub fn request_dom_extraction(&self, tab_id: TabId) {
        if let Some(tab) = self.tabs.get(&tab_id) {
            let _ = tab.webview.evaluate_script("window.__zoneExtractDom && window.__zoneExtractDom();");
        }
    }
}
