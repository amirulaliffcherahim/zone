# Implementation Spec: Making Every Interactive Browser Feature Work (`DESIGN_PLAN.md`)

**Target Reader:** Smaller coding model / implementing agent.  
**Repository:** `C:\Users\amiru\Local\zone-browser`  
**Verify Command:** `cargo test && cargo build --release --bin zone`  

Lead goal: Fix the 5 root causes blocking human browsing so that typing a URL, clicking sidebar buttons, switching tabs, splitting views, and using the on-board AI agent all work end-to-end.

---

## 1. Root Cause Analysis (Why You Cannot Browse Right Now)

### Blocker 1: Full-Window Chrome WebView Intercepts Mouse & Keyboard Focus
* **Where:** `crates/zone-engine-web/src/lib.rs` line 161 (`init_chrome_sidebar`) and `handle_resize` line 461.
* **The Bug:** `init_chrome_sidebar` sets the Chrome WebView bounds to `PhysicalSize::new(width, height)` (the entire window `1280x820`), while the Content WebView sits at `x = 244`. Because both are child `HWND`s of the same parent `tao::Window`, Windows Z-order routes mouse clicks or focus events to the full-window Chrome WebView or causes the two sibling `HWND`s to fight for input.
* **The Fix:** Restrict the Chrome WebView strictly to the left sidebar width (`x: 0, y: 0, width: self.sidebar_width, height: height`) in both `init_chrome_sidebar` and `handle_resize`. Set the content WebView bounds to (`x: self.sidebar_width, y: 0, width: width - self.sidebar_width, height: height`). Sibling `HWND`s must occupy **strictly non-overlapping rectangles**.

