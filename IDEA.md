# Project Overview: Lofi Desktop Application

Cross-platform, low-resource lofi desktop app offering atmospheric backgrounds, audio playback, and a modular button suite.

---

## Core Architecture & Tech Stack

* **Core Framework:** Tauri + Rust (Backend)
* **Frontend UI:** Vanilla TypeScript, HTML5, CSS3 (No heavy JavaScript frameworks)
* **Rendering Engine:** HTML5 `<video>` and Canvas API for optimized static/dynamic media backgrounds
* **Plugin Architecture:** Dynamic WebAssembly (Wasm) runtime linked via unified Rust trait interfaces

---
## Key Value Propositions

* **Atmospheric Personalization:** Users can set custom local photos or videos as living wallpaper backgrounds, paired with continuous lofi audio streaming.
* **Local-First File Management:** Complete privacy and local control over music libraries and visual assets using local or custom user directories.
* **Extensible Button Suite:** A clean, bottom-overlay control panel containing extensible modules (EQ, Music, Visuals, Sounds, Scenes, History, Share, Keys, Focus, Sleep) designed for minimal UI footprint and quick access.
* **Ultra-Low Resource Footprint:** Designed specifically to run continuously in the background without draining system resources or distracting from primary tasks.
