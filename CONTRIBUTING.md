# Contributing to Kissa

Thank you for considering contributing to **Kissa**! Whether you're fixing a bug, adding a new native scene, improving audio DSP, building a WebAssembly dock plugin, or refining documentation, your help is welcome.

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

---

## Project Philosophy (Core Invariants)

Before making architectural changes, keep Kissa's core principles in mind:

1. **100% Pure Rust — Zero WebView:** Kissa exists specifically to stay tiny and fast without Electron, Chromium, WebView2, or Node.js.
2. **Ultra-Low Resource Footprint:** Avoid per-frame heap allocations in `SceneRenderer::paint` (`src/ui/scenes.rs`) and inside the real-time `cpal` audio callback (`src/audio/mod.rs`). Keep media streaming bounded in RAM.
3. **Local-First & Private:** All configs, scenes, history, focus tasks, media, and plugins live strictly on disk (`dirs::data_dir()/kissa`). Never add telemetry or mandatory cloud dependencies.
4. **Unified Dock & Plugin Trait:** Every core dock button (`EQ` through `Sleep`) and every external `.wasm` / `.wat` module implements `LofiPlugin` (`src/plugins/mod.rs`). Never hardcode dock buttons in UI code.

---

## Development Setup

### Prerequisites
- **[Rust](https://rustup.rs/)** stable toolchain (`1.80+` recommended)
- **Optional:** **[FFmpeg](https://ffmpeg.org/)** (`ffmpeg` + `ffprobe` in `PATH`) for testing local video wallpapers
- **Linux only:** System libraries required by `eframe`, `cpal`, and `rfd`:
  ```bash
  sudo apt-get install -y libasound2-dev libgtk-3-dev libxkbcommon-dev libwayland-dev libxcb-shape0-dev libxcb-xfixes0-dev
  ```

### Running & Testing Locally

```bash
# Clone your fork
git clone https://github.com/<your-username>/kissa-lofi.git
cd kissa-lofi

# Run unit tests (Wasm runtime, plugin registry, WAT compilation, storage scanner)
cargo test

# Run Kissa in debug mode
cargo run

# Run Kissa with release optimizations (recommended for audio/video testing)
cargo run --release

# Capture a lossless native screenshot of the window (useful for UI PRs)
cargo run --release -- --play --screenshot assets/preview.png
```

---

## Codebase Map

| Path | Responsibility |
| :--- | :--- |
| `src/main.rs` | `eframe` entrypoint, window icon, CLI flags (`--video`, `--audio`, `--play`, `--screenshot`) |
| `src/models.rs` | Serde state models, built-in stations, ambient channels, plugin UI widget contracts |
| `src/storage.rs` | Local app-data directory layout, JSON persistence, `WalkDir` media scanner, curated scenes |
| `src/video.rs` | Photo loader (`image`) & single-buffer streaming `ffmpeg` `rgb24` local video decoder |
| `src/audio/` | Real-time `cpal` mixer, 5-band biquad EQ (`dsp.rs`), procedural synth (`synth.rs`), 8-layer ambience (`ambience.rs`), streaming `symphonia` decoder (`decoder.rs`) |
| `src/plugins/` | Unified `LofiPlugin` trait (`mod.rs`), 10 built-in dock plugins (`builtin.rs`), `wasmi` `.wasm`/`.wat` sandbox (`wasm_runtime.rs`) |
| `src/ui/` | `KissaApp` loop (`mod.rs`), 4 GPU scenes (`scenes.rs`), frosted bottom dock (`dock.rs`), popovers & Plugin Manager (`popovers.rs`) |
| `plugins/` | WebAssembly Plugin ABI v1 specification (`README.md`) & example `.wat` plugin |

---

## Building a WebAssembly Plugin

You don't even have to fork Kissa to extend it! External `.wasm` and `.wat` plugins can register new buttons directly onto the bottom dock with interactive sliders, metrics, and EQ/DSP transforms.

- Read the full ABI v1 specification in [`plugins/README.md`](plugins/README.md).
- Study the working example in [`plugins/rainy-lo-fi-warmth.wat`](plugins/rainy-lo-fi-warmth.wat).
- Drop your `.wasm` or `.wat` file straight onto the Kissa window to test it live.
- Share your plugin using the **Plugin Showcase** issue template!

---

## Submitting a Pull Request

1. Fork the repository and create a feature branch from `main`:
   ```bash
   git checkout -b feat/my-cool-feature
   ```
2. Keep commits focused and descriptive (Conventional Commits like `feat:`, `fix:`, `perf:`, `docs:` are encouraged).
3. Run the full verification checklist before opening your PR:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets
   cargo test
   cargo build --release
   ```
4. If your change affects visuals or UI layout, include a before/after screenshot (you can generate one cleanly with `--screenshot`).
5. Open a Pull Request against `main` and fill out the PR template.

---

## Good Areas to Contribute

- **Audio & Library:** Embedded ID3/Vorbis cover art & metadata extraction for local tracks, M3U playlists, internet radio streams.
- **Visuals:** New native GPU `SceneRenderer` scenes, custom Wasm visual shader hooks, optional CPU software renderer mode for ultra-low RAM.
- **Platform Polish:** System tray integration across Windows/macOS/Linux, media key bindings (MPRIS / SMTC / MediaPlayer), pre-built GitHub Release bundles.
- **Plugins:** Community `.wasm`/`.wat` dock modules (ambient generators, vintage tape curves, Pomodoro presets).
