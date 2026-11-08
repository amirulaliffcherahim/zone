# Reverse Engineering Report: Zen Browser (Desktop)

**Target:** Zen Browser (x64 Windows)  
**Binary Path:** `C:\Program Files\Zen Browser\zen.exe`  
**Installation Path:** `C:\Program Files\Zen Browser`  
**Analysis Provider:** REA (`morluto/rea` v6.7.0) + Ghidra (v12.1.4) + REA Artifact Graph  
**Analysis Date:** 2026-11-08  
**Report Output Dir:** `C:\Users\amiru\Local\zone-browser`

---

## 1. Executive Summary & Build Identity

Zen Browser is a privacy-focused, Arc-inspired browser built directly upon Mozilla Firefox's Gecko engine (`xul.dll`). Rather than wrapping Electron or Chromium, Zen leverages Mozilla's native XRE (XULRunner Runtime Engine) coupled with an extensive custom frontend JavaScript/ES module architecture and customized Gecko layout rules.

### Core Build Metadata (from `application.ini` & `platform.ini`)
* **Vendor:** Mozilla
* **Name / RemotingName:** `Zen` / `zen`
* **Version:** `1.23.3b`
* **Gecko Milestone:** `158.0` (Min/Max: `158.0`)
* **Build ID:** `20261016101044`
* **Source Repository:** `https://github.com/zen-browser/desktop`
* **Source Commit Stamp:** `cba27b90fef1c80096c8531541efe5d7b133baae`
* **App GUID:** `{ec8030f7-c20a-464f-9b0e-13a3a9e97384}`
* **Update URL:** `https://updates.zen-browser.app/updates/browser/%BUILD_TARGET%/%CHANNEL%/update.xml`
* **Update MAR Channel:** `release`

---

## 2. Target Identification & Digests

| Artifact | Format | Size (Bytes) | SHA-256 Digest | Role |
| :--- | :--- | :--- | :--- | :--- |
| `zen.exe` | PE32+ (x86_64) | 1,264,496 | `01B571ED41925AAB52D0D8CCC7F9D5149A379870C044136ADE2293E7A3E8DE4C` | Native stub launcher & pre-XUL skeleton |
| `xul.dll` | PE32+ (x86_64) | 175,971,184 | `9B7A5522A3F5DBC317C223C2E70C948D4E30124D4053301719B4B1783A51BBC9` | Core Gecko/Firefox layout & JS engine |
| `browser\omni.ja`| ZIP Container | 114,221,137 | `2BA776CEA7CCF6E1FF53E752B69DD1A21FEBDBD04B23062521DDDEA8A9351BD7` | Zen custom browser UI (JS, CSS, XUL, icons) |
| `omni.ja` | ZIP Container | 73,017,545 | `0B991475075F881BA01034629B7EB63CE0A3233E1FD4C6726D9168FB95B8FDF1` | Core Gecko XPCOM & toolkit runtime components |
| `mozglue.dll` | PE32+ (x86_64) | 705,392 | `(shipped)` | Memory allocator & platform abstractions |
| `nss3.dll` | PE32+ (x86_64) | 3,244,912 | `(shipped)` | Network Security Services (TLS/crypto) |

---

## 3. REA Evidence Ledger

* **Evidence ID `ev_b36addf7a24edd7bba9d270db9db56d8cc6e0937d3c21d2e2085a1f6645a5c2d`**:  
  `inspect_artifact` on `zen.exe`. Safe artifact graph materialized root node `art_86947eb260d092529e063da10140e324142399ef4b43820ff52241314c2e998e`.
* **Evidence ID `ev_913615ac292d1986d36d2fdbb954c2b1fb1eb4cecae0aa70f273fe2f68e8f80b`**:  
  `inspect_managed_artifact` on `zen.exe`. Proved non-managed native PE binary (`cli-directory-absent`, machine `0x8664`, `characteristics: 34`).
* **Evidence ID `ev_528665f65de2b5a293c4e27e25f5212a5984cac798709ecd9ec31a8d0d99af85`**:  
  `binary_overview` via Ghidra provider on `zen.exe`. Confirmed 10 memory segments, 1,703 procedures, and 986 defined strings.
* **Evidence ID `ev_d1f50cae2422c32a35e555ee140447fda335524e323b161e00a72aea1f90c842`**:  
  `search_strings` on `zen.exe`. Identified Mozilla XUL runner string anchors: `XUL_APP_FILE=%s` (`0x140069c4d`), `CreateAndStorePreXULSkeletonUI` (`0x140071096`), `PollPreXULSkeletonUIEvents` (`0x140071400`).