### Blocker 2: `with_html` Drops `window.ipc` and Blocks `evaluate_script` in WebView2
* **Where:** `crates/zone-engine-web/src/lib.rs` line 168 (`.with_html(chrome::CHROME_HTML)`).
* **The Bug:** On Windows `wry 0.49`, `.with_html(...)` calls `ICoreWebView2::NavigateToString`, which runs the page in an opaque `about:blank` origin where custom IPC initialization scripts and external image loads (`google.com/s2/favicons`) behave inconsistently. Furthermore, the initial HTML in `chrome.rs` hardcoded two dummy `<div class="tab-row">` elements without `onclick` handlers (`init-1`, `init-2` UUIDs don't exist in `self.tabs`), so clicking them sends `{"action":"switch_tab","id":"init-1"}` which fails `Uuid` deserialization in Rust (`warn!("Rejected untrusted Chrome IPC message")`)!
* **The Fix:**
  1. Serve `CHROME_HTML` over a local custom protocol `.with_custom_protocol("zone".into(), ...)` and `.with_url("http://zone.localhost/")` (on Windows WebView2, `wry` maps custom protocol `"zone"` to `http://zone.<path>`).
  2. Remove the fake `"init-1"` / `"init-2"` seed call in `chrome.rs` so only real `TabId(Uuid)` values from Rust populate `tabList`.

### Blocker 3: `with_navigation_handler` Overwrites `current_url` with Subframe URLs
* **Where:** `crates/zone-engine-web/src/lib.rs` line 284.
* **The Bug:** `with_navigation_handler` fires for **every iframe, ad frame, and `about:blank` sub-resource** on a webpage. When loading GitHub or Google, an embedded iframe navigates to `https://.../frame.html` and overwrites `tab.current_url` with the iframe URL.
* **The Fix:** Do not update `tab.current_url` from `with_navigation_handler`. Instead, track the real top-level `window.location.href` and `document.title` inside the main-frame initialization script (`DOM_EXTRACTOR_JS` runs only in main frame when registered or checks `window === window.top`) and post `{ "type": "page_meta", "url": window.location.href, "title": document.title }` on `DOMContentLoaded`, `load`, `popstate`, and `hashchange`.

### Blocker 4: workspaces Are Visual-Only (No Per-Workspace Tab Filtering)
* **Where:** `crates/zone-engine-web/src/chrome.rs` line 575.
* **The Bug:** Clicking the workspace dots at the bottom of the sidebar only changes the text label in JS; it does not filter or switch tabs in Rust.
* **The Fix:** Add `workspace: String` to `WebTab` in `crates/zone-engine-web/src/lib.rs` and `SwitchWorkspace { name: String }` to `ChromeCommand`. Filter `tabs_info` by `self.active_workspace` and show/switch to that workspace's active tab.

### Blocker 5: Gear 1 Agent Actions (`Click`, `Input`) Are Stubs
* **Where:** `crates/zone-agent/src/lib.rs` line 183 (`action_handler`) and `crates/zone-engine-pure/src/lib.rs`.
* **The Bug:** In `zone-agent`, `AgentAction::Click` only returns a formatted string (`"Simulated click..."`) without following `<a href="...">` links or mutating page state. And `InteractiveElement` in `zone-core` does not store `href: Option<String>`.
* **The Fix:** Add `pub href: Option<String>` to `InteractiveElement` in `crates/zone-core/src/lib.rs`. Populate `href` in `zone-engine-pure` (resolving relative URLs against the base URL via `url::Url::join`). When `AgentAction::Click { target_id }` is called on an element with `Some(href)`, actually navigate `state.pure_engine.load_and_parse(&href)` and update `current_url` and `current_snapshot`.

---

## 2. Step-by-Step Task List for Implementing Model

Execute these 5 tasks in order. Run `cargo test` after each task before moving to the next.

---

### Task 1: Fix Non-Overlapping Window Geometry & Custom Protocol Chrome (`crates/zone-engine-web`)

**Files:** `crates/zone-engine-web/src/lib.rs`, `crates/zone-engine-web/src/chrome.rs`

1. In `crates/zone-engine-web/src/lib.rs`:
   - Change `content_bounds(&self)` so the content area starts immediately at `self.sidebar_width` with zero overlap:
     ```rust
     pub fn content_bounds(&self) -> LayoutRect {
         let (w, h) = self.window_size;
         let content_x = self.sidebar_width as f32;
         let content_w = (w.saturating_sub(self.sidebar_width)).max(100) as f32;
         let content_h = h.max(100) as f32;
         LayoutRect::new(content_x, 0.0, content_w, content_h)
     }
     ```
   - In `init_chrome_sidebar`, set the Chrome WebView bounds strictly to `self.sidebar_width` wide (not full window `width`):
     ```rust
     let rect = Rect {
         position: PhysicalPosition::new(0, 0).into(),
         size: PhysicalSize::new(self.sidebar_width, height).into(),
     };
     ```
   - In `handle_resize(&mut self, width: u32, height: u32)`, set the Chrome WebView bounds to `PhysicalSize::new(self.sidebar_width, height)`.
   - In `init_chrome_sidebar`, replace `.with_html(chrome::CHROME_HTML)` with custom protocol `"zone"` and URL `"http://zone.localhost/index.html"`:
     ```rust
     let webview = WebViewBuilder::new()
         .with_bounds(rect)
         .with_custom_protocol("zone".into(), |_id, _req| {
             wry::http::Response::builder()
                 .header("Content-Type", "text/html; charset=utf-8")
                 .header("Access-Control-Allow-Origin", "*")
                 .body(std::borrow::Cow::Borrowed(chrome::CHROME_HTML.as_bytes()))
                 .unwrap()
         })
         .with_url("http://zone.localhost/index.html")
     ```
2. In `crates/zone-engine-web/src/chrome.rs`:
   - Remove `<main class="content-frame-area">...</main>` from the HTML body so `<aside class="sidebar">` fills `100vw` (`width: 100vw; height: 100vh; border-right: 1px solid var(--separator);`).
   - Remove the hardcoded dummy `<div class="tab-row">` HTML inside `<div id="tabList"></div>` and remove the fake `window.__zoneUpdateState({ active_tab_id: "init-2", ... })` call at the bottom of `<script>`, because `"init-1"` is not a valid `Uuid` and causes `serde_json::from_str::<ChromeCommand>` to reject tab clicks.

**Completion Criterion (Task 1):**
- `cargo check` succeeds with 0 warnings.
- Chrome WebView width is `244px` (`0..244`) and Content WebView starts at `244px` (`244..width`).
- No `"init-1"` or `"init-2"` strings exist in `crates/zone-engine-web/src/chrome.rs`.

---

### Task 2: Fix Top-Level URL/Title Sync & Omnibox Navigation (`crates/zone-engine-web`)

**Files:** `crates/zone-engine-web/src/lib.rs`, `crates/zone-engine-web/src/chrome.rs`

1. In `crates/zone-engine-web/src/lib.rs`:
   - Inside `DOM_EXTRACTOR_JS`, add top-level metadata reporting that only runs when `window === window.top`:
     ```javascript
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
         setTimeout(reportMeta, 100);
     }
     ```
   - In `create_tab`'s content `with_ipc_handler`, add a match arm for `Some("page_meta")`:
     ```rust
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
     ```
   - Remove `.with_navigation_handler(...)` from `create_tab` so subframes/ad-frames never overwrite `tab.current_url`.

2. Add a unit test in `crates/zone-engine-web/src/lib.rs` verifying that `serde_json::from_str::<ChromeCommand>` parses:
   - `{"action":"navigate","url":"https://example.com"}`
   - `{"action":"back"}`, `{"action":"forward"}`, `{"action":"reload"}`, `{"action":"new_tab"}`, `{"action":"toggle_split"}`, `{"action":"request_sync"}`
   - `{"action":"switch_tab","id":"550e8400-e29b-41d4-a716-446655440000"}`
   - `{"action":"close_tab","id":"550e8400-e29b-41d4-a716-446655440000"}`

**Completion Criterion (Task 2):**
- `cargo test -p zone-engine-web` passes the new `ChromeCommand` deserialization unit tests.

---

### Task 3: Make Workspaces Actually Switch Tab Groups (`crates/zone-engine-web` & `crates/zone-app`)

**Files:** `crates/zone-engine-web/src/lib.rs`, `crates/zone-engine-web/src/chrome.rs`, `crates/zone-app/src/main.rs`

1. In `crates/zone-engine-web/src/lib.rs`:
   - Add `SwitchWorkspace { name: String }` variant to `ChromeCommand`.
   - Add `pub workspace: String` to `WebTab`, and `pub active_workspace: String` to `WebViewController` (initialized to `"Default Workspace".to_string()`).
   - Add `pub active_workspace: String` to `ChromeStatePayload`.
   - When `create_tab` is called, assign `workspace: self.active_workspace.clone()`.
   - Implement `pub fn switch_workspace(&mut self, window: &Window, name: String)`:
     - Sets `self.active_workspace = name`.
     - Finds the last tab in `self.tab_order` belonging to `self.active_workspace`.
     - If none exists in that workspace yet, calls `self.create_tab(window, TabId::new(), "https://www.google.com", bounds)` so the new workspace always has an active tab.
     - Calls `self.switch_tab(target_id)` and `self.sync_chrome_state()`.
   - In `sync_chrome_state()`, filter `tabs_info` so only tabs where `t.workspace == self.active_workspace` are sent to the sidebar.
2. In `crates/zone-engine-web/src/chrome.rs`:
   - Update `.ws-dot` click handler to send:
     ```javascript
     ipc.postMessage(JSON.stringify({ action: "switch_workspace", name: dot.getAttribute("data-ws") }));
     ```
3. In `crates/zone-app/src/main.rs`:
   - Wire `ChromeCommand::SwitchWorkspace { name } => controller.switch_workspace(&window, name),`.

**Completion Criterion (Task 3):**
- `cargo test` passes and `SwitchWorkspace` deserializes and switches tab groups cleanly.

---

### Task 4: Make Gear 1 Agent Click Actually Follow Links & Track Real Uptime (`crates/zone-core`, `crates/zone-engine-pure`, `crates/zone-agent`)

**Files:** `crates/zone-core/src/lib.rs`, `crates/zone-engine-pure/src/lib.rs`, `crates/zone-agent/src/lib.rs`, `crates/zone-app/src/main.rs`

1. In `crates/zone-core/src/lib.rs`:
   - Add `#[serde(default)] pub href: Option<String>` to `InteractiveElement`.
2. In `crates/zone-engine-pure/src/lib.rs`:
   - When parsing elements in `load_and_parse(url)`, extract `element.value().attr("href")` and resolve it against `url` using `url::Url::parse(url).ok().and_then(|base| base.join(raw_href).ok()).map(|u| u.to_string())`. Store it in `InteractiveElement.href`.
3. In `crates/zone-agent/src/lib.rs`:
   - Add `pub started_at: std::time::Instant` to `AgentState`.
   - In `health_handler`, replace hardcoded `uptime_seconds: 42` with `uptime_seconds: state.started_at.elapsed().as_secs()`.
   - In `action_handler` for `AgentAction::Click { target_id }`:
     - Clone the matched `InteractiveElement` before dropping the read lock on `current_snapshot`.
     - If `target.href` is `Some(target_url)`, call `state.pure_engine.load_and_parse(&target_url).await`, update `state.current_url` and `state.current_snapshot`, and return `ActionResult { success: true, message: format!("Clicked [{}] '{}' -> navigated to {}", target_id, target.text, target_url), new_url: Some(target_url) }`.

**Completion Criterion (Task 4):**
- `cargo test -p zone-agent` passes all unit tests.
- Clicking a link in `/v1/act` navigates to its `href` and updates `/v1/state` URL.

---

### Task 5: End-to-End Build, Visual Check & Test Suite Verification

1. Run `cargo test --workspace` — all tests in `zone-ui`, `zone-engine-web`, and `zone-agent` must pass with 0 warnings.
2. Run `cargo build --release --bin zone`.
3. Launch `zone.exe`, wait 3 seconds, capture the window via Win32 `PrintWindow`, and inspect with `read`:
   - Verify the left `244px` sidebar shows the active tabs (`amirulaliffcherahim/zone`, `Hacker News`) with favicons and titles populated from Rust state.
   - Verify the right content pane renders the live webpage and receives mouse/scroll/click input directly.
4. Commit and push to `origin/main`.
