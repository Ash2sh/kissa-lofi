pub mod builtin;
pub mod wasm_runtime;

use crate::models::{
    DockModuleSpec, EqPreset, PluginActionResponse, PluginEqTransformResult,
    PluginMetadata, PluginPanelView, PluginRegistrySnapshot,
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
    #[error("Plugin '{0}' does not support this capability")]
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub enum PluginEvent {
    TrackChanged { bpm_hint: i32, mood_code: i32 },
    SceneChanged { scene_index: i32 },
    FocusSessionComplete { total_sessions: i32 },
}

/// Unified Rust trait interface shared by ALL built-in dock module plugins
/// (EQ, Music, Visuals, Sounds, Scenes, History, Share, Keys, Focus, Sleep)
/// and dynamically loaded WebAssembly (.wasm / .wat) plugins.
pub trait LofiPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn metadata(&self) -> PluginMetadata;
    fn dock_module(&self) -> Option<DockModuleSpec>;

    fn eq_presets(&self) -> Vec<EqPreset> {
        Vec::new()
    }

    fn render_panel(&self) -> Result<Option<PluginPanelView>, PluginError> {
        Ok(None)
    }

    fn invoke_action(
        &self,
        _action_id: &str,
        _value: f32,
        _current_eq: [f32; 5],
    ) -> Result<PluginActionResponse, PluginError> {
        Err(PluginError::Unsupported(self.id().to_string()))
    }

    fn transform_eq(
        &self,
        _bands: [f32; 5],
        _intensity: f32,
    ) -> Result<PluginEqTransformResult, PluginError> {
        Err(PluginError::Unsupported(self.id().to_string()))
    }

    fn on_event(&self, _event: &PluginEvent) -> Result<i32, PluginError> {
        Ok(0)
    }
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

    pub fn ensure_starter_wasm_plugins(plugins_dir: &Path) -> Result<(), PluginError> {
        fs::create_dir_all(plugins_dir).map_err(|e| PluginError::Io(e.to_string()))?;
        for (filename, wasm_bytes) in wasm_runtime::starter_wasm_modules() {
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

        for builtin_plugin in builtin::builtin_module_plugins() {
            self.plugins.push(builtin_plugin);
        }

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
                            p.extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.to_ascii_lowercase())
                                .as_deref(),
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
                        self.load_errors.push(format!("{}: {err}", path.display()));
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
        let mut dock_modules = Vec::new();
        let mut eq_presets = Vec::new();

        for plugin in &self.plugins {
            let mut meta = plugin.metadata();
            let enabled = !self.disabled.contains(&meta.id);
            meta.enabled = enabled;
            plugins.push(meta);

            if enabled {
                if let Some(dock) = plugin.dock_module() {
                    dock_modules.push(dock);
                }
                eq_presets.extend(plugin.eq_presets());
            }
        }

        dock_modules.sort_by_key(|m| m.order);

        PluginRegistrySnapshot {
            plugins,
            dock_modules,
            eq_presets,
            load_errors: self.load_errors.clone(),
        }
    }

    pub fn render_plugin_panel(
        &self,
        plugin_id: &str,
    ) -> Result<Option<PluginPanelView>, PluginError> {
        if self.disabled.contains(plugin_id) {
            return Err(PluginError::Disabled(plugin_id.to_string()));
        }
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.id() == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;
        plugin.render_panel()
    }

    pub fn invoke_plugin_action(
        &self,
        plugin_id: &str,
        action_id: &str,
        value: f32,
        current_eq: [f32; 5],
    ) -> Result<PluginActionResponse, PluginError> {
        if self.disabled.contains(plugin_id) {
            return Err(PluginError::Disabled(plugin_id.to_string()));
        }
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.id() == plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;
        plugin.invoke_action(action_id, value, current_eq)
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
}
