pub mod native;
pub mod wasm_runtime;

use crate::models::{
    EqPreset, PluginEqTransformResult, PluginKind, PluginMetadata, PluginRegistrySnapshot,
    PluginSceneContribution,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("WebAssembly runtime error: {0}")]
    Wasm(String),
    #[error("Plugin ABI mismatch: expected v{expected}, got v{found}")]
    AbiMismatch { expected: i32, found: i32 },
    #[error("Invalid plugin metadata: {0}")]
    InvalidMetadata(String),
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Plugin '{0}' is currently disabled")]
    Disabled(String),
}

#[derive(Debug, Clone)]
pub enum PluginEvent {
    TrackChanged { bpm_hint: i32, mood_code: i32 },
    SceneChanged { scene_index: i32 },
    FocusSessionComplete { total_sessions: i32 },
}

/// Unified Rust trait interface shared by both native plugins and dynamically
/// loaded WebAssembly (.wasm / .wat) plugins.
pub trait LofiPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn metadata(&self) -> PluginMetadata;
    fn eq_presets(&self) -> Vec<EqPreset>;
    fn scene_contributions(&self) -> Vec<PluginSceneContribution>;
    fn transform_eq(
        &self,
        bands: [f32; 5],
        intensity: f32,
    ) -> Result<PluginEqTransformResult, PluginError>;
    fn on_event(&self, event: &PluginEvent) -> Result<i32, PluginError>;
}

pub struct PluginManager {
    plugins: Vec<Box<dyn LofiPlugin>>,
    disabled: HashSet<String>,
    load_errors: Vec<String>,
}

impl PluginManager {
    pub fn new(disabled_ids: &[String]) -> Self {
        Self {
            plugins: Vec::new(),
            disabled: disabled_ids.iter().cloned().collect(),
            load_errors: Vec::new(),
        }
    }

    /// Seeds starter .wasm binaries into the default plugins directory on first launch
    /// so users immediately have real dynamically loaded Wasm modules on disk.
    pub fn ensure_starter_wasm_plugins(plugins_dir: &Path) -> Result<(), PluginError> {
        fs::create_dir_all(plugins_dir).map_err(|e| PluginError::Io(e.to_string()))?;

        let starters = wasm_runtime::starter_wasm_modules();
        for (filename, wasm_bytes) in starters {
            let dest = plugins_dir.join(filename);
            if !dest.exists() {
                fs::write(&dest, wasm_bytes).map_err(|e| PluginError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn reload(&mut self, plugin_dirs: &[PathBuf]) {
        self.plugins.clear();
        self.load_errors.clear();

        // 1. Register unified native built-in plugin
        self.plugins.push(Box::new(native::CoreAtmospherePlugin::new()));

        // 2. Discover & dynamically load .wasm and .wat plugins from all configured directories
        let mut seen_ids: HashSet<String> = self
            .plugins
            .iter()
            .map(|p| p.id().to_string())
            .collect();

        for dir in plugin_dirs {
            if !dir.exists() {
                continue;
            }
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && matches!(
                            p.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
                            Some("wasm") | Some("wat")
                        )
                })
                .collect();
            paths.sort();

            for path in paths {
                match wasm_runtime::WasmPlugin::from_file(&path) {
                    Ok(plugin) => {
                        if seen_ids.insert(plugin.id().to_string()) {
                            self.plugins.push(Box::new(plugin));
                        }
                    }
                    Err(err) => {
                        self.load_errors
                            .push(format!("{}: {err}", path.display()));
                    }
                }
            }
        }
    }

    pub fn set_enabled(&mut self, plugin_id: &str, enabled: bool) -> Result<(), PluginError> {
        if !self.plugins.iter().any(|p| p.id() == plugin_id) {
            return Err(PluginError::NotFound(plugin_id.to_string()));
        }
        if enabled {
            self.disabled.remove(plugin_id);
        } else {
            self.disabled.insert(plugin_id.to_string());
        }
        Ok(())
    }

    pub fn disabled_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.disabled.iter().cloned().collect();
        ids.sort();
        ids
    }

    pub fn snapshot(&self) -> PluginRegistrySnapshot {
        let mut plugins = Vec::new();
        let mut eq_presets = Vec::new();
        let mut scene_contributions = Vec::new();

        for plugin in &self.plugins {
            let mut meta = plugin.metadata();
            let enabled = !self.disabled.contains(&meta.id);
            meta.enabled = enabled;
            plugins.push(meta);

            if enabled {
                eq_presets.extend(plugin.eq_presets());
                scene_contributions.extend(plugin.scene_contributions());
            }
        }

        PluginRegistrySnapshot {
            plugins,
            eq_presets,
            scene_contributions,
            load_errors: self.load_errors.clone(),
        }
    }

    pub fn apply_eq_transform(
        &self,
        plugin_id: &str,
        bands: [f32; 5],
        intensity: f32,
    ) -> Result<PluginEqTransformResult, PluginError> {
        if self.disabled.contains(plugin_id) {
            return Err(PluginError::Disabled(plugin_id.to_string()));
        }
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.id() == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;
        plugin.transform_eq(bands, intensity.clamp(0.0, 1.0))
    }

    pub fn dispatch_event(&self, event: &PluginEvent) -> usize {
        let mut handled = 0;
        for plugin in &self.plugins {
            if !self.disabled.contains(plugin.id()) && plugin.on_event(event).is_ok() {
                handled += 1;
            }
        }
        handled
    }

    pub fn counts(&self) -> (usize, usize) {
        let total = self.plugins.len();
        let wasm = self
            .plugins
            .iter()
            .filter(|p| p.metadata().kind == PluginKind::Wasm)
            .count();
        (total, wasm)
    }
}