* **Evidence ID `ev_03512b11c7f99495f9d58319c14eac14369672c9612a640a8c10efb340c4968f`**:  
  `inspect_artifact` on `browser/omni.ja`. Container scan indexed 12,499 unique node artifacts and 13,531 occurrence entries.

---

## 4. Native Binary Analysis (`zen.exe`)

### 4.1 PE Segment Layout (Ghidra auto-analysis)
| Segment | Start VA | End VA | Length (Bytes) | Characteristics |
| :--- | :--- | :--- | :--- | :--- |
| `Headers` | `0x140000000` | `0x140000400` | 1,024 | PE DOS & NT Headers |
| `.text` | `0x140001000` | `0x140063200` | 401,920 | Code / Execution |
| `.rdata` | `0x140064000` | `0x140075400` | 70,656 | Read-only constants & Import tables |
| `.data` | `0x140076000` | `0x140076e28` | 3,624 | Read-write globals |
| `.pdata` | `0x140077000` | `0x14007a000` | 12,288 | Exception table (x64 unwind) |
| `.freestd` | `0x14007a000` | `0x14007a200` | 512 | Free standard linkage |
| `.tls` | `0x14007b000` | `0x14007b200` | 512 | Thread Local Storage |
| `.rsrc` | `0x14007c000` | `0x140136a00` | 764,416 | PE Resources (Icons, Manifest) |
| `.reloc` | `0x140137000` | `0x140137600` | 1,536 | Base relocations |
| `tdb` | `0xff00000000` | `0xff00001850` | 6,224 | Thread data block |

### 4.2 Dynamic Loading & Execution Flow
`zen.exe` acts as a specialized bootstrap launcher:
1. **Pre-XUL Skeleton UI:** Calls `mozilla::CreateAndStorePreXULSkeletonUI` and `mozilla::PollPreXULSkeletonUIEvents` to display an instant launch window before Gecko initializes.
2. **Dependent Libraries Loading Order** (via `dependentlibs.list`):
   ```
   mozglue.dll -> nss3.dll -> gkcodecs.dll -> lgpllibs.dll -> xul.dll
   ```
3. **XRE Bootstrap:** Resolves `XRE_main` in `xul.dll` and transfers execution to Gecko with `application.ini` arguments.

### 4.3 PE Resource Composition
* Total entries examined: 172
* Total resources: 83
* Primary icon groups: 15 distinct resolutions and color depths (including 16x16 up to 128x128 / uncompressed PNGs for Windows high-DPI displays).
* RT_MANIFEST (ID 1, offset 492656, 1692 bytes): Windows Common Controls 6.0, DPI Awareness (Per-Monitor V2), Long Path Awareness.

---

## 5. Frontend & UI Architecture (`browser/omni.ja`)

Zen Browser's differentiating features are housed primarily within `browser/omni.ja`. Out of 13,531 container entries, **976 files** specifically implement Zen's proprietary UI, styling, and functionality.

### 5.1 Architecture Model
* **Startup Hook:** `MozBeforeInitialXULLayout` in `ZenStartup.mjs`.
* **DOM Transformation:** Modifies standard Firefox UI DOM trees dynamically:
  - Reparents `#nav-bar` and `#PersonalToolbar` into `#zen-appcontent-navbar-container`.
  - Shifts the tab notification deck into `#browser` to eliminate vertical tab overlaps.
  - Switches `#tabbrowser-arrowscrollbox` orientation from standard horizontal to `vertical`.
  - Controls initial window opacity with `#zen-browser-background` and `Framer Motion`-style animations (`zen-vendor/motion.min.mjs`).

### 5.2 Key Subsystems & Modules

#### 1. Spaces & Workspaces
* **Core Modules:**
  - `modules/zen/ZenSpaceManager.mjs` (116,475 bytes)
  - `modules/zen/ZenSpace.mjs` (14,903 bytes)
  - `modules/zen/ZenSpacesSwipe.mjs` (9,478 bytes)
  - `modules/zen/ZenSpacesSyncModel.sys.mjs` (25,453 bytes)
  - `modules/zen/ZenSpacesSyncApplier.sys.mjs` (30,694 bytes)
  - `modules/zen/ZenWindowSync.sys.mjs` (57,679 bytes)
