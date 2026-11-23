# AGENTS.md — Nocturne (Lofi Rust Client)

Instructions and architectural invariants for AI coding agents and contributors working on **Nocturne**.

---

## Project Summary

**Nocturne** is a cross-platform, ultra-low-resource, local-first lofi desktop application built with **Tauri v2 + Rust** and a **zero-framework Vanilla TypeScript / HTML5 / CSS3** frontend, inspired by the layout and atmospheric feel of the **LofiCafe** website.

### Core Invariants (Do Not Break)
1. **No Heavy JS Frameworks:** Keep the frontend 100% Vanilla TypeScript, HTML5, and CSS3. Do not introduce React, Vue, Svelte, Angular, or bulky CSS frameworks.
2. **Ultra-Low Resource Footprint:** Maintain adaptive Canvas FPS caps (`15 / 30 / 60 FPS`), DPR capping (`<= 1.5`), `document.visibilitychange` pausing for Canvas and `<video>`, zero per-frame allocations in hot render loops, and chunked streaming (`lofi://` with `206 Partial Content`) instead of loading media into RAM.
3. **Local-First & Private:** All configs, scenes, listening history, focus checklists, music libraries, visual assets, and plugins stay strictly on disk inside Tauri's `app_data_dir` or explicit user-selected directories. Never add telemetry or mandatory cloud dependencies.
4. **Unified Plugin Trait Architecture:** Native Rust plugins and dynamic WebAssembly (`.wasm` / `.wat`) modules must both implement the unified `LofiPlugin` trait (`src-tauri/src/plugins/mod.rs`). Never bypass `LofiPlugin` with Wasm-only special cases in command handlers.
5. **Media Protocol Allowlist Security:** The custom `lofi://` URI scheme handler (`src-tauri/src/media_protocol.rs`) must always validate requested paths via `LocalStorage::is_path_allowed` before opening files.

---

## Repository Map

```text
├── IDEA.md                             # Original product vision & architecture spec
├── AGENTS.md                           # Agent & contributor guide (this file)
├── README.md                           # User-facing documentation
├── index.html                          # HTML5 stage (<video>, <img>, <canvas>) + HUD containers
├── package.json                        # Vite + TypeScript + Tauri API/CLI
├── tsconfig.json                       # Strict TypeScript config (noEmit)
├── vite.config.ts                      # Vite config (port 1420 for Tauri devUrl)
├── plugins/
│   ├── README.md                       # WebAssembly Plugin ABI v1 specification
│   └── rainy-lo-fi-warmth.wat          # Example community WebAssembly Text (.wat) plugin
├── src/
│   ├── main.ts                         # LofiApp bootstrap, Tauri bridge, shortcuts, drag-and-drop, timers
│   ├── types/index.ts                  # TypeScript IPC contracts (mirror src-tauri/src/models.rs)
│   ├── styles/main.css                 # Frosted-glass bottom dock, popovers, cinema mode, responsive styles
│   ├── services/
│   │   ├── backend.ts                  # Tauri invoke() wrapper + localStorage browser-preview fallback
│   │   ├── audioEngine.ts              # Web Audio API synth stations, local/stream player, 5-band EQ, 8-layer ambience
│   │   └── visualEngine.ts             # HTML5 <video>/<img>/<canvas> renderer, 4 procedural scenes, FX overlays
│   └── ui/
│       ├── icons.ts                    # Inline 1.5px stroke SVG icons
│       ├── dock.ts                     # Bottom-overlay control bar & 10-module button suite
│       └── popovers.ts                 # Floating popover views for all 10 modules
└── src-tauri/
    ├── Cargo.toml                      # Tauri v2, tauri-plugin-dialog, wasmi, wat, walkdir, mime_guess
    ├── tauri.conf.json                 # Window config, build hooks, bundle icons
    ├── capabilities/default.json       # Tauri v2 window & dialog permissions
    └── src/
        ├── main.rs                     # Binary entrypoint (windows_subsystem = "windows")
        ├── lib.rs                      # Tauri builder, system tray, lofi:// protocol, #[tauri::command] handlers
        ├── models.rs                   # Serde camelCase structs shared with frontend
        ├── storage.rs                  # Local directory management, JSON persistence, WalkDir scanner, allowlist
        ├── media_protocol.rs           # Custom lofi:// streaming protocol with HTTP Range (206 Partial Content)
        └── plugins/
            ├── mod.rs                  # LofiPlugin trait, PluginEvent, PluginManager registry
            ├── native.rs               # CoreAtmospherePlugin (native Rust implementation)
            └── wasm_runtime.rs         # wasmi-based WasmPlugin loader/executor + starter .wasm generators
```

---

## Contracts & Conventions

### Rust ↔ TypeScript IPC
- All Rust IPC structs in `src-tauri/src/models.rs` use `#[serde(rename_all = "camelCase")]`.
- Whenever you add or modify a field or `#[tauri::command]` in `src-tauri/src/lib.rs`, update:
  1. `src/types/index.ts`
  2. `src/services/backend.ts` (including the non-Tauri browser fallback so `pnpm dev` continues working standalone)

### WebAssembly Plugin ABI (v1)
Plugins loaded by `WasmPlugin` (`src-tauri/src/plugins/wasm_runtime.rs`) must export:
- `memory` (linear memory, >= 1 page)
- `lofi_abi_version() -> i32` (must return `1`)
- `lofi_metadata_ptr() -> i32` & `lofi_metadata_len() -> i32` (UTF-8 JSON manifest)
- `lofi_result_ptr() -> i32` (offset for 5 little-endian `f32` EQ bands)
- `lofi_transform_eq(f32, f32, f32, f32, f32, f32) -> f32` (5 band gains + intensity `0..1`, returns suggested tape cutoff Hz)
- Optional: `lofi_on_event(i32, i32) -> i32`

When updating `.wat` files manually, keep `lofi_metadata_len` equal to the exact UTF-8 byte length of the embedded JSON string and verify via `cargo test`.

### Frontend UI Guidelines
- Keep the bottom dock compact and aligned with the floating frosted-glass overlay aesthetic.
- Avoid blocking modals; every feature lives cleanly inside one of the 10 modular popovers (`eq`, `music`, `visuals`, `sounds`, `scenes`, `history`, `share`, `keys`, `focus`, `sleep`).
- Escape all dynamic user strings (file names, scene names, plugin metadata) before interpolating into HTML (`esc()` in `src/ui/popovers.ts`).

---

## Development & Verification Commands

```bash
# Install dependencies
pnpm install

# TypeScript strict check + production frontend bundle
pnpm run build

# Run Rust unit tests (Wasm runtime, plugin manager, WAT compilation, HTTP Range, storage allowlist)
cargo test --manifest-path src-tauri/Cargo.toml

# Build Tauri Rust binary (debug)
cargo build --manifest-path src-tauri/Cargo.toml

# Run full desktop app in dev mode
pnpm tauri dev

# Build optimized release bundle
pnpm tauri build
```

Before committing any change, verify:
1. `pnpm run build` passes with 0 TypeScript errors.
2. `cargo test --manifest-path src-tauri/Cargo.toml` passes all tests with 0 warnings.
3. No runtime console errors when testing the UI.
