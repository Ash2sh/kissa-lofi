use crate::models::{
    EqPreset, PluginEqTransformResult, PluginKind, PluginMetadata, PluginSceneContribution,
};
use crate::plugins::{LofiPlugin, PluginError, PluginEvent};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use wasmi::{Engine, Instance, Linker, Module, Store};

const EXPECTED_ABI_VERSION: i32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmManifestJson {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    eq_presets: Vec<WasmEqPresetJson>,
    #[serde(default)]
    scenes: Vec<WasmSceneJson>,
    #[serde(default)]
    transform_note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmEqPresetJson {
    id: String,
    name: String,
    bands: [f32; 5],
    tape_cutoff_hz: f32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmSceneJson {
    id: String,
    name: String,
    mood: String,
    description: String,
    visual_id: String,
    eq_bands: [f32; 5],
    tape_cutoff_hz: f32,
    #[serde(default)]
    ambience: HashMap<String, f32>,
    accent_color: String,
}

struct WasmState {
    store: Store<()>,
    instance: Instance,
}

/// Dynamically loaded WebAssembly plugin executed inside `wasmi` and bridged
/// seamlessly through the unified `LofiPlugin` Rust trait.
pub struct WasmPlugin {
    metadata: PluginMetadata,
    eq_presets: Vec<EqPreset>,
    scenes: Vec<PluginSceneContribution>,
    transform_note: String,
    runtime: Mutex<WasmState>,
}

impl WasmPlugin {
    pub fn from_file(path: &Path) -> Result<Self, PluginError> {
        let raw = fs::read(path).map_err(|e| PluginError::Io(e.to_string()))?;
        let is_wat = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.eq_ignore_ascii_case("wat"))
            .unwrap_or(false);

        let wasm_bytes = if is_wat {
            wat::parse_bytes(&raw)
                .map_err(|e| PluginError::Wasm(format!("Failed parsing WAT: {e}")))?
                .into_owned()
        } else {
            raw
        };

        Self::from_bytes(&wasm_bytes, Some(path.to_string_lossy().to_string()))
    }

    pub fn from_bytes(wasm_bytes: &[u8], file_path: Option<String>) -> Result<Self, PluginError> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| PluginError::Wasm(format!("Invalid Wasm module: {e}")))?;
        let mut store = Store::new(&engine, ());
        let linker = <Linker<()>>::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginError::Wasm(format!("Instantiation failed: {e}")))?
            .start(&mut store)
            .map_err(|e| PluginError::Wasm(format!("Start failed: {e}")))?;

        // 1. Validate ABI version
        let abi_fn = instance
            .get_typed_func::<(), i32>(&store, "lofi_abi_version")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_abi_version': {e}")))?;
        let abi_ver = abi_fn
            .call(&mut store, ())
            .map_err(|e| PluginError::Wasm(format!("lofi_abi_version trap: {e}")))?;
        if abi_ver != EXPECTED_ABI_VERSION {
            return Err(PluginError::AbiMismatch {
                expected: EXPECTED_ABI_VERSION,
                found: abi_ver,
            });
        }

        // 2. Read plugin metadata JSON from Wasm linear memory
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| PluginError::Wasm("Missing export 'memory'".to_string()))?;
        let ptr_fn = instance
            .get_typed_func::<(), i32>(&store, "lofi_metadata_ptr")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_metadata_ptr': {e}")))?;
        let len_fn = instance
            .get_typed_func::<(), i32>(&store, "lofi_metadata_len")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_metadata_len': {e}")))?;

        let ptr = ptr_fn
            .call(&mut store, ())
            .map_err(|e| PluginError::Wasm(format!("lofi_metadata_ptr trap: {e}")))?
            as usize;
        let len = len_fn
            .call(&mut store, ())
            .map_err(|e| PluginError::Wasm(format!("lofi_metadata_len trap: {e}")))?
            as usize;

        let mut json_buf = vec![0u8; len];
        memory
            .read(&store, ptr, &mut json_buf)
            .map_err(|e| PluginError::Wasm(format!("Failed reading metadata from Wasm memory: {e}")))?;
        let manifest: WasmManifestJson = serde_json::from_slice(&json_buf)
            .map_err(|e| PluginError::InvalidMetadata(format!("Invalid metadata JSON: {e}")))?;

        let source_tag = format!("plugin:{}", manifest.id);
        let eq_presets = manifest
            .eq_presets
            .into_iter()
            .map(|p| EqPreset {
                id: p.id,
                name: p.name,
                bands: p.bands,
                tape_cutoff_hz: p.tape_cutoff_hz,
                source: source_tag.clone(),
            })
            .collect();

        let scenes = manifest
            .scenes
            .into_iter()
            .map(|s| PluginSceneContribution {
                id: s.id,
                name: s.name,
                mood: s.mood,
                description: s.description,
                visual_id: s.visual_id,
                eq_bands: s.eq_bands,
                tape_cutoff_hz: s.tape_cutoff_hz,
                ambience: s.ambience,
                accent_color: s.accent_color,
                plugin_id: manifest.id.clone(),
            })
            .collect();

        let metadata = PluginMetadata {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            author: manifest.author,
            description: manifest.description,
            kind: PluginKind::Wasm,
            file_path,
            capabilities: manifest.capabilities,
            enabled: true,
        };

        let transform_note = manifest
            .transform_note
            .unwrap_or_else(|| "Executed Wasm EQ DSP curve transform".to_string());

        Ok(Self {
            metadata,
            eq_presets,
            scenes,
            transform_note,
            runtime: Mutex::new(WasmState { store, instance }),
        })
    }
}

