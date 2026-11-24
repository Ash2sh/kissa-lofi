# AGENTS.md — Kissa (`kissa-lofi`)

Instructions and architectural invariants for AI coding agents and contributors working on **Kissa**.

---

## Project Summary

**Kissa** is a cross-platform, ultra-low-resource, local-first lofi desktop application built in **100% Pure Rust** — zero WebView, zero Chromium, zero Node.js — using **`eframe` / `egui` (`glow` OpenGL)** for native GPU UI, **`cpal` + `symphonia`** for real-time audio & streaming playback, and **`wasmi`** for dynamic WebAssembly plugins, inspired by the layout and atmospheric feel of the **LofiCafe** website.

### Core Invariants (Do Not Break)
1. **Zero WebView / 100% Native Rust:** Never reintroduce Electron, WebView2, Tauri WebView, Node.js, or browser runtimes. Keep Kissa a single native Rust binary with minimal RAM footprint.
2. **Ultra-Low Resource Footprint:** Maintain adaptive render FPS caps (`15 / 30 / 60 FPS`), minimized-window throttling (`pause_when_hidden`), bounded streaming audio ring buffers (`<= 1 MB` RAM via `symphonia`), single reused RGBA frame buffer for streaming local video wallpapers (`<= 960×540` capped via `ffmpeg` pipe updating one GPU texture in-place), and zero per-frame heavy allocations in audio/render loops.
3. **Local-First & Private:** All configs, scenes, listening history, focus checklists, music libraries, visual assets, and plugins stay strictly on disk inside the OS app data directory (`dirs::data_dir()/kissa`) or explicit user-selected directories. Never add telemetry or mandatory cloud dependencies.
4. **Unified Dock & Plugin Trait Architecture:** All 10 core dock buttons (`EQ`, `Music`, `Visuals`, `Sounds`, `Scenes`, `History`, `Share`, `Keys`, `Focus`, `Sleep`) and all external WebAssembly (`.wasm` / `.wat`) modules implement the unified `LofiPlugin` trait (`src/plugins/mod.rs`). Never hardcode the dock module list in the UI or mix plugin management into `Scenes`; the bottom dock is built dynamically from `PluginRegistrySnapshot.dock_modules`, and plugins are managed in the dedicated **Plugin Manager** (`G`).
5. **Safe Media Handling:** Keep local audio/video decoding bounded in memory, spawn background subprocesses (`ffmpeg`/`ffprobe`) with `CREATE_NO_WINDOW` on Windows, and cleanly terminate child processes on exit or background switch.

---

## Repository Map

```text
├── IDEA.md                     # Original product vision & architecture spec
├── AGENTS.md                   # Agent & contributor guide (this file)
├── README.md                   # User-facing documentation
├── Cargo.toml                  # Single pure-Rust crate (eframe/egui glow, cpal, symphonia, wasmi, image, rfd)
├── assets/
│   ├── kissa-icon.png          # Embedded window & README app icon
│   └── preview.png             # App preview screenshot
├── plugins/
│   ├── README.md               # WebAssembly Plugin ABI v1 specification
│   └── rainy-lo-fi-warmth.wat  # Example community WebAssembly Text (.wat) plugin
└── src/
    ├── main.rs                 # Native eframe/egui entrypoint, embedded window icon, CLI flags
    ├── models.rs               # Serde camelCase state models, stations, ambience defs, plugin UI contracts
    ├── storage.rs              # Local directory management, JSON persistence, WalkDir scanner, curated scenes
    ├── video.rs                # Photo decoder (image) & single-buffer streaming ffmpeg local video renderer
    ├── audio/
    │   ├── mod.rs              # Real-time cpal audio output engine (synth + local + EQ + ambience + sleep fade)
    │   ├── dsp.rs              # Stereo 5-band parametric biquad EQ + warm tape low-pass + soft limiter
    │   ├── synth.rs            # Pure-Rust procedural lofi synth (4 stations + focus chime)
    │   ├── ambience.rs         # 8-layer procedural ambient soundscape mixer
    │   └── decoder.rs          # Streaming symphonia local audio decoder with bounded ring buffer & seek
    ├── plugins/
    │   ├── mod.rs              # LofiPlugin trait, PluginEvent, PluginManager registry
    │   ├── builtin.rs          # All 10 built-in dock module plugins (EQ..Sleep) implementing LofiPlugin
    │   └── wasm_runtime.rs     # wasmi-based WasmPlugin loader, dock UI builder & starter .wasm generator
    └── ui/
        ├── mod.rs              # KissaApp (eframe::App), keyboard shortcuts, drag-and-drop, timers, HUD
        ├── scenes.rs           # Native GPU egui::Painter scenes (4 animated scenes) + media cover & FX overlays
        ├── dock.rs             # Floating dark frosted-glass bottom dock matching LofiCafe aesthetic
        └── popovers.rs         # Floating popovers for all 10 modules, Plugin Manager (G) & Wasm plugin panels
```

---

## Contracts & Conventions

### WebAssembly Plugin ABI (v1)
Plugins loaded by `WasmPlugin` (`src/plugins/wasm_runtime.rs`) must export:
- `memory` (linear memory, >= 1 page)
- `lofi_abi_version() -> i32` (must return `1`)
- `lofi_metadata_ptr() -> i32` & `lofi_metadata_len() -> i32` (UTF-8 JSON manifest)
- `lofi_result_ptr() -> i32` (offset for 5 little-endian `f32` EQ bands)
- `lofi_transform_eq(f32, f32, f32, f32, f32, f32) -> f32` (5 band gains + intensity `0..1`, returns suggested tape cutoff Hz)
- Optional: `lofi_on_event(i32, i32) -> i32`

When updating `.wat` files manually, keep `lofi_metadata_len` equal to the exact UTF-8 byte length of the embedded JSON string and verify via `cargo test`.

### Native UI Guidelines
- Keep the bottom dock compact and aligned with the floating dark frosted-glass overlay aesthetic.
- Avoid blocking dialogs inside the render loop except explicit user-triggered `rfd` file/folder pickers; every feature lives cleanly inside a floating popover above the dock (`eq`, `music`, `visuals`, `sounds`, `scenes`, `history`, `share`, `keys`, `focus`, `sleep`, `Plugin Manager`, or dynamic Wasm plugin panels).
- Never allocate large per-frame buffers in `SceneRenderer::paint` or the `cpal` audio callback.

---

## Development & Verification Commands

```bash
# Run all unit tests (Wasm runtime, plugin manager, WAT compilation, storage scanner)
cargo test

# Run Kissa in debug mode
cargo run

# Build optimized release binary (stripped, LTO)
cargo build --release

# Launch release binary directly
./target/release/kissa
```

Before committing any change, verify:
1. `cargo test` passes all tests with 0 warnings.
2. `cargo build --release` succeeds cleanly.
3. No runtime panics or memory growth during video + audio playback.
