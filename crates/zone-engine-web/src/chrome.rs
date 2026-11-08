pub const CHROME_HTML: &str = r####"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Zen Browser</title>
<style>
  :root {
    --zen-bg-light: #e6e4e2;
    --zen-bg-dark: #1b1b1b;
    --zen-surface-light: rgba(255, 255, 255, 0.58);
    --zen-surface-dark: rgba(255, 255, 255, 0.08);
    --zen-surface-hover-light: rgba(0, 0, 0, 0.055);
    --zen-surface-hover-dark: rgba(255, 255, 255, 0.06);
    --zen-active-tab-light: #ffffff;
    --zen-active-tab-dark: rgba(255, 255, 255, 0.14);
    --zen-text-light: #1c1c1e;
    --zen-text-dark: #f0f0f2;
    --zen-muted-light: rgba(28, 28, 30, 0.52);
    --zen-muted-dark: rgba(240, 240, 242, 0.52);
    --zen-separator-light: rgba(0, 0, 0, 0.08);
    --zen-separator-dark: rgba(255, 255, 255, 0.08);
    --zen-shadow-tab: 0 1px 2.5px rgba(0, 0, 0, 0.08);
    --zen-shadow-content: 0 3px 10px rgba(0, 0, 0, 0.14);

    /* Default to Zen Browser's dark carbon theme (#1b1b1b) matching your live Zen Browser */
    --bg: var(--zen-bg-dark);
    --surface: var(--zen-surface-dark);
    --surface-hover: var(--zen-surface-hover-dark);
    --active-tab: var(--zen-active-tab-dark);
    --text: var(--zen-text-dark);
    --muted: var(--zen-muted-dark);
    --separator: var(--zen-separator-dark);
  }

  [data-theme="light"] {
    --bg: var(--zen-bg-light);
    --surface: var(--zen-surface-light);
    --surface-hover: var(--zen-surface-hover-light);
    --active-tab: var(--zen-active-tab-light);
    --text: var(--zen-text-light);
    --muted: var(--zen-muted-light);
    --separator: var(--zen-separator-light);
  }

  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    user-select: none;
    -webkit-font-smoothing: antialiased;
  }

  body {
    background: var(--bg);
    color: var(--text);
    font-family: "Segoe UI Variable Text", "Segoe UI", -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 13px;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    display: flex;
    flex-direction: row;
  }

  /* Left Zen Navigator Toolbox (Sidebar) */
  .sidebar {
    width: 244px;
    height: 100vh;
    padding: 8px 10px 10px 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex-shrink: 0;
  }

  /* Row 1: Top toolbar buttons (Drag region + Menu + Extensions + Back/Forward/Reload) */
  .top-bar {
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px;
  }

  .top-left, .top-right {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .drag-spacer {
    flex: 1;
    height: 100%;
    cursor: default;
  }

  .icon-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text);
    opacity: 0.65;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.1s ease, opacity 0.1s ease, transform 0.1s ease;
  }

  .icon-btn:hover {
    background: var(--surface-hover);
    opacity: 1;
  }

  .icon-btn:active {
    transform: scale(0.95);
  }

  .icon-btn svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }

  /* Zen 3-dot indicator with green status dot */
  .zen-dots-btn {
    position: relative;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .zen-dots-btn:hover {
    background: var(--surface-hover);
  }
  .zen-dots-status {
    position: absolute;
    top: 5px;
    right: 4px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
  }

  /* Row 2: Zen Pill Urlbar */
  .urlbar-wrapper {
    position: relative;
    width: 100%;
    height: 36px;
    background: var(--surface);
    border-radius: 10px;
    display: flex;
    align-items: center;
    padding: 0 10px;
    gap: 8px;
    transition: background-color 0.12s ease, box-shadow 0.12s ease;
  }

  .urlbar-wrapper:hover {
    background: color-mix(in srgb, var(--surface) 92%, var(--text) 8%);
  }

  .urlbar-wrapper:focus-within {
    background: var(--active-tab);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12), 0 0 0 1.5px rgba(0, 0, 0, 0.12);
  }

  .urlbar-icon {
    width: 14px;
    height: 14px;
    color: var(--muted);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .urlbar-icon svg {
    width: 14px;
    height: 14px;
    fill: currentColor;
  }

  .urlbar-input {
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    font-family: inherit;
    font-size: 13px;
    font-weight: 450;
    outline: none;
  }

  .urlbar-input::placeholder {
    color: var(--muted);
  }

  /* Row 3: Essentials Grid (Pinned Icon Tiles) */
  .essentials-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
    width: 100%;
  }

  .essential-tile {
    height: 44px;
    border-radius: 10px;
    background: var(--surface);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.12s ease, transform 0.1s ease;
  }

  .essential-tile:hover {
    background: var(--active-tab);
  }

  .essential-tile:active {
    transform: scale(0.97);
  }

  .essential-tile img, .essential-tile svg {
    width: 18px;
    height: 18px;
    border-radius: 4px;
  }

  /* Row 4: Workspace Header */
  .workspace-header {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 8px 2px 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    border-radius: 6px;
  }

  .workspace-header:hover {
    color: var(--text);
  }

  .workspace-header svg {
    width: 13px;
    height: 13px;
    fill: currentColor;
  }

  /* Row 5: Vertical Tab List */
  .tabs-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    padding-top: 2px;
  }

  .tab-row {
    height: 34px;
    padding: 0 9px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--text);
    cursor: pointer;
    transition: background-color 0.1s ease;
    position: relative;
  }

  .tab-row:hover {
    background: var(--surface-hover);
  }

  .tab-row.active {
    background: var(--active-tab);
    box-shadow: var(--zen-shadow-tab);
    font-weight: 500;
  }

  .tab-favicon {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .tab-favicon img {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    display: block;
  }

  .tab-favicon svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
    opacity: 0.7;
  }

  .tab-title {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 13px;
  }

  .tab-close {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    border: none;
    background: transparent;
    color: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.1s ease, background-color 0.1s ease;
  }

  .tab-row:hover .tab-close,
  .tab-row.active .tab-close {
    opacity: 0.8;
  }

  .tab-close:hover {
    opacity: 1 !important;
    background: var(--surface-hover);
    color: var(--text);
  }

  .tab-close svg {
    width: 12px;
    height: 12px;
    fill: currentColor;
  }

  /* Thin separator + "+ New Tab" row right inside the list like Zen */
  .tabs-divider {
    height: 1px;
    background: var(--separator);
    margin: 5px 6px;
  }

  .new-tab-row {
    height: 34px;
    padding: 0 9px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--text);
    opacity: 0.75;
    cursor: pointer;
    border: none;
    background: transparent;
    font-family: inherit;
    font-size: 13px;
    width: 100%;
    text-align: left;
    transition: background-color 0.1s ease, opacity 0.1s ease;
  }

  .new-tab-row:hover {
    background: var(--surface-hover);
    opacity: 1;
  }

  .new-tab-row svg {
    width: 15px;
    height: 15px;
    fill: currentColor;
  }

  /* Bottom Footer: Workspace Switcher Dots & Theme Toggle */
  .sidebar-footer {
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px;
    margin-top: auto;
  }

  .workspace-dots {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ws-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
    opacity: 0.45;
    border: none;
    cursor: pointer;
    transition: opacity 0.15s ease, transform 0.15s ease;
  }

  .ws-dot.active {
    opacity: 1;
    background: var(--text);
    transform: scale(1.2);
  }

  /* Right Content Area Frame (The white rounded card frame behind WebView2) */
  .content-frame-area {
    flex: 1;
    height: 100vh;
    padding: 6px 6px 6px 0;
    display: flex;
    flex-direction: column;
  }

  .content-card {
    flex: 1;
    background: #ffffff;
    border-radius: 10px;
    box-shadow: var(--zen-shadow-content);
    position: relative;
    overflow: hidden;
  }