impl LofiPlugin for WasmPlugin {
    fn id(&self) -> &str {
        &self.metadata.id
    }

    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn eq_presets(&self) -> Vec<EqPreset> {
        self.eq_presets.clone()
    }

    fn scene_contributions(&self) -> Vec<PluginSceneContribution> {
        self.scenes.clone()
    }

    fn transform_eq(
        &self,
        bands: [f32; 5],
        intensity: f32,
    ) -> Result<PluginEqTransformResult, PluginError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| PluginError::Wasm("Plugin mutex poisoned".to_string()))?;
        let WasmState { store, instance } = &mut *guard;

        let transform_fn = instance
            .get_typed_func::<(f32, f32, f32, f32, f32, f32), f32>(&*store, "lofi_transform_eq")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_transform_eq': {e}")))?;

        let result_ptr_fn = instance
            .get_typed_func::<(), i32>(&*store, "lofi_result_ptr")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_result_ptr': {e}")))?;

        let cutoff_hz = transform_fn
            .call(
                &mut *store,
                (
                    bands[0],
                    bands[1],
                    bands[2],
                    bands[3],
                    bands[4],
                    intensity.clamp(0.0, 1.0),
                ),
            )
            .map_err(|e| PluginError::Wasm(format!("lofi_transform_eq trap: {e}")))?;

        let result_ptr = result_ptr_fn
            .call(&mut *store, ())
            .map_err(|e| PluginError::Wasm(format!("lofi_result_ptr trap: {e}")))?
            as usize;

        let memory = instance
            .get_memory(&*store, "memory")
            .ok_or_else(|| PluginError::Wasm("Missing export 'memory'".to_string()))?;

        let mut raw_bytes = [0u8; 20]; // 5 x f32 little-endian
        memory
            .read(&*store, result_ptr, &mut raw_bytes)
            .map_err(|e| PluginError::Wasm(format!("Failed reading EQ result: {e}")))?;

        let mut output_bands = [0.0f32; 5];
        for i in 0..5 {
            let val = f32::from_le_bytes([
                raw_bytes[i * 4],
                raw_bytes[i * 4 + 1],
                raw_bytes[i * 4 + 2],
                raw_bytes[i * 4 + 3],
            ]);
            output_bands[i] = (val.clamp(-12.0, 12.0) * 10.0).round() / 10.0;
        }

        Ok(PluginEqTransformResult {
            plugin_id: self.metadata.id.clone(),
            plugin_name: self.metadata.name.clone(),
            input_bands: bands,
            output_bands,
            suggested_tape_cutoff_hz: cutoff_hz.clamp(1600.0, 20000.0),
            note: self.transform_note.clone(),
        })
    }

    fn on_event(&self, event: &PluginEvent) -> Result<i32, PluginError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| PluginError::Wasm("Plugin mutex poisoned".to_string()))?;
        let WasmState { store, instance } = &mut *guard;

        let Ok(event_fn) = instance.get_typed_func::<(i32, i32), i32>(&*store, "lofi_on_event") else {
            return Ok(0);
        };

        let (kind, value) = match event {
            PluginEvent::TrackChanged { bpm_hint, mood_code } => (1, bpm_hint.saturating_add(*mood_code)),
            PluginEvent::SceneChanged { scene_index } => (2, *scene_index),
            PluginEvent::FocusSessionComplete { total_sessions } => (3, *total_sessions),
        };

        event_fn
            .call(&mut *store, (kind, value))
            .map_err(|e| PluginError::Wasm(format!("lofi_on_event trap: {e}")))
    }
}

