# Kissa WebAssembly Dock Plugin ABI (v1)

In **Kissa**, every button on the bottom dock—including all 10 built-in modules (`EQ`, `Music`, `Visuals`, `Sounds`, `Scenes`, `History`, `Share`, `Keys`, `Focus`, `Sleep`) and external `.wasm` / `.wat` extensions—is registered through the unified Rust `LofiPlugin` trait.

Third-party plugins run inside the lightweight `wasmi` WebAssembly runtime and can add their own button directly onto Kissa's bottom dock, render a safe declarative glass popover panel, contribute EQ presets, and run real-time Wasm DSP transforms.

## Required Exports

| Export | Signature | Description |
| --- | --- | --- |
| `memory` | Linear memory (min 1 page) | Shared memory used for manifest JSON and DSP result output |
| `lofi_abi_version` | `() -> i32` | Must return `1` |
| `lofi_metadata_ptr` | `() -> i32` | Byte offset of UTF-8 JSON manifest in linear memory |
| `lofi_metadata_len` | `() -> i32` | Byte length of UTF-8 JSON manifest |
| `lofi_result_ptr` | `() -> i32` | Byte offset where 5 little-endian `f32` EQ outputs are written |
| `lofi_transform_eq` | `(f32, f32, f32, f32, f32, f32) -> f32` | Input 5 EQ bands (dB) + `intensity` (`0..1`), writes 5 output `f32` bands at `lofi_result_ptr`, returns suggested tape low-pass cutoff Hz |
| `lofi_on_event` *(optional)* | `(i32, i32) -> i32` | Lifecycle hook (`1` = track changed, `2` = scene changed, `3` = focus complete) |

## Manifest JSON (Adding a Custom Dock Button)

Include `"dockModule"` and `"panelActions"` in the manifest JSON exposed via `lofi_metadata_ptr`/`lofi_metadata_len` to register your own bottom-dock button and interactive popover:

```json
{
  "id": "wasm-my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "author": "You",
  "description": "Adds a custom dock button and Wasm audio transform.",
  "capabilities": ["dock-module", "eq-dsp", "eq-presets"],
  "dockModule": {
    "id": "wasm-my-plugin",
    "label": "Warmth",
    "icon": "flame",
    "order": 120
  },
  "panelActions": [
    { "id": "apply", "label": "Apply Warmth", "style": "accent", "effect": "wasm-eq" },
    { "id": "rain", "label": "Add Rain (50%)", "style": "default", "effect": "ambience", "target": "rain", "amount": 0.5 }
  ]
}
```

Available built-in icon keys for `dockModule.icon`: `tape`, `flame`, `coffee`, `puzzle`, `eq`, `music`, `visuals`, `sounds`, `scenes`, `history`, `share`, `keys`, `focus`, `sleep`.

## Installing Custom Plugins

Open the **Plugin Manager** (`G` or top-right **Plugins** button) and click **Install .wasm / .wat**, drop a `.wasm`/`.wat` file directly onto the Kissa window, or copy it into your local `plugins/` folder and click **Reload**. See [`rainy-lo-fi-warmth.wat`](rainy-lo-fi-warmth.wat) for a complete working example.
