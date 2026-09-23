# Island Plugin System — Design

**Status:** accepted · **API 2 IPC implementation; Wasm/GPU extensions planned**

**Stack choice:** **Option E (hybrid host)** · **Presentation model 3 (full custom view)** · **Marketplace M1 (curated GitHub index)**  
**Date:** 2026-08-12  
**Public SDK:** [Sullivansome/island-plugin-sdk](https://github.com/Sullivansome/island-plugin-sdk)

This document is binding for agents and humans. Implementation may lag; do not invent a second plugin model without updating this file.


**Implementation status (2026-09-08):** API **2** authenticates host-spawned IPC sessions with per-launch credentials and enforces manifest grants for host operations. The host owns shutdown, child reaping, runtime discovery and package locks. Both GPUI and GTK consume shared core/design state and forward pointer, scroll, keyboard, focus and expansion events. Remote IPC tarballs require SHA-256, identity/version validation and explicit permission acceptance. Native executables are **not OS-sandboxed**. Wasm, shared-memory/GPU transport, embedded surfaces and a full plugin accessibility tree remain accepted future extensions, not current features. See [architecture-review.md](architecture-review.md) for verification limits.

---

## 1. Goals

1. **Third parties extend Island without forking or editing our source.**
2. **Full creative control** over expanded (and optionally collapsed) visual content.
3. **Host still owns** window chrome, expand/collapse motion, z-order, notch geometry, queue arbitration, permissions, and design-system *frame*.
4. **Multi-GUI future** (GPUI + Swift / WinUI 3 / Qt|GTK) shares one plugin protocol.
5. **Marketplace** is a **curated index** (M1), not a closed binary store (yet).

Non-goals (v1):

- Payments / paid listings  
- Automatic notarization of every community binary  
- Unrestricted process injection into other apps  
- Replacing the host island shell with Electron/Flutter

---

## 2. Architecture (Option E)

```text
┌──────────────────────────────────────────────────────────────────┐
│                         island-core                              │
│  registry · permission gate · presentation queue · lifecycle     │
│  design frame (size, radius, expand factor) · event bus          │
└────────────────────────────┬─────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
   IPC plugins          Wasm plugins         Built-in plugins
   (out-of-process)     (sandboxed guest)    (in-tree, same API)
        │                    │                    │
        └────────────────────┼────────────────────┘
                             │  Plugin API (versioned)
                             ▼
                    View Surface protocol
                    (full custom view transport)
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
           GPUI shell    Swift shell    WinUI / Qt|GTK
```

| Runtime tier | Who | Load model | Trust |
|--------------|-----|------------|-------|
| **Built-in** | First-party packages that dogfood the Plugin API | Linked into host or `plugins/builtin/` | Full |
| **IPC host** | Community / partners (any language) | Separate process, authenticated loopback TCP | User-granted permissions |
| **Wasm guest** | Untrusted / index defaults | `wasmtime` (or equiv.) in host | Capability imports only |

**Rule:** every tier implements the **same logical Plugin API**. Built-ins dogfood the API. No “secret” path that only first-party UI can use—except OS privileges the sandbox cannot grant.

---

## 3. Presentation model 3 — Full custom view

### 3.1 What “full custom view” means

A plugin owns **pixels and interaction inside the island content rect** (and optionally the collapsed strip). The host owns:

| Host-owned | Plugin-owned |
|------------|--------------|
| Outer pill chrome (optional frame) | Everything *inside* the content viewport |
| Expand/collapse animation of the **host window / frame** | Internal micro-interactions |
| Hit region of the island shell, outside-click collapse | Pointer/keyboard inside the view surface |
| Queue: who is visible, priority, TTL | When to request present / dismiss |
| Permissions, kill switch, “Managed by X” | Business logic, network, own process state |

Plugins **do not** create their own always-on-top island window by default (avoids duplicate notches). They render into a **host-provided surface**.

### 3.2 View surface (transport)

Cross-toolkit constraint: GPUI, Swift, WinUI, and Qt/GTK cannot share one widget tree. Full custom view is therefore defined as a **portable surface protocol**, not “return a GPUI `AnyElement`.”

#### Primary transport: **Shared surface / frame buffer** (v1 target)

```text
Plugin (or Wasm guest)
    │  produces frames: BGRA8 or opaque handle
    ▼
ViewSurface {
  width, height, scale,
  dirty rects,
  frame_id,
  input: pointer/key events from host → plugin
}
    │
    ▼
Host composites texture into island content rect (GPU or CPU blit)
```

| Backend | Mechanism |
|---------|-----------|
| **IPC native plugin** | Shared memory ring + vsync-ish tick, or platform IOSurface / DXGI shared handle / dmabuf (phase 2) |
| **Wasm** | Guest writes into linear memory framebuffer; host uploads texture each frame (cap FPS) |
| **Built-in** | May short-circuit: render GPUI subtree *as if* it were a surface (adapter), same queue API |

#### Secondary transport: **Child process window embed** (optional, advanced)

Plugin opens a borderless window; host reparents / positions it into the content rect (macOS: child window; Win: `SetParent`; Linux: embed or layer-shell attach).  

Use only when shared-buffer is insufficient (complex native toolkits inside the plugin). Still subject to permissions (`surface.embed`).

#### Explicitly deferred

| Idea | Why later |
|------|-----------|
| Plugin returns GPUI `Div` types | Ties ecosystem to one toolkit; breaks Swift/WinUI |
| Unrestricted WebView for all plugins | Heavy; XSS surface; design drift |
| Plugin draws *outside* host frame | Duplicates island; fights notch math |

WebView may appear later as **one built-in surface backend** (`surface.kind = "html"`), not the only model.

### 3.3 Coordinate & sizing contract

```text
Host announces:
  content_size: { w, h }     // logical px of content rect
  scale: f32                 // backing scale
  expand: 0.0..=1.0          // animation progress (plugin may cross-fade)
  phase: collapsed | expanding | expanded | collapsing

Plugin may request:
  preferred_size: { w, h }   // host clamps to design max
  min_size / max_size
```

Host design tokens still cap max expanded size (e.g. 380×160 default, configurable). Plugins cannot force a 4K overlay.

### 3.4 Input

Host forwards into the focused surface:

- pointer move/down/up, scroll (logical-point deltas; positive x/y scroll the viewport right/down)
- optional key events when island is key  
- `action:` callbacks if plugin registers chrome buttons in the **frame** (optional host-drawn title affordances)

Plugin returns:

- `cursor` hint  
- `consume` vs `bubble` (bubble → host may treat as drag/collapse)

### 3.5 Collapsed vs expanded

| Mode | Custom view |
|------|-------------|
| **Expanded** | Full custom view (required for “model 3” plugins) |
| **Collapsed** | Either host-drawn glyph strip from plugin metadata **or** small custom view (max height = collapsed pill) |

Manifest declares:

```toml
[view]
expanded = "custom"
collapsed = "metadata"   # or "custom"
```

---

## 4. Plugin package layout

```text
com.example.awesome/
  island-plugin.toml      # required manifest
  README.md
  icon.png                # 256² market + UI
  # exactly one primary entry (declared in manifest):
  bin/island-plugin-…     # IPC executable  OR
  plugin.wasm             # Wasm guest     OR
  (builtin only)          # linked in host
  assets/…                # optional
```

### 4.1 Manifest (`island-plugin.toml`)

```toml
api_version = 2

[plugin]
id = "com.example.awesome"          # reverse-DNS, immutable
name = "Awesome"
version = "0.1.0"
description = "Does something delightful in the island"
authors = ["Example <hi@example.com>"]
license = "MIT"
homepage = "https://github.com/example/awesome-island"
repository = "https://github.com/example/awesome-island"

[entry]
# one of:
kind = "ipc"                        # "ipc" | "wasm" | "builtin"
command = "bin/awesome-island"      # ipc: relative to package root
# wasm_path = "plugin.wasm"
# builtin_id = "music"              # first-party only

[view]
expanded = "custom"
collapsed = "metadata"
preferred_expanded = { w = 360, h = 140 }

[permissions]
# empty = present-only defaults
grant = [
  "present",
  "surface.framebuffer",
  # "surface.embed",
  # "net",
  # "net:api.github.com",
  # "fs:read",
  # "clipboard:read",
  # "media",
  # "shell:open-url",
]

[market]
categories = ["productivity", "media"]
screenshots = ["market/shot1.png"]
```

### 4.2 Semantic versioning

- **`api_version`**: host plugin protocol major. Host requires an exact major match.
- **`plugin.version`**: plugin semver for updates in the index.  
- Breaking changes to frame protocol bump `api_version`.

---

## 5. Plugin API (logical)

Language-agnostic; IPC uses JSON Lines or MessagePack length-prefixed frames. Wasm maps to host imports/exports with the same names.

### 5.1 Host → Plugin

| Message | Purpose |
|---------|---------|
| `Hello { api_version, host_version, instance_id }` | Handshake |
| `Configure { permissions, data_dir }` | Sandbox paths |
| `SurfaceReady { surface_id, w, h, scale }` | Content rect ready |
| `SurfaceResize { w, h, scale }` | Host resized |
| `ExpandState { expand, phase }` | Animation tick |
| `Input { … }` | Pointer/key |
| `Focus { focused: bool }` | |
| `OpenUrlResult { request_id, error? }` | Launch result, only for requests that supplied an ID |
| `Tick { time_ms }` | Optional; prefer plugin-driven frames |
| `Shutdown` | Unload |

### 5.2 Plugin → Host

| Message | Purpose |
|---------|---------|
| `HelloOk { api_version, name, token }` | Host-issued per-launch credential |
| `Present { priority, ttl_ms?, reason }` | Request visibility in queue |
| `Dismiss` | Yield island |
| `SetCollapsedMeta { leading, trailing, dot }` | If collapsed = metadata |
| `PreferredSize { w, h }` | Soft request |
| `Frame { surface_id, frame_id, format, … }` | Submit custom view pixels / handle |
| `SetCursor { name }` | |
| `OpenUrl { url, request_id? }` | Requires `shell:open-url`; an ID opts into a launch result |
| `Log { level, msg }` | Host log pane |
| `RequestPermission { … }` | Optional runtime prompt |

### 5.3 Queue arbitration

```text
priority: u8        # 0–100, higher wins
ttl_ms: optional    # auto-dismiss
sticky: bool        # survives until Dismiss (needs permission "present.sticky")
```

Host policy (defaults):

1. Higher priority preempts lower.  
2. Same priority → round-robin or most recent `Present`.  
3. User can pin “focus plugin” in settings later.  
4. Built-in emergency (permission prompt) can preempt everything.

---

## 6. Security model

Full custom view is powerful; treat IPC plugins as **semi-trusted code**.

| Control | Behavior |
|---------|----------|
| **Permissions** | Enforced in core; surface/frame APIs fail closed |
| **Process isolation** | IPC plugins are separate processes; crash ≠ host crash |
| **Wasm** | No FS/net unless imported; fuel/time limits; max framebuffer size |
| **Kill switch** | Host tray: Disable plugin / Uninstall |
| **Provenance** | Index records `sha256`, source repo, signed release optional |
| **Frame caps** | Max resolution, max FPS (e.g. 30), max shared-memory bytes |
| **No ambient authority** | Plugin home directory only under `…/Island/plugins/<id>/data` |

### 6.1 Index trust (M1)

- Listing requires a PR to the **curated index** repo (or `registry/` in-monorepo for bootstrap).  
- Maintainers review: manifest, permissions, source URL, hash.  
- Installer verifies hash before enable.  
- “Verified” badge = optional maintainers’ meta flag (not full notarization).

Wasm should be the **default recommendation** for new untrusted listings; IPC allowed when justified (`surface.embed`, native SDK).

---

## 7. Marketplace M1 — Curated index

### 7.1 Shape

Separate git repo (preferred) or `registry/` subtree:

```text
island-plugins-index/          # e.g. Sullivansome/island-plugins-index
  README.md
  index/v1.json                # bulk catalog for clients
  plugins/
    com.example.awesome.json   # per-plugin metadata + releases
```

### 7.2 `index/v1.json` (sketch)

```json
{
  "api_version": 1,
  "generated_at": "2026-08-12T00:00:00Z",
  "plugins": [
    {
      "id": "com.example.awesome",
      "name": "Awesome",
      "description": "…",
      "categories": ["media"],
      "latest": "0.1.0",
      "entry_kinds": ["ipc", "wasm"],
      "permissions_max": ["present", "surface.framebuffer", "net"],
      "repository": "https://github.com/example/awesome-island",
      "meta_path": "plugins/com.example.awesome.json"
    }
  ]
}
```

### 7.3 Per-plugin file

```json
{
  "id": "com.example.awesome",
  "releases": [
    {
      "version": "0.1.0",
      "published_at": "2026-08-12",
      "api_version": 2,
      "artifacts": [
        {
          "os": "macos",
          "arch": "aarch64",
          "kind": "ipc",
          "url": "https://github.com/example/awesome-island/releases/download/v0.1.0/awesome-macos-arm64.tar.gz",
          "sha256": "…"
        },
        {
          "os": "any",
          "arch": "wasm32",
          "kind": "wasm",
          "url": "https://…/plugin.wasm",
          "sha256": "…"
        }
      ]
    }
  ]
}
```

### 7.4 Client commands and Market

```bash
island-cli plugin search <query>
island-cli plugin info <id>
island-cli plugin install <id>[@version]
island-cli plugin update [id] --accept-permissions
island-cli plugin list
island-cli plugin disable <id>
island-cli plugin remove <id>
```

Install flow:

1. Fetch `index/v1.json` (cached).  
2. Resolve artifact for current OS/arch (prefer wasm if both exist and user default is safe).  
3. Download → sha256 → extract under user plugins dir.  
4. Prompt permissions from manifest.  
5. Register + optional first launch.

Default plugin directories:

| OS | Path |
|----|------|
| macOS | `~/Library/Application Support/Island/plugins/` |
| Linux | `~/.local/share/Island/plugins/` |
| Windows | `%APPDATA%\Island\plugins\` |

Index URL default: `https://raw.githubusercontent.com/Sullivansome/island-plugin-sdk/main/registry/index/v1.json` (overridable).

### 7.5 Authoring path (no core fork)

1. `cargo generate` / `npm` / any stack → implement IPC or Wasm using **published `island-plugin` protocol crate / docs**.  
2. Ship GitHub Release artifacts.  
3. Open PR to **island-plugins-index** with metadata + hashes.  
4. After merge, users `island-cli plugin install com.you.thing`.

---

## 8. SDK surface (what we publish)

| Crate / package | Role |
|-----------------|------|
| `island-plugin-api` | Types, messages, api_version (no GUI) |
| `island-plugin-sdk` | IPC client helper (Rust): connect, present, frame submit |
| `island-plugin-sdk-wasm` | Guest bindings + framebuffer helpers |
| Docs site / `docs/plugin-authoring.md` | Non-Rust authors (JSON protocol + examples in Swift/Python) |

Swift/WinUI shells consume the **same IPC** as external plugins when talking to core; they are not a separate plugin ABI.

---

## 9. Host integration (current tree → target)

### 9.1 Today

Rust workspace: `island-core` owns queue/expiry/expansion; `island-plugin-api` defines the portable protocol; host and SDK exchange length-prefixed JSON/BGRA frames. The GPUI shell composites custom surfaces, with Spotify as the active stock integration. Authenticated permissions and verified remote IPC archives are implemented. Wasm and shared-memory/GPU surfaces remain planned. See [architecture-review.md](architecture-review.md).

### 9.2 Target modules

```text
crates/
  island-core/           # queue, permissions, registry, expand policy
  island-plugin-api/
  island-plugin-host/    # IPC server, Wasm runtime, surface compositor hooks
  island-plugin-sdk/
apps/
  island-gpui/           # current binary
registry/                # optional bootstrap index until separate repo
```

### 9.3 GPUI compositor hook

```text
DynamicIsland content rect
  → if the active presentation has a Frame
       sample latest Frame texture
       draw as image/quad inside pill
  → else empty chrome (collapsed metadata only)
```

First-party integrations use the same frame protocol as third-party IPC plugins.

---

## 10. Phased delivery

| Phase | Deliverable | Done when |
|-------|-------------|-----------|
| **P0** | Extract `island-core` presentation queue + capability enforcement | ✅ `crates/island-core` · built-ins on queue |
| **P1** | IPC protocol + `island-plugin-sdk` + host loader (no Wasm) | ✅ length-prefixed JSON/frames · `island-plugin-host` |
| **P2** | **Framebuffer custom view** (BGRA frames) | ✅ binary frames + GPUI `RenderImage` composite |
| **P3** | Input forwarding + collapsed metadata | ✅ metadata, pointer/scroll/key events, focus, expansion broadcasts; native labels and transport controls |
| **P4** | CLI install from **local folder** | ✅ `island-cli plugin install ./path` |
| **P5** | **M1 index** bootstrap + search/info | ✅ `registry/` · bounded, hash-verified remote IPC tarballs with consent |
| **P6** | Wasm guest + default safe path for index | ⏳ not started |
| **P7** | Optional `surface.embed` + platform IOSurface/DXGI | ⏳ not started |

**Shipped layout:** `crates/island-plugin-{api,sdk,host}` · `crates/island-core` · `crates/island-cli` · `plugins/spotify` · `registry/`.

---

## 11. Example: minimal IPC plugin (conceptual)

```rust
// plugin main
fn main() {
    let mut host = island_plugin_sdk::connect().unwrap();
    host.present(Present { priority: 40, ttl_ms: Some(10_000), .. });
    let mut t = 0.0f32;
    loop {
        match host.poll() {
            Event::SurfaceReady { w, h, .. } => { /* alloc buffer */ }
            Event::ExpandState { expand, .. } if expand > 0.2 => {
                t += 0.05;
                let pixels = render_frame(w, h, t); // full custom view
                host.submit_frame(Frame { pixels, .. });
            }
            Event::Input(i) => { /* hit test */ }
            Event::Shutdown => break,
            _ => {}
        }
    }
}
```

Users never open a PR against `Sullivansome/island` for this—only against the **index** if they want discovery.

---

## 12. Risks & mitigations

| Risk | Mitigation |
|------|------------|
| Design chaos (ugly plugins) | Host frame + max size; market screenshots; optional “design lint” later |
| GPU/CPU abuse | FPS cap, frame size cap, process priority, user disable |
| Malicious IPC binary | Permissions, hash, process isolation, prefer Wasm on index |
| Toolkit fragmentation | Surface protocol only—no GPUI types in API |
| Protocol churn | `api_version`; compatibility window ≥ 1 major host release |
| Notch/embed edge cases | Prefer framebuffer over window embed in v1 |

---

## 13. Decisions log

| Decision | Choice | Notes |
|----------|--------|-------|
| Architecture | **E hybrid** | IPC + Wasm + builtin, one API |
| Presentation | **3 full custom view** | Framebuffer-first surface protocol |
| Marketplace | **M1 curated index** | GitHub JSON index + PR review |
| Default untrusted runtime | Wasm (from P6) | IPC for advanced/native |
| In-process Rust dylib plugins | **No** (public) | ABI/crash risk; not required for E |
| WebView-as-default | **No** | Optional backend later |

---

## 14. Open questions (resolve during P1–P2)

1. Frame format v1: only CPU BGRA shared memory, or jump to IOSurface on macOS immediately?  
2. Should host draw a mandatory 1px hairline + shadow outside plugin pixels always? (recommend **yes**)  
3. Multi-plugin visible at once (split island) vs single winner? (recommend **single winner** v1)  
4. Index in monorepo `registry/` vs `Sullivansome/island-plugins-index` from day one?

---

## 15. Agent checklist

When implementing plugins:

- [ ] No plugin types leak AppKit/GPUI/WinUI into `island-plugin-api`  
- [ ] Built-in modes use the same queue as external plugins  
- [ ] Custom view goes through `ViewSurface` / `Frame`, not ad-hoc `div` injection from dylibs  
- [ ] Permissions fail closed  
- [ ] Index install verifies `sha256`  
- [ ] Docs updated for authors (`docs/plugin-authoring.md` when P1 lands)  
- [ ] AGENTS.md multi-GUI rule still holds: shells composite surfaces, they don’t each invent a plugin ABI  

---

## Related

- [AGENTS.md](../AGENTS.md) — multi-GUI and core boundaries  
- Current overlay chrome: `src/island.rs`

## Shared plugin management

Market and the CLI now share catalog resolution, verified review/apply, installed state and runtime reporting in `island-plugin-host`. The versioned management schema lives in `island-core::management`. See [plugin-market.md](plugin-market.md) for the implemented Discover/Installed flow, immediate CLI control, offline locking, data retention, catalog caching and release artifact pipeline. Native IPC grants remain host-operation capabilities, not an OS sandbox.

### Compact activities and attention peeks

API 2 `Present` accepts an optional `mode`: `expanded` (default for existing plugins), `compact`, or `peek`. Queue priority, sticky lifetime, and TTL are independent of expansion. Core owns the four-second peek and 350 ms pointer dwell policies across shells. Explicit user opening persists until collapse; compact updates cannot undo manual collapse. `PreferredSize` can grow or shrink the content without presenting again.

`OpenUrl` still requires `shell:open-url`. Alongside HTTP(S), the host accepts exactly `codex://threads/<UUID>` for opening a local Codex session; other app schemes, route shapes, queries, and fragments are rejected.

URL rate limits (one attempt per five seconds), invalid destinations, OS launch failures, and timeouts are action errors: they do not disconnect the plugin or dismiss its surface. API 2 callers may supply `request_id` to receive a matching `OpenUrlResult`; `error: null` means the OS launcher accepted the request, not that the destination window was verified. Legacy callers receive no new response variant. Callers should suppress duplicate requests and tolerate a missing reply from older hosts. Missing capabilities still fail closed.

### Hover activities and collapsed icons

`Present.mode = "hover"` expands only during pointer dwell and collapses on leave; explicit clicks do not pin this mode. Queue priority and sticky lifetime remain independent. `SetCollapsedMeta.icon` optionally carries a BGRA `SurfaceFrame`, at most 96 × 96 pixels with exact byte length; `pixels` is base64 in JSON to stay within the 64 KiB control-message limit. Hosts validate it before accepting metadata and fall back to leading text when absent. Existing message defaults remain compatible.
