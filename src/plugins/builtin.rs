use crate::models::{DockModuleSpec, PluginKind, PluginMetadata};
use crate::plugins::{LofiPlugin, PluginError, PluginEvent};

pub struct BuiltinModulePlugin {
    plugin_id: &'static str,
    module_id: &'static str,
    name: &'static str,
    label: &'static str,
    icon: &'static str,
    shortcut: &'static str,
    order: i32,
    description: &'static str,
}

impl BuiltinModulePlugin {
    #[allow(clippy::too_many_arguments)]
    const fn new(
        plugin_id: &'static str,
        module_id: &'static str,
        name: &'static str,
        label: &'static str,
        icon: &'static str,
        shortcut: &'static str,
        order: i32,
        description: &'static str,
    ) -> Self {
        Self {
            plugin_id,
            module_id,
            name,
            label,
            icon,
            shortcut,
            order,
            description,
        }
    }
}

impl LofiPlugin for BuiltinModulePlugin {
    fn id(&self) -> &str {
        self.plugin_id
    }

    fn dock_module(&self) -> Option<DockModuleSpec> {
        Some(DockModuleSpec {
            id: self.module_id.to_string(),
            plugin_id: self.plugin_id.to_string(),
            label: self.label.to_string(),
            icon: self.icon.to_string(),
            shortcut: Some(self.shortcut.to_string()),
            order: self.order,
            panel_kind: "builtin".to_string(),
        })
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: self.plugin_id.to_string(),
            name: self.name.to_string(),
            version: "1.0.0".to_string(),
            author: "Kissa Core".to_string(),
            description: self.description.to_string(),
            kind: PluginKind::Builtin,
            file_path: None,
            capabilities: vec!["dock-module".to_string()],
            dock_module: self.dock_module(),
            enabled: true,
        }
    }

    fn on_event(&self, _event: &PluginEvent) -> Result<i32, PluginError> {
        Ok(1)
    }
}

pub fn builtin_module_plugins() -> Vec<Box<dyn LofiPlugin>> {
    vec![
        Box::new(BuiltinModulePlugin::new(
            "builtin-eq",
            "eq",
            "Equalizer & Tape Filter",
            "EQ",
            "eq",
            "1",
            10,
            "5-band parametric native Rust DSP equalizer, tape low-pass filter, and EQ presets.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-music",
            "music",
            "Music & Local Library",
            "Music",
            "music",
            "2",
            20,
            "Procedural Rust lofi synth stations and streaming local audio library (MP3/FLAC/WAV/OGG).",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-visuals",
            "visuals",
            "Visual Backgrounds",
            "Visuals",
            "visuals",
            "3",
            30,
            "Native GPU Canvas scenes, local photo/video wallpapers, FX overlays, and FPS power caps.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-sounds",
            "sounds",
            "Ambient Soundscapes",
            "Sounds",
            "sounds",
            "4",
            40,
            "8-layer procedural Rust ambient soundscape synthesizer.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-scenes",
            "scenes",
            "Atmosphere Scenes",
            "Scenes",
            "scenes",
            "5",
            50,
            "Save and recall complete atmosphere snapshots combining visual, EQ, ambience, and audio.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-history",
            "history",
            "Listening History",
            "History",
            "history",
            "6",
            60,
            "Private on-disk history of recently played stations, local tracks, and scenes.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-share",
            "share",
            "Share & Import",
            "Share",
            "share",
            "7",
            70,
            "Export and import portable local-first scene codes and JSON presets.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-keys",
            "keys",
            "Keyboard Shortcuts",
            "Keys",
            "keys",
            "8",
            80,
            "Quick-reference keyboard shortcuts, Cinema Mode, and Fullscreen controls.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-focus",
            "focus",
            "Focus Timer",
            "Focus",
            "focus",
            "9",
            90,
            "Pomodoro / deep-work timer with completion chime and persistent local task checklist.",
        )),
        Box::new(BuiltinModulePlugin::new(
            "builtin-sleep",
            "sleep",
            "Sleep Timer",
            "Sleep",
            "sleep",
            "0",
            100,
            "Countdown sleep timer with smooth 30-second audio and ambience fade-out.",
        )),
    ]
}
