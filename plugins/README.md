# Lofi Desktop WebAssembly Plugin ABI (v1)

Lofi Desktop dynamically loads `.wasm` and `.wat` plugins through the unified Rust `LofiPlugin` trait using the lightweight `wasmi` WebAssembly runtime.

## Required Exports

| Export | Signature | Description |
| --- | --- | --- |
| `memory` | Linear memory (min 1 page) | Shared memory used for metadata JSON and DSP result output |
| `lofi_abi_version` | `() -> i32` | Must return `1` |
| `lofi_metadata_ptr` | `() -> i32` | Byte offset of UTF-8 JSON manifest in linear memory |
| `lofi_metadata_len` | `() -> i32` | Byte length of UTF-8 JSON manifest |
| `lofi_result_ptr` | `() -> i32` | Byte offset where 5 little-endian `f32` EQ outputs are written |
| `lofi_transform_eq` | `(f32, f32, f32, f32, f32, f32) -> f32` | Input 5 EQ bands (dB) + `intensity` (`0..1`), writes 5 output `f32` bands at `lofi_result_ptr`, returns suggested tape low-pass cutoff Hz |
| `lofi_on_event` *(optional)* | `(i32, i32) -> i32` | Lifecycle hook (`1` = track changed, `2` = scene changed, `3` = focus complete) |

## Installing Custom Plugins

Drop any `.wasm` or `.wat` file into your app's `plugins/` folder (shown in the **Scenes → Wasm Plugins** panel) or click **Install .wasm** directly in the UI.