* **Functionality:** Implements full workspace partitioning with trackpad swipe gesture transitions, per-space color gradients (`ZenGradientGenerator.mjs`), per-space pinned tabs, and multi-window sync.

#### 2. Split View System
* **Core Modules:**
  - `chrome/browser/content/browser/zen-components/ZenViewSplitter.mjs` (82,309 bytes)
  - `chrome/browser/content/browser/zen-styles/zen-split-view.css` (10,854 bytes)
* **Design:** Implements an arbitrary n-way tree structure (`nsSplitNode` & `nsSplitLeafNode`) supporting nested row and column flexbox splits across active tab sessions.

#### 3. Boosts Engine & Zap (Arc-style DOM/CSS overrides)
* **Core Modules:**
  - `modules/zen/boosts/ZenBoostsManager.sys.mjs` (23,631 bytes)
  - `modules/zen/boosts/ZenBoostsEditor.mjs` (52,203 bytes)
  - `modules/zen/boosts/ZenZapDissolve.sys.mjs` (16,664 bytes)
  - `modules/zen/boosts/ZenSelectorComponent.sys.mjs` (22,757 bytes)
  - `modules/zen/boosts/ZenZapOverlayChild.sys.mjs` (11,731 bytes)
* **Persistence:** Serializes user customizations into compressed LZ4 format (`zen-boosts.jsonlz4`).
* **Zap Dissolve:** Provides an interactive element-picker overlay allowing users to permanently erase DOM nodes on target domains.

#### 4. Glance (Preview Overlay)
* **Core Modules:**
  - `chrome/browser/content/browser/zen-components/ZenGlanceManager.mjs` (60,690 bytes)
  - `actors/ZenGlanceChild.sys.mjs` & `actors/ZenGlanceParent.sys.mjs`
  - `chrome/browser/content/browser/zen-styles/zen-glance.css` (4,401 bytes)
* **Mechanism:** Intercepts link clicks to display ephemeral background iframe overlays scaled at 0.97x with 80-step Arc-bezier easing, preventing full tab creation unless promoted.

#### 5. Live Folders
* **Core Modules:**
  - `modules/zen/ZenLiveFoldersManager.sys.mjs` (18,948 bytes)
  - `modules/zen/GithubLiveFolder.sys.mjs` (11,460 bytes)
  - `modules/zen/RssLiveFolder.sys.mjs` (8,717 bytes)
* **Functionality:** Specialized tab folders that periodically poll GitHub pull requests/issues or RSS feeds to dynamically populate tab items.

#### 6. Zen Mods & Marketplace
* **Core Modules:**
  - `chrome/browser/content/browser/zen-components/ZenMods.mjs` (20,063 bytes)
  - `actors/ZenModsMarketplaceChild.sys.mjs` & `actors/ZenModsMarketplaceParent.sys.mjs`
* **Functionality:** Provides integrated custom script and CSS modification downloads from the Zen community repository.

---

## 6. Telemetry, Settings & Network Boundaries

1. **Auto-Updater:** Queries `https://updates.zen-browser.app/updates/browser/%BUILD_TARGET%/%CHANNEL%/update.xml`.
2. **Local Preferences Stored:**
   - `zen.watermark.enabled`
   - `zen.welcome-screen.seen`
   - `zen.updates.last-build-id`
   - `zen.updates.last-version`
3. **Data LZ4 Files:**
   - `zen-boosts.jsonlz4` (user-created page alterations)

---

## 7. Conclusions & Analysis Boundaries

* **Observed:**
  - Zen Browser is a native Firefox fork (Gecko 158.0).
  - Executable `zen.exe` is a 1.26MB native PE32+ bootstrap stub.
  - The heavy lifting of Gecko resides in `xul.dll` (176MB).
  - All proprietary Zen UI features (Spaces, Split View, Boosts, Glance, Compact Tabs) are implemented via privileged Web/ES modules and XUL/DOM overlays packaged in `browser/omni.ja`.
* **Inferred:**
  - Fast update iteration is achieved by modifying high-level JavaScript and CSS in `omni.ja` without needing to recompile `xul.dll` for every UI fix.
* **Search Boundary & Limitations:**
  - Ghidra analysis performed on `zen.exe`. Decompilation of the 176MB `xul.dll` was bypassed in favor of module inspection since Zen's proprietary UX resides in `browser/omni.ja`.
