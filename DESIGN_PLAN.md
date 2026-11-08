# UI & Human Browser Experience Design Plan (`DESIGN_PLAN.md`)

**Objective:** Transform Zone Browser from a developer terminal/TUI prototype into a polished, first-class desktop web browser matching **Zen Browser / Arc** aesthetics and everyday human browsing ergonomics.

---

## 1. Honest Diagnosis: Why the Current Build Looks Like a TUI

1. **Clunky Windows System Titlebar:** The default white/grey Win32 titlebar (`Zone Browser` with standard minimize/maximize/close) clashes violently with the dark sidebar and ruins the borderless, integrated "Arc/Zen" look.
2. **Broken Visual Hierarchy in Sidebar:**
   - Uses ugly Unicode characters (`←`, `→`, `↻`) and raw emoji (`🌐`, `💼`, `💻`) instead of crisp, 1.5px stroke SVG icons (which we already discovered inside Zen's `browser/omni.ja`!).
   - The `TABS` label looks like a terminal debug header.
   - The tab list was empty on initial load because `create_tab` fired before the sidebar DOM finished initializing.
3. **Harsh Content Framing:** In Zen Browser, the web viewport sits inside a subtle **floating rounded card** (`8px` inset margin, `10px` border radius, subtle drop shadow) so the unified dark/gradient theme frames the entire window. Currently, the webview butts harshly against the right/bottom edges with a raw Win32 border.
4. **Missing Core Human Browser Mechanics:**
   - **Real Tab Favicons & Live Titles:** Tabs need the actual website `<title>` and favicon (`https://www.google.com/s2/favicons?domain=...` or DOM `<link rel="icon">`), not raw URLs.
   - **Pinned Essentials Grid (Top of Sidebar):** Zen/Arc puts 4–8 compact icon tiles at the top of the sidebar for daily sites (GitHub, YouTube, Mail, Docs) above the vertical tab strip.
   - **Loading Progress Bar & Security Padlock:** Visual feedback when a page is navigating, plus HTTPS lock status inside the omnibox.
   - **Keyboard Shortcuts Everywhere:** `Ctrl+T` (New Tab), `Ctrl+W` (Close Tab), `Ctrl+L` (Focus URL bar), `Ctrl+Tab` (Next Tab), `Ctrl+R` / `F5` (Reload), `Alt+Left` / `Alt+Right` (Back/Forward) must work even when focus is inside the web content.
   - **`target="_blank"` Link Handling:** Clicking external links that request a new window must open a new Zone tab instead of spawning a broken popup or doing nothing.

---

## 2. Visual & Layout Architecture (Zen "Single-Toolbar" Aesthetic)

```
+-----------------------------------------------------------------------------------+
| [Drag Region]  ● ● ● (Win Controls on Right)                                      |
| +-----------------------+ +-----------------------------------------------------+ |
| | [←] [→] [↻]      [Split]| |                                                     | |
| |                       | |                                                     | |
| | [🔒 Search or URL... ]| |                                                     | |
| |                       | |                                                     | |
| | +---+ +---+ +---+ +---+ |                                                     | |
| | |GH | |HN | |YT | |GM | |              Web Content Viewport                   | |
| | +---+ +---+ +---+ +---+ |           (Inset 8px all around,                    | |
| | ───────────────────── |            rounded corners, framed by                 | |
| | [ + New Tab    Ctrl+T]|            Zen ambient dark surface)                  | |
| |                       | |                                                     | |
| | ● Hacker News      [×]| |                                                     | |
| | ○ GitHub · amirul  [×]| |                                                     | |
| |                       | |                                                     | |
| |                       | |                                                     | |
| | ───────────────────── | |                                                     | |
| | [Space: Personal ▾] ● | |                                                     | |
| +-----------------------+ +-----------------------------------------------------+ |
+-----------------------------------------------------------------------------------+
```

### 2.1 Framing & Window Geometry (Solving the Win32 Airspace Constraint)
Because `WebView2` is a native Win32 child `HWND` that always draws on top of its parent area:
* **Custom Borderless Window (`with_decorations(false)`):** Remove the ugly Windows 11 system titlebar.
* **Full-Window Chrome Background:** The Chrome WebView spans the **entire window** (`0, 0, width, height`), rendering the deep Zen ambient background (`#0f1015` with subtle radial accent glow), the left `250px` sidebar, a `36px` top drag strip with custom Windows minimize/maximize/close SVG buttons, and the `8px` right/bottom frame border.
* **Inset Content WebView (`x: 256, y: 36, w: width - 264, h: height - 44`):** The active content `WebView2` sits cleanly inside the right-hand frame area, leaving the top drag bar and `8px` border visible around it.

---

## 3. Design Tokens & Craft Rules (Following `ui-design` Skill)

* **Surface Mode:** **Operate** — scanability, standard desktop controls, zero emoji icons, high-craft SVG iconography.
* **Color Tokens (CSS Custom Properties):**
  - `--bg-frame`: `#0d0e12` (Outer window & sidebar shell)
  - `--bg-elevated`: `#161821` (Omnibox, Essential tiles, hover surface)
  - `--bg-active-tab`: `rgba(255, 255, 255, 0.09)` (Active pill tab)
  - `--border-subtle`: `rgba(255, 255, 255, 0.07)`
  - `--text-primary`: `#f3f4f6`
  - `--text-secondary`: `#9ca3af`
  - `--accent-primary`: `#818cf8` (Soft indigo/lavender focus ring & progress bar)
* **Typography:** System UI (`Segoe UI Variable Display`, `-apple-system`, `Inter`), `13px` UI controls, `500` medium weight for active tabs, `tabular-nums` for counters.
* **Iconography:** Inline 16×16 crisp SVG icons (`stroke-width="1.75"`, `stroke-linecap="round"`) extracted/adapted from Zen Browser's icon set (Back, Forward, Reload, Lock, Plus, Close, Split, Sidebar Collapse, Window Minimize/Maximize/Close). **Zero emojis.**

---

## 4. Human Browser Feature Checklist (Implementation Steps)

### Step 1: Borderless Window & Custom Titlebar / Drag Region
* Set `.with_decorations(false)` on the Tao `WindowBuilder` (with native resize borders preserved via Win32 styles or IPC commands: `minimize_window`, `maximize_window`, `close_window`, `drag_window` calling `window.drag_window()`).
* Top `36px` bar provides seamless window dragging and standard Windows caption buttons on the top-right.

### Step 2: Rich Sidebar Chrome (Essentials + Omnibox + Vertical Tabs)
* **Top Navigation Row:** SVG Back, Forward, Reload, and Split View toggle buttons.
* **Omnibox Bar:** Security padlock SVG, domain highlighting, clear button, Enter to navigate or Google search, focus-select on click.
* **Essentials Grid (4 Tiles):** Quick-launch icon cards at the top of the sidebar (`GitHub`, `Hacker News`, `Google`, `Rust Docs`) with real favicons.
* **Vertical Tab Strip:**
  - `+ New Tab` button with `Ctrl+T` shortcut badge.
  - Each tab shows: **16×16 Favicon**, **Live Page Title** (updated via `with_document_title_changed_handler` and navigation events), and hover **Close (`×`)** SVG button.
  - Animated top progress bar during page loads.

### Step 3: Real Browser Lifecycle & Shortcut Forwarding
* **Live Title & URL Sync:** Hook `WebViewBuilder::with_navigation_handler` and `with_document_title_changed_handler` so clicking any link inside a webpage immediately updates the omnibox URL, tab title, and favicon in the sidebar.
* **`target="_blank"` Popup Interceptor:** Hook `with_new_window_req_handler` so links that open in a new tab automatically spawn a new Zone tab and switch to it.
* **Content-to-Host Hotkey Forwarding:** Inject a tiny `keydown` listener into content webviews that forwards `Ctrl+T`, `Ctrl+W`, `Ctrl+L`, `Ctrl+R`, and `Ctrl+Tab` over IPC so keyboard shortcuts work even when focus is inside a webpage.

### Step 4: Honest Accounting & Agent Separation
* Fix `uptime_seconds` in `zone-agent` to track real `Instant::now().elapsed().as_secs()`.
* Keep the content IPC channel strictly allowlisted to `dom_extracted` and `hotkey` events; all privileged window/tab actions stay locked to the Chrome WebView's separate IPC handler.

---

## 5. Verification Protocol
1. Build and launch `zone.exe`.
2. Capture a full-window screenshot and inspect visually with `read`:
   - Confirm no white system titlebar.
   - Confirm SVG icons, Essentials grid, active tab favicon + title, and 8px framed web content.
3. Test human flows:
   - Click an Essential tile -> loads site.
   - Click `+ New Tab` -> opens second tab, appears in vertical tab list, switches view.
   - Type a search query in Omnibox -> navigates to Google search results.
   - Click a link on the page -> Omnibox URL and tab title update automatically.