/// Generates real compiled `.wasm` binary modules from WebAssembly Text Format
/// for first-run installation into `<app_data>/plugins/`.
pub fn starter_wasm_modules() -> Vec<(&'static str, Vec<u8>)> {
    let midnight_manifest = r##"{"id":"wasm-midnight-tape","name":"Midnight Tape Warmth","version":"1.0.0","author":"Lofi Wasm SDK","description":"WebAssembly DSP module that sculpts analog reel-to-reel warmth and mellow late-night roll-off.","capabilities":["eq-dsp","eq-presets","scene-presets","event-hooks"],"eqPresets":[{"id":"wasm-midnight-reel","name":"Midnight Reel (Wasm)","bands":[3.8,2.2,-1.0,-2.8,-5.2],"tapeCutoffHz":4600}],"scenes":[{"id":"wasm-midnight-tape-scene","name":"Analog Midnight Reel","mood":"LATE NIGHT","description":"Wasm-crafted reel-to-reel warmth paired with soft storm rain and vinyl crackle.","visualId":"rainy-window-cat","eqBands":[3.8,2.2,-1.0,-2.8,-5.2],"tapeCutoffHz":4600,"ambience":{"rain":0.52,"vinyl":0.32},"accentColor":"#38bdf8"}],"transformNote":"Sculpted analog tape curve inside WebAssembly sandbox"}"##;

    let focus_manifest = r##"{"id":"wasm-deep-focus","name":"Deep Focus Clarity","version":"1.0.0","author":"Lofi Wasm SDK","description":"WebAssembly module tuned for gentle vocal-band dip and soft air presence during long study sessions.","capabilities":["eq-dsp","eq-presets","scene-presets","event-hooks"],"eqPresets":[{"id":"wasm-study-clarity","name":"Study Clarity (Wasm)","bands":[1.4,0.8,-1.8,1.2,-1.5],"tapeCutoffHz":11200}],"scenes":[{"id":"wasm-deep-focus-scene","name":"Library Rain Focus","mood":"DEEP FOCUS","description":"Balanced Wasm acoustic profile for distraction-free coding and reading.","visualId":"neon-rain-skyline","eqBands":[1.4,0.8,-1.8,1.2,-1.5],"tapeCutoffHz":11200,"ambience":{"rain":0.35,"fireplace":0.20},"accentColor":"#34d399"}],"transformNote":"Applied distraction-free focus EQ curve via Wasm runtime"}"##;

    vec![
        (
            "midnight-tape.wasm",
            compile_eq_wasm_module(midnight_manifest, [3.8, 2.2, -1.0, -2.8, -5.2], 4600.0),
        ),
        (
            "deep-focus-boost.wasm",
            compile_eq_wasm_module(focus_manifest, [1.4, 0.8, -1.8, 1.2, -1.5], 11200.0),
        ),
    ]
}

fn escape_wat_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for b in s.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\{:02x}", b)),
        }
    }
    out
}

