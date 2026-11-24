use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDirectories {
    pub app_data_dir: String,
    pub default_music_dir: String,
    pub default_visuals_dir: String,
    pub default_sounds_dir: String,
    pub default_plugins_dir: String,
    pub custom_music_dirs: Vec<String>,
    pub custom_visual_dirs: Vec<String>,
    pub custom_sound_dirs: Vec<String>,
    pub custom_plugin_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Audio,
    Image,
    Video,
    Ambient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub file_path: String,
    pub kind: MediaKind,
    pub extension: String,
    pub size_bytes: u64,
    pub modified_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub music: Vec<MediaItem>,
    pub visuals: Vec<MediaItem>,
    pub ambient_sounds: Vec<MediaItem>,
    pub directories: LibraryDirectories,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqPreset {
    pub id: String,
    pub name: String,
    /// 5 bands in dB: [60Hz, 230Hz, 910Hz, 3.6kHz, 12kHz]
    pub bands: [f32; 5],
    /// Low-pass lofi tape cutoff frequency in Hz (1600..20000)
    pub tape_cutoff_hz: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualFxSettings {
    pub rain_overlay: f32,
    pub vignette: f32,
    pub dim: f32,
    pub warm_tint: f32,
    pub fps_cap: u32,
    pub pause_when_hidden: bool,
}

impl Default for VisualFxSettings {
    fn default() -> Self {
        Self {
            rain_overlay: 0.55,
            vignette: 0.35,
            dim: 0.04,
            warm_tint: 0.08,
            fps_cap: 30,
            pause_when_hidden: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSource {
    /// "canvas" | "image" | "video"
    pub kind: String,
    /// Built-in canvas scene id or local file path
    pub id: String,
    pub label: String,
}

impl Default for VisualSource {
    fn default() -> Self {
        Self {
            kind: "canvas".to_string(),
            id: "rainy-window-cat".to_string(),
            label: "Rainy Window Cat".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceRef {
    /// "station" | "local"
    pub kind: String,
    pub id: String,
    pub label: String,
}

impl Default for AudioSourceRef {
    fn default() -> Self {
        Self {
            kind: "station".to_string(),
            id: "focus-zone-relax-mellow".to_string(),
            label: "focus-zone-relax-mellow".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenePreset {
    pub id: String,
    pub name: String,
    pub mood: String,
    pub description: String,
    pub builtin: bool,
    pub visual: VisualSource,
    pub visual_fx: VisualFxSettings,
    pub eq_preset_id: String,
    pub eq_bands: [f32; 5],
    pub tape_cutoff_hz: f32,
    pub ambience: HashMap<String, f32>,
    pub audio_source: AudioSourceRef,
    pub accent_rgb: [u8; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    /// "track" | "station" | "scene"
    pub kind: String,
    pub ref_id: String,
    pub timestamp_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusTask {
    pub id: String,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub volume: f32,
    pub muted: bool,
    pub active_scene_id: String,
    pub visual: VisualSource,
    pub visual_fx: VisualFxSettings,
    pub eq_preset_id: String,
    pub eq_bands: [f32; 5],
    pub tape_cutoff_hz: f32,
    pub ambience: HashMap<String, f32>,
    pub audio_source: AudioSourceRef,
    pub shuffle: bool,
    pub repeat_mode: String,
    pub custom_music_dirs: Vec<String>,
    pub custom_visual_dirs: Vec<String>,
    pub custom_sound_dirs: Vec<String>,
    pub custom_plugin_dirs: Vec<String>,
    pub extra_files: Vec<String>,
    pub custom_eq_presets: Vec<EqPreset>,
    pub focus_minutes: u32,
    pub break_minutes: u32,
    pub completed_focus_sessions: u32,
    pub focus_tasks: Vec<FocusTask>,
    pub disabled_plugins: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            volume: 0.72,
            muted: false,
            active_scene_id: "rainy-window-chill".to_string(),
            visual: VisualSource::default(),
            visual_fx: VisualFxSettings::default(),
            eq_preset_id: "flat".to_string(),
            eq_bands: [0.0, 0.0, 0.0, 0.0, 0.0],
            tape_cutoff_hz: 18500.0,
            ambience: HashMap::new(),
            audio_source: AudioSourceRef::default(),
            shuffle: false,
            repeat_mode: "all".to_string(),
            custom_music_dirs: Vec::new(),
            custom_visual_dirs: Vec::new(),
            custom_sound_dirs: Vec::new(),
            custom_plugin_dirs: Vec::new(),
            extra_files: Vec::new(),
            custom_eq_presets: Vec::new(),
            focus_minutes: 25,
            break_minutes: 5,
            completed_focus_sessions: 0,
            focus_tasks: vec![FocusTask {
                id: "task-1".to_string(),
                text: "Deep work session with lofi beats".to_string(),
                done: false,
            }],
            disabled_plugins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PluginKind {
    Builtin,
    Wasm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockModuleSpec {
    pub id: String,
    pub plugin_id: String,
    pub label: String,
    pub icon: String,
    pub shortcut: Option<String>,
    pub order: i32,
    /// "builtin" | "plugin"
    pub panel_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub kind: PluginKind,
    pub file_path: Option<String>,
    pub capabilities: Vec<String>,
    pub dock_module: Option<DockModuleSpec>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMetricItem {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionButton {
    pub id: String,
    pub label: String,
    #[serde(default = "default_btn_style")]
    pub style: String,
}

fn default_btn_style() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PluginWidget {
    Text {
        label: String,
        value: String,
    },
    Metrics {
        items: Vec<PluginMetricItem>,
    },
    Slider {
        id: String,
        label: String,
        min: f32,
        max: f32,
        step: f32,
        value: f32,
        unit: String,
    },
    Actions {
        title: Option<String>,
        buttons: Vec<PluginActionButton>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPanelView {
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub widgets: Vec<PluginWidget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PluginHostCommand {
    SetEq {
        bands: [f32; 5],
        tape_cutoff_hz: f32,
        label: String,
    },
    SetAmbience {
        channel: String,
        level: f32,
    },
    SetVisual {
        visual_id: String,
    },
    Toast {
        message: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionResponse {
    pub panel: Option<PluginPanelView>,
    pub commands: Vec<PluginHostCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEqTransformResult {
    pub plugin_id: String,
    pub plugin_name: String,
    pub input_bands: [f32; 5],
    pub output_bands: [f32; 5],
    pub suggested_tape_cutoff_hz: f32,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRegistrySnapshot {
    pub plugins: Vec<PluginMetadata>,
    pub dock_modules: Vec<DockModuleSpec>,
    pub eq_presets: Vec<EqPreset>,
    pub load_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LofiStation {
    pub id: &'static str,
    pub name: &'static str,
    pub mood: &'static str,
    pub listeners: u32,
    pub bpm: f32,
    pub description: &'static str,
    pub accent_rgb: [u8; 3],
    pub chords: &'static [[u8; 5]],
}

#[derive(Debug, Clone)]
pub struct AmbientChannelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub const BUILTIN_STATIONS: &[LofiStation] = &[
    LofiStation {
        id: "focus-zone-relax-mellow",
        name: "focus-zone-relax-mellow",
        mood: "CHILLING",
        listeners: 8,
        bpm: 74.0,
        description: "Warm electric piano chords and soft dusty boom-bap for calm evening flow.",
        accent_rgb: [59, 130, 246],
        chords: &[
            [53, 57, 60, 64, 67],
            [52, 55, 59, 62, 67],
            [50, 53, 57, 60, 64],
            [48, 52, 55, 59, 62],
        ],
    },
    LofiStation {
        id: "midnight-study-rhodes",
        name: "midnight-study-rhodes",
        mood: "FOCUSING",
        listeners: 14,
        bpm: 78.0,
        description: "Steady mellow Rhodes loops with minimal percussion for deep concentration.",
        accent_rgb: [139, 92, 246],
        chords: &[
            [57, 60, 64, 67, 71],
            [53, 57, 60, 64, 69],
            [55, 59, 62, 65, 69],
            [52, 55, 59, 62, 66],
        ],
    },
    LofiStation {
        id: "tokyo-afterhours-dub",
        name: "tokyo-afterhours-dub",
        mood: "DRIFTING",
        listeners: 11,
        bpm: 70.0,
        description: "Rain-soaked minor seventh harmonies and deep tape-filtered sub bass.",
        accent_rgb: [236, 72, 153],
        chords: &[
            [51, 55, 58, 62, 65],
            [50, 53, 57, 60, 65],
            [48, 51, 55, 58, 62],
            [46, 50, 53, 57, 60],
        ],
    },
    LofiStation {
        id: "sleepy-sunset-tape",
        name: "sleepy-sunset-tape",
        mood: "UNWINDING",
        listeners: 6,
        bpm: 64.0,
        description: "Slow hazy tape keys and whisper-soft brushes made for winding down.",
        accent_rgb: [245, 158, 11],
        chords: &[
            [55, 59, 62, 66, 69],
            [54, 57, 61, 64, 69],
            [52, 55, 59, 62, 66],
            [50, 54, 57, 61, 64],
        ],
    },
];

pub const AMBIENT_CHANNELS: &[AmbientChannelDef] = &[
    AmbientChannelDef { id: "rain", name: "Rain", description: "Steady midnight rain on glass" },
    AmbientChannelDef { id: "vinyl", name: "Vinyl", description: "Warm dusty record crackle" },
    AmbientChannelDef { id: "fireplace", name: "Fireplace", description: "Soft crackling hearth embers" },
    AmbientChannelDef { id: "thunder", name: "Thunder", description: "Distant low rolling rumbles" },
    AmbientChannelDef { id: "wind", name: "Night Wind", description: "Gentle breeze outside the window" },
    AmbientChannelDef { id: "cafe", name: "Cafe", description: "Quiet muffled coffeehouse room" },
    AmbientChannelDef { id: "forest", name: "Forest", description: "Soft evening crickets & leaves" },
    AmbientChannelDef { id: "ocean", name: "Ocean", description: "Slow rhythmic nocturnal surf" },
];

pub fn builtin_eq_presets() -> Vec<EqPreset> {
    vec![
        EqPreset {
            id: "flat".to_string(),
            name: "Flat EQ".to_string(),
            bands: [0.0, 0.0, 0.0, 0.0, 0.0],
            tape_cutoff_hz: 18500.0,
            source: "builtin".to_string(),
        },
        EqPreset {
            id: "warm-lofi".to_string(),
            name: "Warm Lofi".to_string(),
            bands: [3.0, 2.0, -0.5, -2.0, -3.5],
            tape_cutoff_hz: 6800.0,
            source: "builtin".to_string(),
        },
        EqPreset {
            id: "bass-boost".to_string(),
            name: "Bass Boost".to_string(),
            bands: [5.5, 3.0, 0.0, -0.5, 0.5],
            tape_cutoff_hz: 14500.0,
            source: "builtin".to_string(),
        },
        EqPreset {
            id: "late-night".to_string(),
            name: "Late Night".to_string(),
            bands: [4.0, 1.5, -1.5, -3.0, -4.5],
            tape_cutoff_hz: 5200.0,
            source: "builtin".to_string(),
        },
        EqPreset {
            id: "radio-vinyl".to_string(),
            name: "Radio Vinyl".to_string(),
            bands: [-2.0, 2.5, 3.0, -1.0, -5.0],
            tape_cutoff_hz: 4800.0,
            source: "builtin".to_string(),
        },
        EqPreset {
            id: "focus-clarity".to_string(),
            name: "Focus Clarity".to_string(),
            bands: [1.0, 0.0, -1.0, 2.0, 1.5],
            tape_cutoff_hz: 16000.0,
            source: "builtin".to_string(),
        },
    ]
}
