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
    pub stream_url: String,
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
    /// Low-pass lofi tape cutoff frequency in Hz (1200..20000)
    pub tape_cutoff_hz: f32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualFxSettings {
    pub rain_overlay: f32,
    pub vignette: f32,
    pub film_grain: f32,
    pub dim: f32,
    pub warm_tint: f32,
    pub blur: f32,
    pub fps_cap: u32,
    pub pause_when_hidden: bool,
}

impl Default for VisualFxSettings {
    fn default() -> Self {
        Self {
            rain_overlay: 0.45,
            vignette: 0.35,
            film_grain: 0.12,
            dim: 0.08,
            warm_tint: 0.1,
            blur: 0.0,
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
    pub stream_url: Option<String>,
}

impl Default for VisualSource {
    fn default() -> Self {
        Self {
            kind: "canvas".to_string(),
            id: "rainy-window-cat".to_string(),
            label: "Rainy Window Cat".to_string(),
            stream_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceRef {
    /// "station" | "local" | "stream"
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
    pub accent_color: String,
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
pub struct CustomStream {
    pub id: String,
    pub name: String,
    pub mood: String,
    pub url: String,
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
    pub allowed_files: Vec<String>,
    pub custom_eq_presets: Vec<EqPreset>,
    pub custom_streams: Vec<CustomStream>,
    pub focus_minutes: u32,
    pub break_minutes: u32,
    pub completed_focus_sessions: u32,
    pub focus_tasks: Vec<FocusTask>,
    pub disabled_plugins: Vec<String>,
    pub hud_auto_hide: bool,
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
            allowed_files: Vec::new(),
            custom_eq_presets: Vec::new(),
            custom_streams: Vec::new(),
            focus_minutes: 25,
            break_minutes: 5,
            completed_focus_sessions: 0,
            focus_tasks: vec![
                FocusTask {
                    id: "task-1".to_string(),
                    text: "Deep work session with lofi beats".to_string(),
                    done: false,
                },
            ],
            disabled_plugins: Vec::new(),
            hud_auto_hide: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PluginKind {
    Native,
    Wasm,
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
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSceneContribution {
    pub id: String,
    pub name: String,
    pub mood: String,
    pub description: String,
    pub visual_id: String,
    pub eq_bands: [f32; 5],
    pub tape_cutoff_hz: f32,
    pub ambience: HashMap<String, f32>,
    pub accent_color: String,
    pub plugin_id: String,
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
    pub eq_presets: Vec<EqPreset>,
    pub scene_contributions: Vec<PluginSceneContribution>,
    pub load_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetrics {
    pub uptime_secs: u64,
    pub loaded_plugins: usize,
    pub wasm_plugins: usize,
    pub indexed_tracks: usize,
    pub indexed_visuals: usize,
}
