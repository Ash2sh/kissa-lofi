use crate::models::{
    DockModuleSpec, EqPreset, PluginActionButton, PluginActionResponse, PluginEqTransformResult,
    PluginHostCommand, PluginKind, PluginMetadata, PluginMetricItem, PluginPanelView, PluginWidget,
};
use crate::plugins::{LofiPlugin, PluginError, PluginEvent};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use wasmi::{Engine, Instance, Linker, Module, Store};

const EXPECTED_ABI_VERSION: i32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmDockSpecJson {
    id: String,
    label: String,
    icon: String,
    #[serde(default)]
    shortcut: Option<String>,
    #[serde(default = "default_wasm_order")]
    order: i32,
}

fn default_wasm_order() -> i32 {
    120
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmPanelActionJson {
    id: String,
    label: String,
    #[serde(default = "default_btn_style")]
    style: String,
    #[serde(default)]
    effect: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    amount: Option<f32>,
}

fn default_btn_style() -> String {
    "default".to_string()
}

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
    dock_module: Option<WasmDockSpecJson>,
    #[serde(default)]
    panel_subtitle: Option<String>,
    #[serde(default)]
    panel_actions: Vec<WasmPanelActionJson>,
    #[serde(default)]
    eq_presets: Vec<WasmEqPresetJson>,
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

struct WasmState {
    store: Store<()>,
    instance: Instance,
    intensity: f32,
    last_cutoff_hz: f32,
    invocations: u32,
}

pub struct WasmPlugin {
    metadata: PluginMetadata,
    dock_module: Option<DockModuleSpec>,
    panel_subtitle: String,
    panel_actions: Vec<WasmPanelActionJson>,
    eq_presets: Vec<EqPreset>,
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
            .map_err(|e| PluginError::Wasm(format!("Failed reading metadata: {e}")))?;
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

        let dock_module = manifest.dock_module.map(|d| DockModuleSpec {
            id: d.id,
            plugin_id: manifest.id.clone(),
            label: d.label,
            icon: d.icon,
            shortcut: d.shortcut,
            order: d.order,
            panel_kind: "plugin".to_string(),
        });

        let panel_subtitle = manifest
            .panel_subtitle
            .unwrap_or_else(|| format!("WebAssembly dock plugin · v{}", manifest.version));

        let metadata = PluginMetadata {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            author: manifest.author,
            description: manifest.description,
            kind: PluginKind::Wasm,
            file_path,
            capabilities: manifest.capabilities,
            dock_module: dock_module.clone(),
            enabled: true,
        };

        let transform_note = manifest
            .transform_note
            .unwrap_or_else(|| "Executed Wasm EQ DSP curve transform".to_string());

        Ok(Self {
            metadata,
            dock_module,
            panel_subtitle,
            panel_actions: manifest.panel_actions,
            eq_presets,
            transform_note,
            runtime: Mutex::new(WasmState {
                store,
                instance,
                intensity: 0.75,
                last_cutoff_hz: 18500.0,
                invocations: 0,
            }),
        })
    }

    fn build_panel_view(&self, state: &WasmState) -> Option<PluginPanelView> {
        let dock = self.dock_module.as_ref()?;
        let mut widgets = vec![
            PluginWidget::Text {
                label: "About Plugin".to_string(),
                value: self.metadata.description.clone(),
            },
            PluginWidget::Metrics {
                items: vec![
                    PluginMetricItem {
                        label: "Runtime".to_string(),
                        value: "WebAssembly (wasmi)".to_string(),
                    },
                    PluginMetricItem {
                        label: "DSP Runs".to_string(),
                        value: state.invocations.to_string(),
                    },
                    PluginMetricItem {
                        label: "Last Tape Cutoff".to_string(),
                        value: format!("{:.1} kHz", state.last_cutoff_hz / 1000.0),
                    },
                ],
            },
        ];

        if self.metadata.capabilities.iter().any(|c| c == "eq-dsp") {
            widgets.push(PluginWidget::Slider {
                id: "intensity".to_string(),
                label: "Wasm DSP Intensity".to_string(),
                min: 0.0,
                max: 100.0,
                step: 5.0,
                value: (state.intensity * 100.0).round(),
                unit: "%".to_string(),
            });
        }

        let buttons: Vec<PluginActionButton> = if !self.panel_actions.is_empty() {
            self.panel_actions
                .iter()
                .map(|a| PluginActionButton {
                    id: a.id.clone(),
                    label: a.label.clone(),
                    style: a.style.clone(),
                })
                .collect()
        } else if self.metadata.capabilities.iter().any(|c| c == "eq-dsp") {
            vec![
                PluginActionButton {
                    id: "apply-dsp".to_string(),
                    label: format!("Apply {} DSP", self.metadata.name),
                    style: "accent".to_string(),
                },
                PluginActionButton {
                    id: "reset-eq".to_string(),
                    label: "Reset Flat EQ".to_string(),
                    style: "default".to_string(),
                },
            ]
        } else {
            Vec::new()
        };

        if !buttons.is_empty() {
            widgets.push(PluginWidget::Actions {
                title: Some("Plugin Controls".to_string()),
                buttons,
            });
        }

        Some(PluginPanelView {
            title: self.metadata.name.clone(),
            subtitle: self.panel_subtitle.clone(),
            icon: dock.icon.clone(),
            widgets,
        })
    }

    fn execute_eq_locked(
        &self,
        state: &mut WasmState,
        bands: [f32; 5],
        intensity: f32,
    ) -> Result<PluginEqTransformResult, PluginError> {
        let WasmState {
            store, instance, ..
        } = state;

        let transform_fn = instance
            .get_typed_func::<(f32, f32, f32, f32, f32, f32), f32>(&*store, "lofi_transform_eq")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_transform_eq': {e}")))?;
        let result_ptr_fn = instance
            .get_typed_func::<(), i32>(&*store, "lofi_result_ptr")
            .map_err(|e| PluginError::Wasm(format!("Missing export 'lofi_result_ptr': {e}")))?;

        let clamped_k = intensity.clamp(0.0, 1.0);
        let cutoff_hz = transform_fn
            .call(
                &mut *store,
                (bands[0], bands[1], bands[2], bands[3], bands[4], clamped_k),
            )
            .map_err(|e| PluginError::Wasm(format!("lofi_transform_eq trap: {e}")))?;

        let result_ptr = result_ptr_fn
            .call(&mut *store, ())
            .map_err(|e| PluginError::Wasm(format!("lofi_result_ptr trap: {e}")))?
            as usize;

        let memory = instance
            .get_memory(&*store, "memory")
            .ok_or_else(|| PluginError::Wasm("Missing export 'memory'".to_string()))?;

        let mut raw_bytes = [0u8; 20];
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

        let clamped_cutoff = cutoff_hz.clamp(1600.0, 20000.0);
        state.last_cutoff_hz = clamped_cutoff;
        state.invocations = state.invocations.saturating_add(1);

        Ok(PluginEqTransformResult {
            plugin_id: self.metadata.id.clone(),
            plugin_name: self.metadata.name.clone(),
            input_bands: bands,
            output_bands,
            suggested_tape_cutoff_hz: clamped_cutoff,
            note: self.transform_note.clone(),
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

    fn dock_module(&self) -> Option<DockModuleSpec> {
        self.dock_module.clone()
    }

    fn eq_presets(&self) -> Vec<EqPreset> {
        self.eq_presets.clone()
    }

    fn render_panel(&self) -> Result<Option<PluginPanelView>, PluginError> {
        let guard = self
            .runtime
            .lock()
            .map_err(|_| PluginError::Wasm("Plugin mutex poisoned".to_string()))?;
        Ok(self.build_panel_view(&guard))
    }

    fn invoke_action(
        &self,
        action_id: &str,
        value: f32,
        current_eq: [f32; 5],
    ) -> Result<PluginActionResponse, PluginError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| PluginError::Wasm("Plugin mutex poisoned".to_string()))?;

        if action_id == "intensity" {
            guard.intensity = (value / 100.0).clamp(0.0, 1.0);
            let panel = self.build_panel_view(&guard);
            return Ok(PluginActionResponse {
                panel,
                commands: Vec::new(),
            });
        }

        let mut commands = Vec::new();
        let matched_action = self.panel_actions.iter().find(|a| a.id == action_id);
        let effect = matched_action
            .map(|a| a.effect.as_str())
            .unwrap_or(action_id);

        match effect {
            "wasm-eq" | "apply-dsp" => {
                let k = matched_action
                    .and_then(|a| a.amount)
                    .unwrap_or(guard.intensity);
                let res = self.execute_eq_locked(&mut guard, current_eq, k)?;
                commands.push(PluginHostCommand::SetEq {
                    bands: res.output_bands,
                    tape_cutoff_hz: res.suggested_tape_cutoff_hz,
                    label: format!("{} (Wasm)", self.metadata.name),
                });
                commands.push(PluginHostCommand::Toast {
                    message: format!("{}: {}", self.metadata.name, res.note),
                });
            }
            "reset-eq" => {
                guard.last_cutoff_hz = 18500.0;
                commands.push(PluginHostCommand::SetEq {
                    bands: [0.0, 0.0, 0.0, 0.0, 0.0],
                    tape_cutoff_hz: 18500.0,
                    label: "Flat EQ".to_string(),
                });
                commands.push(PluginHostCommand::Toast {
                    message: "Reset EQ to Flat".to_string(),
                });
            }
            "ambience" => {
                if let Some(channel) = matched_action.and_then(|a| a.target.clone()) {
                    let level = matched_action.and_then(|a| a.amount).unwrap_or(0.42);
                    commands.push(PluginHostCommand::SetAmbience {
                        channel: channel.clone(),
                        level,
                    });
                    commands.push(PluginHostCommand::Toast {
                        message: format!(
                            "{}: set {} to {:.0}%",
                            self.metadata.name,
                            channel,
                            level * 100.0
                        ),
                    });
                }
            }
            "visual" => {
                if let Some(visual_id) = matched_action.and_then(|a| a.target.clone()) {
                    commands.push(PluginHostCommand::SetVisual { visual_id });
                }
            }
            _ => {
                commands.push(PluginHostCommand::Toast {
                    message: format!("Triggered '{}' on {}", action_id, self.metadata.name),
                });
            }
        }

        let panel = self.build_panel_view(&guard);
        Ok(PluginActionResponse { panel, commands })
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
        self.execute_eq_locked(&mut guard, bands, intensity)
    }

    fn on_event(&self, event: &PluginEvent) -> Result<i32, PluginError> {
        let mut guard = self
            .runtime
            .lock()
            .map_err(|_| PluginError::Wasm("Plugin mutex poisoned".to_string()))?;
        let WasmState {
            store, instance, ..
        } = &mut *guard;

        let Ok(event_fn) = instance.get_typed_func::<(i32, i32), i32>(&*store, "lofi_on_event")
        else {
            return Ok(0);
        };

        let (kind, value) = match event {
            PluginEvent::TrackChanged {
                bpm_hint,
                mood_code,
            } => (1, bpm_hint.saturating_add(*mood_code)),
            PluginEvent::SceneChanged { scene_index } => (2, *scene_index),
            PluginEvent::FocusSessionComplete { total_sessions } => (3, *total_sessions),
        };

        event_fn
            .call(&mut *store, (kind, value))
            .map_err(|e| PluginError::Wasm(format!("lofi_on_event trap: {e}")))
    }
}

pub fn starter_wasm_modules() -> Vec<(&'static str, Vec<u8>)> {
    let tape_lab_manifest = r##"{"id":"wasm-tape-lab","name":"Tape Lab","version":"1.0.0","author":"Kissa Wasm SDK","description":"External WebAssembly dock plugin that sculpts analog reel-to-reel warmth, dusty vinyl texture, and mellow roll-off inside a wasmi sandbox.","capabilities":["dock-module","eq-dsp","eq-presets","event-hooks"],"dockModule":{"id":"wasm-tape-lab","label":"Tape","icon":"tape","order":110},"panelSubtitle":"External WebAssembly (.wasm) dock plugin","panelActions":[{"id":"apply-tape","label":"Apply Tape Curve","style":"accent","effect":"wasm-eq"},{"id":"vinyl-dust","label":"Add Vinyl Dust (35%)","style":"default","effect":"ambience","target":"vinyl","amount":0.35},{"id":"rain-glow","label":"Add Soft Rain (45%)","style":"default","effect":"ambience","target":"rain","amount":0.45},{"id":"reset-flat","label":"Reset Flat EQ","style":"default","effect":"reset-eq"}],"eqPresets":[{"id":"wasm-midnight-reel","name":"Midnight Reel (Wasm)","bands":[3.8,2.2,-1.0,-2.8,-5.2],"tapeCutoffHz":4600}],"transformNote":"Sculpted analog reel-to-reel curve inside WebAssembly sandbox"}"##;

    vec![(
        "tape-lab.wasm",
        compile_eq_wasm_module(tape_lab_manifest, [3.8, 2.2, -1.0, -2.8, -5.2], 4600.0),
    )]
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

  (func (export "lofi_abi_version") (result i32) (i32.const 1))
  (func (export "lofi_metadata_ptr") (result i32) (i32.const 1024))
  (func (export "lofi_metadata_len") (result i32) (i32.const {byte_len}))
  (func (export "lofi_result_ptr") (result i32) (i32.const 256))

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
    use crate::plugins::{PluginEvent, PluginManager};

    #[test]
    fn loads_and_executes_starter_wasm_dock_plugin_via_trait() {
        let starters = starter_wasm_modules();
        assert_eq!(starters.len(), 1);
        let (name, bytes) = &starters[0];
        let plugin = WasmPlugin::from_bytes(bytes, Some((*name).to_string())).unwrap();
        let trait_obj: Box<dyn LofiPlugin> = Box::new(plugin);
        assert_eq!(trait_obj.dock_module().unwrap().label, "Tape");
        let panel = trait_obj.render_panel().unwrap().unwrap();
        assert_eq!(panel.title, "Tape Lab");
        let res = trait_obj.invoke_action("apply-tape", 0.0, [0.0; 5]).unwrap();
        assert!(!res.commands.is_empty());
        assert_eq!(
            trait_obj
                .on_event(&PluginEvent::TrackChanged {
                    bpm_hint: 78,
                    mood_code: 1
                })
                .unwrap(),
            80
        );
    }

    #[test]
    fn plugin_manager_registers_10_builtin_dock_plugins_plus_wasm() {
        let temp_dir = std::env::temp_dir().join(format!("kissa_native_wasm_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        PluginManager::ensure_starter_wasm_plugins(&temp_dir).unwrap();

        let mut mgr = PluginManager::new(&[]);
        mgr.reload(&[temp_dir.clone()]);
        let snap = mgr.snapshot();
        assert!(snap.load_errors.is_empty());
        assert_eq!(snap.dock_modules.len(), 11);

        mgr.set_enabled("builtin-share", false).unwrap();
        assert_eq!(mgr.snapshot().dock_modules.len(), 10);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn loads_example_wat_plugin_from_repo() {
        let wat_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins")
            .join("rainy-lo-fi-warmth.wat");
        let plugin = WasmPlugin::from_file(&wat_path).unwrap();
        assert_eq!(plugin.id(), "wasm-rainy-lofi-example");
        assert!(plugin.dock_module().is_some());
        let out = plugin.transform_eq([0.0; 5], 1.0).unwrap();
        assert!((out.output_bands[0] - 3.2).abs() < 0.05);
    }
}