</style>
</head>
<body>
  <aside class="sidebar">
    <!-- Top Row: Menu, Extension, Drag Region, Back, Forward, Reload -->
    <div class="top-bar" id="topDragBar">
      <div class="top-left">
        <button class="zen-dots-btn" id="btnToggleTheme" title="Switch Light/Dark Theme">
          <svg viewBox="0 0 18 18" width="16" height="16" fill="currentColor">
            <circle cx="4" cy="9" r="1.5"/>
            <circle cx="9" cy="9" r="1.5"/>
            <circle cx="14" cy="9" r="1.5"/>
          </svg>
          <span class="zen-dots-status" title="Background Agent Ready"></span>
        </button>
        <button class="icon-btn" id="btnSplitView" title="Toggle Split View">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><path d="M13.25 2A2.75 2.75 0 0 1 16 4.75v8.5A2.75 2.75 0 0 1 13.25 16h-8.5A2.75 2.75 0 0 1 2 13.25v-8.5A2.75 2.75 0 0 1 4.75 2zm-3.5 12.5h3.5c.69 0 1.25-.56 1.25-1.25v-8.5c0-.69-.56-1.25-1.25-1.25h-3.5zm-5-11c-.69 0-1.25.56-1.25 1.25v8.5c0 .69.56 1.25 1.25 1.25h3.5v-11z"/></svg>
        </button>
      </div>

      <div class="drag-spacer" id="dragSpacer"></div>

      <div class="top-right">
        <button class="icon-btn" id="btnBack" title="Back (Alt+Left)">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><path d="M7.47 3.22a.75.75 0 1 1 1.06 1.06L4.56 8.25h10.69a.75.75 0 0 1 0 1.5H4.56l3.97 3.97a.75.75 0 1 1-1.06 1.06L2.22 9.53a.75.75 0 0 1-.165-.812l.004-.01q.018-.04.042-.078a.8.8 0 0 1 .119-.16z"/></svg>
        </button>
        <button class="icon-btn" id="btnForward" title="Forward (Alt+Right)">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><path d="M9.47 3.22a.75.75 0 0 1 1.06 0l5.25 5.25a.8.8 0 0 1 .118.16q.025.04.044.083a.75.75 0 0 1-.078.715 1 1 0 0 1-.084.102l-5.25 5.25a.75.75 0 1 1-1.06-1.06l3.97-3.97H2.75a.75.75 0 0 1 0-1.5h10.69L9.47 4.28a.75.75 0 0 1 0-1.06"/></svg>
        </button>
        <button class="icon-btn" id="btnReload" title="Reload (Ctrl+R)">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><path d="M9 1a7.98 7.98 0 0 1 6.263 3.025l.115-.823a.75.75 0 0 1 1.485.205l-.408 2.946a.75.75 0 0 1-.846.64l-2.943-.407a.75.75 0 0 1 .205-1.486l1.475.203a6.5 6.5 0 1 0 1.151 3.879.75.75 0 0 1 1.5.042A8 8 0 1 1 9 1"/></svg>
        </button>
      </div>
    </div>

    <!-- Omnibox Search / URL Pill -->
    <div class="urlbar-wrapper">
      <div class="urlbar-icon">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><path d="M7.75 1.5A6.25 6.25 0 0 1 14 7.75a6.22 6.22 0 0 1-1.334 3.855l3.614 3.615a.75.75 0 0 1-1.06 1.06l-3.615-3.614A6.22 6.22 0 0 1 7.75 14a6.25 6.25 0 0 1 0-12.5m0 1.5a4.75 4.75 0 1 0 0 9.5 4.75 4.75 0 0 0 0-9.5"/></svg>
      </div>
      <input
        type="text"
        class="urlbar-input"
        id="omnibox"
        placeholder="Search or enter address"
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <!-- Essentials Pinned Tiles (2-Column Zen Grid) -->
    <div class="essentials-grid">
      <button class="essential-tile" data-url="https://github.com/amirulaliffcherahim" title="GitHub">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><path d="M12 .297c-6.63 0-12 5.373-12 12c0 5.303 3.438 9.8 8.205 11.385c.6.113.82-.258.82-.577c0-.285-.01-1.04-.015-2.04c-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729c1.205.084 1.838 1.236 1.838 1.236c1.07 1.835 2.809 1.305 3.495.998c.108-.776.417-1.305.76-1.605c-2.665-.3-5.466-1.332-5.466-5.93c0-1.31.465-2.38 1.235-3.22c-.135-.303-.54-1.523.105-3.176c0 0 1.005-.322 3.3 1.23c.96-.267 1.98-.399 3-.405c1.02.006 2.04.138 3 .405c2.28-1.552 3.285-1.23 3.285-1.23c.645 1.653.24 2.873.12 3.176c.765.84 1.23 1.91 1.23 3.22c0 4.61-2.805 5.625-5.475 5.92c.42.36.81 1.096.81 2.22c0 1.606-.015 2.896-.015 3.286c0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>
      </button>
      <button class="essential-tile" data-url="https://news.ycombinator.com" title="Hacker News">
        <svg viewBox="0 0 24 24" width="18" height="18"><rect width="24" height="24" rx="4" fill="#ff6600"/><path d="M7 6h2.2l2.8 5.6L14.8 6H17l-4 7.5V18h-2v-4.5L7 6z" fill="#fff"/></svg>
      </button>
    </div>

    <!-- Workspace Indicator -->
    <div class="workspace-header" id="workspaceHeader">
      <svg viewBox="0 0 512 512"><path d="M 496,152 A 56,56 0 0 0 440,96 H 220.11 A 23.89,23.89 0 0 1 206.8,92 L 179,73.41 A 55.77,55.77 0 0 0 147.89,64 H 72 a 56,56 0 0 0 -56,56 v 48 a 8,8 0 0 0 8,8 h 464 a 8,8 0 0 0 8,-8 z M 16,392 a 56,56 0 0 0 56,56 h 368 a 56,56 0 0 0 56,-56 V 216 a 8,8 0 0 0 -8,-8 H 24 a 8,8 0 0 0 -8,8 z"/></svg>
      <span id="workspaceName">Default Workspace</span>
    </div>

    <!-- Vertical Tab Strip + Inline "+ New Tab" -->
    <div class="tabs-section">
      <div id="tabList">
        <div class="tab-row" data-url="https://github.com/amirulaliffcherahim/zone">
          <div class="tab-favicon">
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><path d="M12 .297c-6.63 0-12 5.373-12 12c0 5.303 3.438 9.8 8.205 11.385c.6.113.82-.258.82-.577c0-.285-.01-1.04-.015-2.04c-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729c1.205.084 1.838 1.236 1.838 1.236c1.07 1.835 2.809 1.305 3.495.998c.108-.776.417-1.305.76-1.605c-2.665-.3-5.466-1.332-5.466-5.93c0-1.31.465-2.38 1.235-3.22c-.135-.303-.54-1.523.105-3.176c0 0 1.005-.322 3.3 1.23c.96-.267 1.98-.399 3-.405c1.02.006 2.04.138 3 .405c2.28-1.552 3.285-1.23 3.285-1.23c.645 1.653.24 2.873.12 3.176c.765.84 1.23 1.91 1.23 3.22c0 4.61-2.805 5.625-5.475 5.92c.42.36.81 1.096.81 2.22c0 1.606-.015 2.896-.015 3.286c0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>
          </div>
          <div class="tab-title">amirulaliffcherahim/zone</div>
          <button class="tab-close" title="Close Tab">
            <svg viewBox="0 0 18 18"><path d="M13.47 3.47a.75.75 0 1 1 1.06 1.06L10.06 9l4.47 4.47a.75.75 0 1 1-1.06 1.06L9 10.06l-4.47 4.47a.75.75 0 0 1-1.06-1.06L7.94 9 3.47 4.53a.75.75 0 1 1 1.06-1.06L9 7.94z"/></svg>
          </button>
        </div>
        <div class="tab-row active" data-url="https://news.ycombinator.com">
          <div class="tab-favicon">
            <svg viewBox="0 0 24 24" width="16" height="16"><rect width="24" height="24" rx="4" fill="#ff6600"/><path d="M7 6h2.2l2.8 5.6L14.8 6H17l-4 7.5V18h-2v-4.5L7 6z" fill="#fff"/></svg>
          </div>
          <div class="tab-title">Hacker News</div>
          <button class="tab-close" title="Close Tab">
            <svg viewBox="0 0 18 18"><path d="M13.47 3.47a.75.75 0 1 1 1.06 1.06L10.06 9l4.47 4.47a.75.75 0 1 1-1.06 1.06L9 10.06l-4.47 4.47a.75.75 0 0 1-1.06-1.06L7.94 9 3.47 4.53a.75.75 0 1 1 1.06-1.06L9 7.94z"/></svg>
          </button>
        </div>
      </div>
      <div class="tabs-divider"></div>
      <button class="new-tab-row" id="btnNewTab">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18"><path d="M9 2.5a.75.75 0 0 1 .75.75v5h5a.75.75 0 0 1 0 1.5h-5v5a.75.75 0 0 1-1.5 0v-5h-5a.75.75 0 0 1 0-1.5h5v-5A.75.75 0 0 1 9 2.5"/></svg>
        <span>New Tab</span>
      </button>
    </div>

    <!-- Bottom Minimal Workspace Dots -->
    <div class="sidebar-footer">
      <div class="workspace-dots">
        <button class="ws-dot active" data-ws="Default Workspace" title="Default Workspace"></button>
        <button class="ws-dot" data-ws="Work" title="Work"></button>
        <button class="ws-dot" data-ws="Research" title="Research"></button>
      </div>
    </div>
  </aside>

  <!-- Right Framed Content Area -->
  <main class="content-frame-area">
    <div class="content-card"></div>
  </main>

  <script>
    const ipc = {
      postMessage: (msg) => {
        if (window.ipc && typeof window.ipc.postMessage === 'function') {
          window.ipc.postMessage(msg);
        } else if (window.chrome && window.chrome.webview && typeof window.chrome.webview.postMessage === 'function') {
          window.chrome.webview.postMessage(msg);
        }
      }
    };
    const omnibox = document.getElementById('omnibox');
    const tabList = document.getElementById('tabList');
    const workspaceName = document.getElementById('workspaceName');

    const globeSvg = `<svg viewBox="0 0 18 18"><path d="M9 1a8 8 0 1 0 0 16A8 8 0 0 0 9 1m0 1.5a6.5 6.5 0 0 1 5.15 2.55H3.85A6.5 6.5 0 0 1 9 2.5M2.63 6.55h12.74a6.5 6.5 0 0 1 0 4.9H2.63a6.5 6.5 0 0 1 0-4.9M9 15.5a6.5 6.5 0 0 1-5.15-2.55h10.3A6.5 6.5 0 0 1 9 15.5"/></svg>`;
    const closeSvg = `<svg viewBox="0 0 18 18"><path d="M13.47 3.47a.75.75 0 1 1 1.06 1.06L10.06 9l4.47 4.47a.75.75 0 1 1-1.06 1.06L9 10.06l-4.47 4.47a.75.75 0 0 1-1.06-1.06L7.94 9 3.47 4.53a.75.75 0 1 1 1.06-1.06L9 7.94z"/></svg>`;

    // Theme switch (Dark carbon vs Light paper)
    document.getElementById('btnToggleTheme').onclick = () => {
      const current = document.documentElement.getAttribute('data-theme');
      document.documentElement.setAttribute('data-theme', current === 'light' ? 'dark' : 'light');
    };

    document.getElementById('btnBack').onclick = () => ipc.postMessage(JSON.stringify({ action: 'back' }));
    document.getElementById('btnForward').onclick = () => ipc.postMessage(JSON.stringify({ action: 'forward' }));
    document.getElementById('btnReload').onclick = () => ipc.postMessage(JSON.stringify({ action: 'reload' }));
    document.getElementById('btnNewTab').onclick = () => ipc.postMessage(JSON.stringify({ action: 'new_tab' }));
    document.getElementById('btnSplitView').onclick = () => ipc.postMessage(JSON.stringify({ action: 'toggle_split' }));

    // Essentials tiles
    document.querySelectorAll('.essential-tile').forEach(tile => {
      tile.onclick = () => {
        const url = tile.getAttribute('data-url');
        if (url) ipc.postMessage(JSON.stringify({ action: 'navigate', url }));
      };
    });

    // Workspace dots
    document.querySelectorAll('.ws-dot').forEach(dot => {
      dot.onclick = () => {
        document.querySelectorAll('.ws-dot').forEach(d => d.classList.remove('active'));
        dot.classList.add('active');
        workspaceName.textContent = dot.getAttribute('data-ws');
      };
    });

    omnibox.addEventListener('focus', () => {
      if (omnibox.dataset.fullUrl) {
        omnibox.value = omnibox.dataset.fullUrl;
      }
      omnibox.select();
    });
    omnibox.addEventListener('blur', () => {
      if (omnibox.dataset.fullUrl && omnibox.value === omnibox.dataset.fullUrl) {
        omnibox.value = formatDisplayUrl(omnibox.dataset.fullUrl);
      }
    });

    omnibox.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        let val = omnibox.value.trim();
        if (val) {
          if (!val.startsWith('http://') && !val.startsWith('https://')) {
            val = (val.includes('.') && !val.includes(' '))
              ? 'https://' + val
              : 'https://www.google.com/search?q=' + encodeURIComponent(val);
          }
          ipc.postMessage(JSON.stringify({ action: 'navigate', url: val }));
          omnibox.blur();
        }
      }
    });

    window.addEventListener('keydown', (e) => {
      if (e.ctrlKey && e.key.toLowerCase() === 't') {
        e.preventDefault();
        ipc.postMessage(JSON.stringify({ action: 'new_tab' }));
      } else if (e.ctrlKey && e.key.toLowerCase() === 'l') {
        e.preventDefault();
        omnibox.focus();
        omnibox.select();
      } else if (e.ctrlKey && e.key.toLowerCase() === 'r') {
        e.preventDefault();
        ipc.postMessage(JSON.stringify({ action: 'reload' }));
      }
    });

    function getFaviconUrl(urlStr) {
      try {
        const u = new URL(urlStr);
        if (u.protocol === 'http:' || u.protocol === 'https:') {
          return `https://www.google.com/s2/favicons?domain=${u.hostname}&sz=32`;
        }
      } catch (_) {}
      return null;
    }

    function formatDisplayUrl(urlStr) {
      try {
        const u = new URL(urlStr);
        return u.hostname.replace(/^www\./, '') + (u.pathname !== '/' ? u.pathname : '');
      } catch (_) {
        return urlStr;
      }
    }

    window.__zoneUpdateState = function(state) {
      if (!state) return;
      if (state.active_url && document.activeElement !== omnibox) {
        // Show clean domain or URL in omnibox when unfocused, full URL when focused
        omnibox.dataset.fullUrl = state.active_url;
        omnibox.value = formatDisplayUrl(state.active_url);
      }
      if (state.tabs) {
        tabList.innerHTML = '';
        state.tabs.forEach(t => {
          const row = document.createElement('div');
          row.className = 'tab-row' + (t.id === state.active_tab_id ? ' active' : '');
          row.onclick = () => ipc.postMessage(JSON.stringify({ action: 'switch_tab', id: t.id }));

          const fav = document.createElement('div');
          fav.className = 'tab-favicon';
          const favUrl = getFaviconUrl(t.url);
          if (favUrl) {
            const img = document.createElement('img');
            img.src = favUrl;
            img.onerror = () => { fav.innerHTML = globeSvg; };
            fav.appendChild(img);
          } else {
            fav.innerHTML = globeSvg;
          }

          const title = document.createElement('div');
          title.className = 'tab-title';
          title.textContent = (t.title && t.title !== t.url) ? t.title : formatDisplayUrl(t.url || 'New Tab');

          const closeBtn = document.createElement('button');
          closeBtn.className = 'tab-close';
          closeBtn.innerHTML = closeSvg;
          closeBtn.title = 'Close Tab (Ctrl+W)';
          closeBtn.onclick = (e) => {
            e.stopPropagation();
            ipc.postMessage(JSON.stringify({ action: 'close_tab', id: t.id }));
          };

          row.appendChild(fav);
          row.appendChild(title);
          row.appendChild(closeBtn);
          tabList.appendChild(row);
        });
      }
    };

    // Initial seed state before first Rust sync
    window.__zoneUpdateState({
      active_tab_id: "init-2",
      active_url: "https://news.ycombinator.com",
      tabs: [
        { id: "init-1", title: "amirulaliffcherahim/zone · GitHub", url: "https://github.com/amirulaliffcherahim/zone" },
        { id: "init-2", title: "Hacker News", url: "https://news.ycombinator.com" }
      ]
    });

    document.addEventListener("DOMContentLoaded", () => {
      ipc.postMessage(JSON.stringify({ action: "request_sync" }));
    });
    // Poll briefly during startup so initial tabs always appear even if Chrome finishes loading after Rust creates tabs
    for (const delay of [50, 150, 400, 800, 1500, 3000]) {
      setTimeout(() => {
        ipc.postMessage(JSON.stringify({ action: "request_sync" }));
      }, delay);
    }
  </script>
</body>
</html>
"####;