pub fn compile_eq_wasm_module(
    manifest_json: &str,
    deltas: [f32; 5],
    target_cutoff_hz: f32,
) -> Vec<u8> {
    let escaped = escape_wat_string(manifest_json);
    let byte_len = manifest_json.len();
    let cutoff_delta = 18500.0 - target_cutoff_hz;

    let wat_src = format!(
        r#"(module
  (memory (export "memory") 1)
  (data (i32.const 1024) "{escaped}")

  (func (export "lofi_abi_version") (result i32)
    (i32.const 1)
  )

  (func (export "lofi_metadata_ptr") (result i32)
    (i32.const 1024)
  )

  (func (export "lofi_metadata_len") (result i32)
    (i32.const {byte_len})
  )

  (func (export "lofi_result_ptr") (result i32)
    (i32.const 256)
  )

  ;; Computes out[i] = in[i] + delta[i] * intensity and writes 5 little-endian f32s to offset 256
  (func (export "lofi_transform_eq")
    (param $b0 f32) (param $b1 f32) (param $b2 f32) (param $b3 f32) (param $b4 f32) (param $k f32)
    (result f32)
    (f32.store (i32.const 256) (f32.add (local.get $b0) (f32.mul (f32.const {d0}) (local.get $k))))
    (f32.store (i32.const 260) (f32.add (local.get $b1) (f32.mul (f32.const {d1}) (local.get $k))))
    (f32.store (i32.const 264) (f32.add (local.get $b2) (f32.mul (f32.const {d2}) (local.get $k))))
    (f32.store (i32.const 268) (f32.add (local.get $b3) (f32.mul (f32.const {d3}) (local.get $k))))
    (f32.store (i32.const 272) (f32.add (local.get $b4) (f32.mul (f32.const {d4}) (local.get $k))))
    (f32.sub (f32.const 18500.0) (f32.mul (f32.const {cutoff_delta}) (local.get $k)))
  )

  (func (export "lofi_on_event") (param $kind i32) (param $value i32) (result i32)
    (i32.add (local.get $kind) (local.get $value))
  )
)"#,
        escaped = escaped,
        byte_len = byte_len,
        d0 = deltas[0],
        d1 = deltas[1],
        d2 = deltas[2],
        d3 = deltas[3],
        d4 = deltas[4],
        cutoff_delta = cutoff_delta,
    );

    wat::parse_str(&wat_src).expect("Starter WAT module must compile cleanly to valid Wasm")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{PluginManager, PluginEvent};

    #[test]
    fn loads_and_executes_starter_wasm_plugins_via_trait() {
        let starters = starter_wasm_modules();
        assert_eq!(starters.len(), 2);

        for (name, bytes) in starters {
            let plugin = WasmPlugin::from_bytes(&bytes, Some(name.to_string()))
                .expect("Starter Wasm module should instantiate");
            let trait_obj: Box<dyn LofiPlugin> = Box::new(plugin);

            let meta = trait_obj.metadata();
            assert_eq!(meta.kind, PluginKind::Wasm);
            assert!(!meta.name.is_empty());
            assert!(!trait_obj.eq_presets().is_empty());

            let res = trait_obj
                .transform_eq([0.0, 0.0, 0.0, 0.0, 0.0], 1.0)
                .expect("Wasm transform_eq should succeed");
            assert_ne!(res.output_bands, [0.0, 0.0, 0.0, 0.0, 0.0]);
            assert!(res.suggested_tape_cutoff_hz < 18500.0);

            let ack = trait_obj
                .on_event(&PluginEvent::TrackChanged {
                    bpm_hint: 78,
                    mood_code: 1,
                })
                .expect("Wasm on_event should succeed");
            assert_eq!(ack, 80);
        }
    }

    #[test]
    fn plugin_manager_loads_native_and_wasm_from_disk() {
        let temp_dir = std::env::temp_dir().join(format!(
            "lofi_wasm_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        PluginManager::ensure_starter_wasm_plugins(&temp_dir).unwrap();

        let mut mgr = PluginManager::new(&[]);
        mgr.reload(&[temp_dir.clone()]);
        let snap = mgr.snapshot();

        assert!(snap.load_errors.is_empty(), "errors: {:?}", snap.load_errors);
        assert_eq!(snap.plugins.len(), 3); // 1 native + 2 wasm
        let (total, wasm) = mgr.counts();
        assert_eq!(total, 3);
        assert_eq!(wasm, 2);

        let eq = mgr
            .apply_eq_transform("wasm-midnight-tape", [1.0, 0.0, 0.0, 0.0, 0.0], 0.5)
            .unwrap();
        assert!((eq.output_bands[0] - 2.9).abs() < 0.05);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn loads_example_wat_plugin_from_repo() {
        let wat_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("plugins")
            .join("rainy-lo-fi-warmth.wat");
        let plugin = WasmPlugin::from_file(&wat_path)
            .expect("Example rainy-lo-fi-warmth.wat should compile and load cleanly");
        assert_eq!(plugin.id(), "wasm-rainy-lofi-example");
        let out = plugin.transform_eq([0.0; 5], 1.0).unwrap();
        assert!((out.output_bands[0] - 3.2).abs() < 0.05);
        assert!((out.suggested_tape_cutoff_hz - 5000.0).abs() < 1.0);
    }
}
