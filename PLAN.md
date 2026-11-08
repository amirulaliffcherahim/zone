# Master Plan: Zone Browser (Path B+ Dual-Gear Architecture)

**Project Name:** Zone Browser  
**Architecture:** Path B+ Dual-Gear Pure Rust Engine  
**Binary Footprint:** ~12–15 MB standalone executable  
**Cold Compile Time:** ~30–45 seconds on Windows (standard MSVC / pure `cargo build`)  
**Core Features:** Zen ergonomics (Spaces, Split View, Vertical Tabs, Glance) + Always-on Headless AI Agent Daemon (`browser-use` Set-of-Marks loop)  
**Location:** `C:\Users\amiru\Local\zone-browser`

---

## 1. Executive Architecture: The Dual-Gear Model

Rather than taking on the 45-minute C++ build bottleneck of SpiderMonkey (`mozjs`) or the layout limitations of raw HTML parsers, Zone uses a **Dual-Gear Engine** unified behind a single `BrowserBackend` trait:

```
+-----------------------------------------------------------------------------------+
|                        Zone Browser (~12 MB Pure Rust Binary)                     |
|                                                                                   |
|  +---------------------------------------+  +----------------------------------+  |
|  |         Interactive UI Shell          |  |    On-Board AI Agent Daemon      |  |
|  |   (winit + Slint / egui + WebView2)   |  |   (100% Rust - ON BY DEFAULT)    |  |
|  |  - SplitNode Tree (2-way / 4-way)     |  |  - Gear 1: html5ever + Taffy     |  |
|  |  - Vertical Tabs & Spaces             |  |    + Boa JS (0 C++, <15ms/page)  |  |
|  |  - Uses OS WebView2 (0 MB bundle size)|  |  - Gear 2: Heavy SPA Fallback    |  |
|  |  - 100% Web Compat (React, Turnstile) |  |    via headless WebView2 / CDP   |  |
|  +-------------------+-------------------+  +-----------------+----------------+  |
|                      |                                        |                   |
|                      +-------------------+--------------------+                   |
|                                          |                                        |
|                            +-------------v--------------+                         |
|                            |  Shared Session & Cookie   |                         |
|                            |     Store (zone-core)      |                         |
|                            +----------------------------+                         |
+-----------------------------------------------------------------------------------+
```

### 1.1 Gear 1: Ultra-Light Pure Rust Headless Worker
* **Stack:** `html5ever` + `scraper` + `taffy` (CSS Box Model / Flexbox) + `boa_engine` (pure Rust JS VM).
* **Role:** Primary engine for the on-board AI agent.
* **Characteristics:** Zero C/C++ dependencies. Parses HTML, resolves CSS layout boxes with Taffy, evaluates page scripts with Boa, and returns indexed DOM targets `[0]`, `[1]`, `[2]` in **< 15 milliseconds** using **~8 MB RAM**.
* **SoM Wireframe Screenshot:** Allocates a blank `1280x800` `RgbaImage`, paints light wireframe boxes + labels from `taffy` layout coordinates via `imageproc` (<2ms), and overlays high-contrast indexed badges. Fast, lightweight, zero extra GPU baggage.
* **Workload:** 80% of autonomous agent tasks: search results, documentation, Wikipedia, blogs, GitHub, Reddit, e-commerce catalog scraping.

### 1.2 Gear 2: Heavy SPA & Interactive Viewport (WebView2 / Wry)
* **Stack:** `wry` linking to Windows Evergreen `WebView2Runtime`.
* **Role:** Interactive human browsing tabs + fallback worker for complex client-side rendered SPAs.
* **Native In-Memory IPC (No CDP overhead):** Uses `wry::with_initialization_script()` and `with_ipc_handler()` for direct Rust-memory message passing with zero network websocket latency and zero bot detection signatures.
* **Native Frame Capture:** Calls `ICoreWebView2::CapturePreview` directly into an in-memory PNG stream, piped into the Rust Set-of-Marks badge painter.
* **Airspace & Z-Order Fix:** Split View child `HWND`s use non-overlapping client-relative `PhysicalPosition` bounds (`webview.set_bounds()`). Glance peek preview is spawned as a dedicated top-level child popup window to avoid the Win32 child HWND airspace bug.

### 1.3 Automatic Gear Shift Heuristics
`zone-agent` evaluates a 3-rule heuristic immediately after Gear 1 parse:
1. **Empty SPA Shell:** Root container (`#root`, `#app`, `#__next`) yields `< 3` actionable elements after initial Boa pass -> promote to Gear 2.
2. **Anti-Bot / WAF:** HTTP `403`/`429` or DOM title/text matches Cloudflare (`cf-turnstile`, `Just a moment...`) -> promote to Gear 2.
3. **Explicit Override:** Caller passes `"engine": "auto" | "pure" | "webview"` in `/v1/act` or `/v1/dom`.

---

## 2. Core Crate Workspace Layout

```
zone-browser/
├── Cargo.toml
├── crates/
│   ├── zone-core/             # Shared state: TabId, SpaceId, SplitNode, AgentAction, Cookies
│   ├── zone-agent/            # Dual-gear perception loop, SoM badge painter, axum REST/WS (:4567)
│   ├── zone-engine-pure/      # Gear 1: html5ever + taffy layout + boa_engine runner
│   ├── zone-engine-web/       # Gear 2: wry / WebView2 native controller & CDP bridge
│   ├── zone-ui/               # Slint / egui UI shell: Spaces rail, vertical tabs, split grid
│   └── zone-app/              # Main binary entry point, CLI flags, lifecycle supervisor
├── assets/                    # Icons, theme presets, badge fonts
└── docs/                      # Architecture, API specs
```

---

## 3. Subsystems Specification

