---
name: zone-browser
description: Control and automate the Zone browser through its on-board Dual-Gear headless worker daemon (always on by default, browser-use Set-of-Marks loop). Uses Gear 1 (pure Rust Boa+Taffy) for lightning scraping and Gear 2 (WebView2 CDP) for complex SPAs.
---

# Zone Browser Automation (`zone-browser`)

Zone is a lightweight (~12MB) Rust browser featuring Zen ergonomics (Spaces, Split View, Vertical Tabs) and an **always-on headless AI agent daemon by default**.

The on-board agent uses a **Dual-Gear engine**:
* **Gear 1 (Pure Rust):** `html5ever` + `taffy` + `boa_engine` for <15ms scraping, zero-C++ DOM indexing, and Set-of-Marks badge drawing.
* **Gear 2 (WebView2 CDP):** Automated fallback for heavy React/Next.js SPAs and complex authenticated workflows.

The local daemon listens on `http://127.0.0.1:4567`.

---

## 1. Core REST Endpoints

Base URL: `http://127.0.0.1:4567`

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/v1/health` | Status check: returns active gear (Gear 1 Pure Rust vs Gear 2 WebView2), uptime, memory |
| `GET` | `/v1/state` | Current active tab URL, title, viewport dimensions, gear mode |
| `GET` | `/v1/dom` | Returns structured interactive DOM tree with numeric index tags (`[0]`, `[1]`, `[2]`) |
| `GET` | `/v1/screenshot` | Viewport PNG (base64) with Set-of-Marks numbered bounding box badges drawn by `imageproc` |
| `POST` | `/v1/act` | Execute a discrete agent action (click, input, scroll, key, navigate) |
| `POST` | `/v1/handoff` | Transfer cookies or push/pull tabs between headless worker and human interactive window |

---

## 2. Agent Action Loop (browser-use Pattern)

Always execute browser tasks following this loop:

```
[Observe State]  -->  GET /v1/dom  (+ optional GET /v1/screenshot)
       |
[Decide Action]  -->  Match user goal to interactive element ID [N]
       |
[Execute Action] -->  POST /v1/act { "action": "click", "target_id": N }
       |
[Verify Impact]  -->  GET /v1/state or GET /v1/dom
```

### Action Schemas (`POST /v1/act`)

#### Click Element
```json
{
  "action": "click",
  "target_id": 14
}
```

#### Fill Input / Type
```json
{
  "action": "input",
  "target_id": 3,
  "text": "search query\n",
  "clear_first": true
}
```

#### Navigate URL
```json
{
  "action": "navigate",
  "url": "https://example.com"
}
```

#### Scroll Viewport
```json
{
  "action": "scroll",
  "direction": "down",
  "amount": 600
}
```

#### Key Press
```json
{
  "action": "key",
  "key": "Enter"
}
```

#### Wait / Delay
```json
{
  "action": "wait",
  "ms": 1500
}
```

---

## 3. DOM Tree Inspection Format

`GET /v1/dom` returns cleaned JSON with calculated bounding box coordinates:

```json
{
  "url": "https://news.ycombinator.com",
  "gear": "pure_rust_gear_1",
  "elements": [
    {
      "id": 0,
      "tag": "a",
      "role": "link",
      "text": "Hacker News",
      "bounds": [10, 10, 120, 30]
    },
    {
      "id": 1,
      "tag": "input",
      "role": "searchbox",
      "placeholder": "Search...",
      "bounds": [500, 12, 200, 26]
    }
  ]
}
```

---

## 4. Operational Rules

1. **Headless By Default:** Autonomous tasks run entirely in the background worker. Never spawn visible windows unless the user requests "open in my window" or "show me the tab".
2. **Automatic Gear Switching:** If Gear 1 (Pure Rust) encounters dynamic client-side hydration that requires full browser execution, the daemon flags or escalates the session to Gear 2 (WebView2 CDP) transparently.
3. **Session Handoff:**
   - Push user session to agent: `POST /v1/handoff { "direction": "to_agent" }`.
   - Push agent tab to human window: `POST /v1/handoff { "direction": "to_human", "url": "..." }`.
4. **Safety & Scope:**
   - Never fill payment or authentication credentials autonomously without human review.
