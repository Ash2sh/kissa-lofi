# Nocturne

> **v0.1.0-alpha.1** · Cross-platform, ultra-low-resource, local-first lofi desktop application built with **Tauri v2**, **Rust**, **Vanilla TypeScript**, and a dynamic **WebAssembly (Wasm)** plugin runtime.

---

## Inspiration & Reference

Nocturne's clean bottom-overlay frosted-glass control dock, modular quick-access suite, and cozy midnight-window atmosphere were designed using the **[LofiCafe](https://lofi.cafe/)** website as a visual and functional reference — reimagined as a private, local-first native desktop client with custom local wallpaper/music libraries, zero frontend frameworks, and extensible WebAssembly plugins.

---

## Key Features

- **Atmospheric Visual Engine (`<video>` + Canvas API)**
  - Includes 4 built-in animated Canvas scenes (*Rainy Window Cat*, *Neon Rain Skyline*, *Midnight Starlight*, *Warm Study Dusk*).
  - Load custom local **photos** (`.jpg`, `.png`, `.webp`, `.gif`, `.avif`) and looping **videos** (`.mp4`, `.webm`, `.mov`) via folder scanning, file picker, or drag-and-drop.
  - Lightweight Canvas atmospheric FX layer (rain streaks, vignette, film grain, dimmer, warm tint, blur) with configurable FPS power cap (`15 / 30 / 60 FPS`) and automatic pause when hidden/minimized.
- **Continuous Lofi Audio & Ambient Mixer (Web Audio API)**
  - 4 built-in procedural lofi stations (*Chilling*, *Focusing*, *Drifting*, *Unwinding*) ready out of the box even before adding local tracks.
  - Full local music library playback (`.mp3`, `.flac`, `.wav`, `.ogg`, `.m4a`, `.aac`, `.opus`) streamed via a custom `lofi://` Rust protocol with HTTP `206 Partial Content` range requests, plus custom audio stream URLs.
  - Real 5-band parametric EQ + Lofi Tape low-pass filter, custom presets, and an 8-layer procedural + custom-file ambient soundscape mixer (*Rain*, *Vinyl*, *Fireplace*, *Thunder*, *Night Wind*, *Cafe*, *Forest*, *Ocean*).
- **Dynamic WebAssembly Plugin Architecture**
  - Unified Rust `LofiPlugin` trait linking native Rust plugins and dynamically loaded `.wasm` / `.wat` modules executed inside `wasmi`.
  - Ships with starter Wasm DSP modules (`midnight-tape.wasm`, `deep-focus-boost.wasm`) and an example plugin at [`plugins/rainy-lo-fi-warmth.wat`](plugins/rainy-lo-fi-warmth.wat). See [`plugins/README.md`](plugins/README.md) for the ABI v1 spec.
- **Extensible 10-Module Bottom Overlay Suite**
  - `1` **EQ** · `2` **Music** · `3` **Visuals** · `4` **Sounds** · `5` **Scenes (+ Wasm Plugins)** · `6` **History** · `7` **Share** · `8` **Keys** · `9` **Focus** · `0` **Sleep**
- **Ultra-Low Resource Footprint**
  - Pure Vanilla TypeScript, HTML5, and CSS3 (~37 KB gzipped frontend bundle, zero UI framework overhead) paired with a lean Rust/Tauri native shell and system tray integration.

---

## Tech Stack

| Layer | Technology |
| --- | --- |
| Desktop Shell & Backend | Tauri v2, Rust, `tauri-plugin-dialog`, System Tray |
| WebAssembly Plugin Runtime | `wasmi` + `wat` via unified Rust `LofiPlugin` trait |
| Frontend UI | Vanilla TypeScript, HTML5, CSS3 (Vite bundler) |
| Visual Rendering | HTML5 `<video>`, `<img>`, and 2D `<canvas>` |
| Audio DSP & Synthesis | Native Web Audio API (`BiquadFilterNode`, procedural synths & ambient generators) |

---

## Keyboard Shortcuts

| Key | Action |
| --- | --- |
| `Space` | Play / Pause |
| `M` | Mute / Unmute |
| `↑` / `↓` | Volume Up / Down |
| `N` / `P` | Next / Previous Track or Station |
| `1` – `9`, `0` | Toggle Dock Modules (`EQ` through `Sleep`) |
| `H` | Toggle Cinema Mode (Hide / Show HUD) |
| `F` | Toggle Fullscreen |
| `Esc` | Close Active Panel |

---

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (v20+) & [pnpm](https://pnpm.io/)
- Platform WebView dependencies for [Tauri v2](https://v2.tauri.app/start/prerequisites/) (WebView2 on Windows)

### Run & Build

```bash
# Install frontend dependencies
pnpm install

# Launch Nocturne desktop app in development mode
pnpm tauri dev

# Run TypeScript check and build frontend bundle
pnpm run build

# Run Rust backend & WebAssembly plugin tests
cargo test --manifest-path src-tauri/Cargo.toml

# Build optimized release desktop binary
pnpm tauri build
```

---

## Alpha Status (`v0.1.0-alpha.1`)

This is the initial alpha release of **Nocturne**. Core local media streaming, Canvas/`<video>` backgrounds, Web Audio synthesis/EQ/ambience, all 10 dock modules, and the dynamic `wasmi` WebAssembly plugin runtime are working and tested; additional visual shader scenes, audio metadata tag parsing, and expanded Wasm plugin hooks are planned for upcoming releases.