### 3.1 On-Board AI Agent Daemon (`zone-agent`)
* **Always-On by Default:** Starts automatically as a background Tokio task on launch. Never steals OS focus or active window coordinates.
* **Perception Engine (browser-use Set-of-Marks):**
  1. **Indexed DOM Extraction:**
     - Computes bounding boxes `(x, y, width, height)` for every visible, clickable, and typable element.
     - Prunes hidden, disabled, and non-actionable elements.
     - Assigns clean integer tags: `[0]`, `[1]`, `[2]`, ...
  2. **Visual Bounding-Box Overlay:**
     - Captures viewport raster buffer (`image::RgbaImage`).
     - Draws high-contrast numbered badges on actionable targets using `imageproc` + `ab_glyph` in pure Rust (<2ms).
     - Exports Base64 PNG for vision models.
  3. **Action Execution:**
     - `click(id)` -> calculates center coordinates, triggers synthetic click event.
     - `input(id, text, clear)` -> focuses input, clears existing value if requested, dispatches keystroke stream.
     - `scroll(direction, pixels)`, `navigate(url)`, `switch_tab(id)`.
* **Loopback Control API (`127.0.0.1:4567`):**
  - Managed by `axum` + `tokio`.
  - Exposes REST endpoints (`/v1/health`, `/v1/state`, `/v1/dom`, `/v1/screenshot`, `/v1/act`, `/v1/handoff`) and a real-time WebSocket event stream (`/v1/stream`).

### 3.2 Zen Ergonomics in Pure Rust (`zone-ui` & `zone-core`)
* **Binary SplitNode Tree:**
  ```rust
  pub enum SplitNode {
      Leaf(TabId),
      Split {
          direction: SplitDirection, // Horizontal | Vertical
          ratio: f32,                // e.g. 0.50
          first: Box<SplitNode>,
          second: Box<SplitNode>,
      },
  }
  ```
  - Calculates normalized viewport scissor rectangles in native Rust.
  - Dynamically resizes child WebView2 viewports with zero lag. Supports 2-way horizontal/vertical and 4-way quadrant grids.
* **Spaces (Workspaces):**
  - In-memory data model storing isolated tab lists, history stacks, theme colors, and pinned essentials per Space.
  - Switched instantly with `Ctrl+1..9` or two-finger trackpad swipe.
* **Vertical Tab Strip:**
  - Left-hand native drawer with smooth collapsing behavior and pin area.
* **Glance (Peek Preview):**
  - Floating modal popover rendered above the active tab to preview links without committing a full tab into the tree.

---

## 4. Shared Dependencies (`Cargo.toml`)

```toml
[workspace]
members = [
    "crates/zone-core",
    "crates/zone-agent",
    "crates/zone-engine-pure",
    "crates/zone-engine-web",
    "crates/zone-ui",
    "crates/zone-app",
]
resolver = "2"

[workspace.dependencies]
# Async & Networking
tokio = { version = "1.43", features = ["full"] }
axum = { version = "0.8", features = ["ws"] }
reqwest = { version = "0.12", features = ["cookies", "gzip", "brotli", "json"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Pure Rust HTML/DOM/CSS & JS Engine
html5ever = "0.29"
markup5ever_rcdom = "0.3"
scraper = "0.22"
taffy = "0.7"
boa_engine = "0.21"
boa_runtime = "0.21"

# Webview & Windowing
wry = "0.49"
tao = "0.31"

# Graphics & Set-of-Marks Image Processing
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }
imageproc = "0.25"
ab_glyph = "0.2"
base64 = "0.22"

# Serialization & Utils
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
async-trait = "0.1"
uuid = { version = "1.11", features = ["v4", "serde"] }
url = { version = "2.5", features = ["serde"] }
cookie_store = "0.21"
```

---

## 5. Phased Implementation Roadmap

### Phase 1: Workspace Scaffolding & Pure-Rust Agent Core (Current)
- [ ] Initialize Cargo workspace with `zone-core`, `zone-engine-pure`, `zone-agent`, `zone-app`.
- [ ] Implement Gear 1 pure-Rust DOM extractor (`html5ever` + `taffy` box model + `boa_engine`).
- [ ] Implement Rust Set-of-Marks badge generator (`imageproc` + `ab_glyph`).
- [ ] Spin up `axum` background daemon on `127.0.0.1:4567` with `/v1/health`, `/v1/dom`, `/v1/act`.
- [ ] Verify unit & integration test: Fetch URL via Gear 1, extract indexed DOM, render visual SoM badges in <25ms.

### Phase 2: Interactive WebView2 Shell & Zen Split View
- [ ] Implement `zone-engine-web` using `wry` / `tao`.
- [ ] Build `SplitNode` binary layout manager for 2-way and 4-way tab tiling.
- [ ] Build left-side Vertical Tab drawer and Spaces switcher in Rust UI.
- [ ] Implement Gear 2 CDP fallback bridge for heavy client-rendered SPAs.

### Phase 3: Session Handoff & Glance Peek
- [ ] Build shared session & cookie synchronizer in `zone-core` (`POST /v1/handoff`).
- [ ] Implement Glance popover (ephemeral child webview over active tab).

### Phase 4: UI Agent Drawer & Local LLM Integration (Completed)
- [x] Add autonomous task runner protocol (`TaskRequest`, `TaskResponse`, `TaskStep`) in `zone-core`.
- [x] Add `/v1/task` and `/v1/task/{id}` routes in `zone-agent` with in-memory task ledger.
- [x] Implement autonomous perception & action loop: navigates, crawls DOM, matches task keywords, executes clicks/inputs.
- [x] Wire up CLI runner: `zone.exe --task "<prompt>"` for instant terminal-driven browser automation.
- [x] Verified live on release build with full execution trace.
