use crate::models::{
    AppConfig, HistoryEntry, LibraryDirectories, LibrarySnapshot, MediaItem, MediaKind, ScenePreset,
    VisualFxSettings, VisualSource, AudioSourceRef,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const AUDIO_EXTS: &[&str] = &["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus", "webm"];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif"];
const VIDEO_EXTS: &[&str] = &["mp4", "webm", "mov", "m4v", "ogv"];

pub struct LocalStorage {
    pub app_data_dir: PathBuf,
    pub music_dir: PathBuf,
    pub visuals_dir: PathBuf,
    pub sounds_dir: PathBuf,
    pub plugins_dir: PathBuf,
}

impl LocalStorage {
    pub fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let music_dir = app_data_dir.join("music");
        let visuals_dir = app_data_dir.join("visuals");
        let sounds_dir = app_data_dir.join("sounds");
        let plugins_dir = app_data_dir.join("plugins");

        for dir in [&app_data_dir, &music_dir, &visuals_dir, &sounds_dir, &plugins_dir] {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create directory {}: {e}", dir.display()))?;
        }

        Ok(Self {
            app_data_dir,
            music_dir,
            visuals_dir,
            sounds_dir,
            plugins_dir,
        })
    }

    fn config_path(&self) -> PathBuf {
        self.app_data_dir.join("config.json")
    }

    fn scenes_path(&self) -> PathBuf {
        self.app_data_dir.join("scenes.json")
    }

    fn history_path(&self) -> PathBuf {
        self.app_data_dir.join("history.json")
    }

    pub fn load_config(&self) -> AppConfig {
        let path = self.config_path();
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&raw) {
                return cfg;
            }
        }
        let cfg = AppConfig::default();
        let _ = self.save_config(&cfg);
        cfg
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;
        fs::write(self.config_path(), raw)
            .map_err(|e| format!("Failed to write config: {e}"))
    }

    pub fn load_scenes(&self) -> Vec<ScenePreset> {
        let mut scenes = default_builtin_scenes();
        if let Ok(raw) = fs::read_to_string(self.scenes_path()) {
            if let Ok(user_scenes) = serde_json::from_str::<Vec<ScenePreset>>(&raw) {
                for scene in user_scenes {
                    if !scene.builtin {
                        scenes.push(scene);
                    }
                }
            }
        }
        scenes
    }

    pub fn save_user_scenes(&self, all_scenes: &[ScenePreset]) -> Result<(), String> {
        let user_only: Vec<&ScenePreset> = all_scenes.iter().filter(|s| !s.builtin).collect();
        let raw = serde_json::to_string_pretty(&user_only)
            .map_err(|e| format!("Failed to serialize scenes: {e}"))?;
        fs::write(self.scenes_path(), raw)
            .map_err(|e| format!("Failed to write scenes: {e}"))
    }

    pub fn load_history(&self) -> Vec<HistoryEntry> {
        if let Ok(raw) = fs::read_to_string(self.history_path()) {
            if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&raw) {
                return entries;
            }
        }
        Vec::new()
    }

    pub fn save_history(&self, entries: &[HistoryEntry]) -> Result<(), String> {
        let trimmed = if entries.len() > 80 {
            &entries[..80]
        } else {
            entries
        };
        let raw = serde_json::to_string_pretty(trimmed)
            .map_err(|e| format!("Failed to serialize history: {e}"))?;
        fs::write(self.history_path(), raw)
            .map_err(|e| format!("Failed to write history: {e}"))
    }

    pub fn directories_snapshot(&self, config: &AppConfig) -> LibraryDirectories {
        LibraryDirectories {
            app_data_dir: self.app_data_dir.to_string_lossy().to_string(),
            default_music_dir: self.music_dir.to_string_lossy().to_string(),
            default_visuals_dir: self.visuals_dir.to_string_lossy().to_string(),
            default_sounds_dir: self.sounds_dir.to_string_lossy().to_string(),
            default_plugins_dir: self.plugins_dir.to_string_lossy().to_string(),
            custom_music_dirs: config.custom_music_dirs.clone(),
            custom_visual_dirs: config.custom_visual_dirs.clone(),
            custom_sound_dirs: config.custom_sound_dirs.clone(),
            custom_plugin_dirs: config.custom_plugin_dirs.clone(),
        }
    }

    pub fn scan_library(&self, config: &AppConfig) -> LibrarySnapshot {
        let mut music_roots = vec![self.music_dir.clone()];
        music_roots.extend(config.custom_music_dirs.iter().map(PathBuf::from));

        let mut visual_roots = vec![self.visuals_dir.clone()];
        visual_roots.extend(config.custom_visual_dirs.iter().map(PathBuf::from));

        let mut sound_roots = vec![self.sounds_dir.clone()];
        sound_roots.extend(config.custom_sound_dirs.iter().map(PathBuf::from));

        let mut music = scan_roots(&music_roots, AUDIO_EXTS, |_| MediaKind::Audio);
        let mut visuals = scan_roots(&visual_roots, &concat_exts(), |ext| {
            if VIDEO_EXTS.contains(&ext) {
                MediaKind::Video
            } else {
                MediaKind::Image
            }
        });
        let ambient_sounds = scan_roots(&sound_roots, AUDIO_EXTS, |_| MediaKind::Ambient);

        // Include individual user-picked files that live outside scanned folders
        let mut seen_paths: HashSet<String> = music
            .iter()
            .chain(visuals.iter())
            .chain(ambient_sounds.iter())
            .map(|item| normalize_path_key(Path::new(&item.file_path)))
            .collect();

        for raw_file in &config.allowed_files {
            let p = Path::new(raw_file);
            if !p.is_file() {
                continue;
            }
            let key = normalize_path_key(p);
            if seen_paths.contains(&key) {
                continue;
            }
            let Some(ext) = p.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()) else {
                continue;
            };
            if VIDEO_EXTS.contains(&ext.as_str()) {
                if let Some(item) = build_media_item(p, MediaKind::Video, &ext) {
                    seen_paths.insert(key);
                    visuals.push(item);
                }
            } else if IMAGE_EXTS.contains(&ext.as_str()) {
                if let Some(item) = build_media_item(p, MediaKind::Image, &ext) {
                    seen_paths.insert(key);
                    visuals.push(item);
                }
            } else if AUDIO_EXTS.contains(&ext.as_str()) {
                if let Some(item) = build_media_item(p, MediaKind::Audio, &ext) {
                    seen_paths.insert(key);
                    music.push(item);
                }
            }
        }

        music.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        visuals.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

        LibrarySnapshot {
            music,
            visuals,
            ambient_sounds,
            directories: self.directories_snapshot(config),
        }
    }

    /// Validates that a requested local file is within the app's local media folders,
    /// user-configured custom directories, or explicitly added individual files.
    pub fn is_path_allowed(&self, requested: &Path, config: &AppConfig) -> bool {
        let Ok(canon_req) = fs::canonicalize(requested) else {
            return false;
        };
        if !canon_req.is_file() {
            return false;
        }

        let mut allowed_dirs: Vec<PathBuf> = vec![
            self.music_dir.clone(),
            self.visuals_dir.clone(),
            self.sounds_dir.clone(),
        ];
        allowed_dirs.extend(config.custom_music_dirs.iter().map(PathBuf::from));
        allowed_dirs.extend(config.custom_visual_dirs.iter().map(PathBuf::from));
        allowed_dirs.extend(config.custom_sound_dirs.iter().map(PathBuf::from));

        for dir in allowed_dirs {
            if let Ok(canon_dir) = fs::canonicalize(&dir) {
                if canon_req.starts_with(&canon_dir) {
                    return true;
                }
            }
        }

        for file in &config.allowed_files {
            if let Ok(canon_file) = fs::canonicalize(file) {
                if canon_req == canon_file {
                    return true;
                }
            }
        }

        false
    }
}

pub fn make_stream_url(path: &Path) -> String {
    let raw = path.to_string_lossy();
    let encoded = utf8_percent_encode(&raw, NON_ALPHANUMERIC).to_string();
    format!("lofi://localhost/media?path={encoded}")
}

fn concat_exts() -> Vec<&'static str> {
    let mut v = Vec::with_capacity(IMAGE_EXTS.len() + VIDEO_EXTS.len());
    v.extend_from_slice(IMAGE_EXTS);
    v.extend_from_slice(VIDEO_EXTS);
    v
}

fn normalize_path_key(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_ascii_lowercase()
}

fn scan_roots<F>(roots: &[PathBuf], allowed_exts: &[&str], classify: F) -> Vec<MediaItem>
where
    F: Fn(&str) -> MediaKind,
{
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let Some(ext) = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
            else {
                continue;
            };
            if !allowed_exts.contains(&ext.as_str()) {
                continue;
            }
            let key = normalize_path_key(path);
            if !seen.insert(key) {
                continue;
            }
            let kind = classify(&ext);
            if let Some(item) = build_media_item(path, kind, &ext) {
                items.push(item);
            }
        }
    }

    items
}

pub fn build_media_item(path: &Path, kind: MediaKind, ext: &str) -> Option<MediaItem> {
    let meta = fs::metadata(path).ok()?;
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();
    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("Local")
        .to_string();
    let modified_unix = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let abs_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let abs_str = strip_windows_verbatim_prefix(&abs_path.to_string_lossy());
    let id = format!(
        "local-{:x}",
        fnv1a_hash(abs_str.as_bytes())
    );

    Some(MediaItem {
        id,
        title,
        subtitle: folder,
        stream_url: make_stream_url(Path::new(&abs_str)),
        file_path: abs_str,
        kind,
        extension: ext.to_string(),
        size_bytes: meta.len(),
        modified_unix,
    })
}

pub fn strip_windows_verbatim_prefix(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        path.to_string()
    }
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn default_builtin_scenes() -> Vec<ScenePreset> {
    let mut rainy_ambience = HashMap::new();
    rainy_ambience.insert("rain".to_string(), 0.42);
    rainy_ambience.insert("vinyl".to_string(), 0.18);

    let mut study_ambience = HashMap::new();
    study_ambience.insert("vinyl".to_string(), 0.24);
    study_ambience.insert("fireplace".to_string(), 0.30);

    let mut night_ambience = HashMap::new();
    night_ambience.insert("rain".to_string(), 0.58);
    night_ambience.insert("thunder".to_string(), 0.22);
    night_ambience.insert("wind".to_string(), 0.18);

    vec![
        ScenePreset {
            id: "rainy-window-chill".to_string(),
            name: "Rainy Window Cat".to_string(),
            mood: "CHILLING".to_string(),
            description: "Midnight rain outside an open apartment window with a quiet silhouette companion.".to_string(),
            builtin: true,
            visual: VisualSource {
                kind: "canvas".to_string(),
                id: "rainy-window-cat".to_string(),
                label: "Rainy Window Cat".to_string(),
                stream_url: None,
            },
            visual_fx: VisualFxSettings {
                rain_overlay: 0.55,
                vignette: 0.35,
                film_grain: 0.10,
                dim: 0.04,
                warm_tint: 0.08,
                blur: 0.0,
                fps_cap: 30,
                pause_when_hidden: true,
            },
            eq_preset_id: "flat".to_string(),
            eq_bands: [0.0, 0.0, 0.0, 0.0, 0.0],
            tape_cutoff_hz: 18500.0,
            ambience: HashMap::new(),
            audio_source: AudioSourceRef {
                kind: "station".to_string(),
                id: "focus-zone-relax-mellow".to_string(),
                label: "focus-zone-relax-mellow".to_string(),
            },
            accent_color: "#7dd3fc".to_string(),
        },
        ScenePreset {
            id: "rain-study-session".to_string(),
            name: "Rain & Vinyl Study".to_string(),
            mood: "FOCUSING".to_string(),
            description: "Gentle rain on glass, warm tape EQ, and dusty vinyl crackle for deep focus.".to_string(),
            builtin: true,
            visual: VisualSource {
                kind: "canvas".to_string(),
                id: "rainy-window-cat".to_string(),
                label: "Rainy Window Cat".to_string(),
                stream_url: None,
            },
            visual_fx: VisualFxSettings {
                rain_overlay: 0.72,
                vignette: 0.40,
                film_grain: 0.15,
                dim: 0.10,
                warm_tint: 0.14,
                blur: 0.0,
                fps_cap: 30,
                pause_when_hidden: true,
            },
            eq_preset_id: "warm-lofi".to_string(),
            eq_bands: [3.0, 2.0, -0.5, -2.0, -3.5],
            tape_cutoff_hz: 6800.0,
            ambience: rainy_ambience,
            audio_source: AudioSourceRef {
                kind: "station".to_string(),
                id: "midnight-study-rhodes".to_string(),
                label: "midnight-study-rhodes".to_string(),
            },
            accent_color: "#a78bfa".to_string(),
        },
        ScenePreset {
            id: "neon-rain-skyline".to_string(),
            name: "Neon Rain Skyline".to_string(),
            mood: "DRIFTING".to_string(),
            description: "Distant city towers shimmering through late-night storm clouds and soft thunder.".to_string(),
            builtin: true,
            visual: VisualSource {
                kind: "canvas".to_string(),
                id: "neon-rain-skyline".to_string(),
                label: "Neon Rain Skyline".to_string(),
                stream_url: None,
            },
            visual_fx: VisualFxSettings {
                rain_overlay: 0.65,
                vignette: 0.42,
                film_grain: 0.12,
                dim: 0.08,
                warm_tint: 0.0,
                blur: 0.0,
                fps_cap: 30,
                pause_when_hidden: true,
            },
            eq_preset_id: "late-night".to_string(),
            eq_bands: [4.0, 1.5, -1.5, -3.0, -4.5],
            tape_cutoff_hz: 5200.0,
            ambience: night_ambience,
            audio_source: AudioSourceRef {
                kind: "station".to_string(),
                id: "tokyo-afterhours-dub".to_string(),
                label: "tokyo-afterhours-dub".to_string(),
            },
            accent_color: "#f472b6".to_string(),
        },
        ScenePreset {
            id: "warm-hearth-dusk".to_string(),
            name: "Warm Dusk Hearth".to_string(),
            mood: "UNWINDING".to_string(),
            description: "Golden dusk clouds, hearth crackle, and warm mellow keys for winding down.".to_string(),
            builtin: true,
            visual: VisualSource {
                kind: "canvas".to_string(),
                id: "warm-study-dusk".to_string(),
                label: "Warm Study Dusk".to_string(),
                stream_url: None,
            },
            visual_fx: VisualFxSettings {
                rain_overlay: 0.0,
                vignette: 0.30,
                film_grain: 0.14,
                dim: 0.06,
                warm_tint: 0.25,
                blur: 0.0,
                fps_cap: 30,
                pause_when_hidden: true,
            },
            eq_preset_id: "radio-vinyl".to_string(),
            eq_bands: [-2.0, 2.5, 3.0, -1.0, -5.0],
            tape_cutoff_hz: 4800.0,
            ambience: study_ambience,
            audio_source: AudioSourceRef {
                kind: "station".to_string(),
                id: "sleepy-sunset-tape".to_string(),
                label: "sleepy-sunset-tape".to_string(),
            },
            accent_color: "#fbbf24".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_local_library_allowlist_and_scans_media() {
        let base = std::env::temp_dir().join(format!("lofi_storage_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let storage = LocalStorage::new(base.clone()).unwrap();
        let cfg = storage.load_config();

        let track = storage.music_dir.join("midnight-rain.mp3");
        fs::write(&track, b"fake-mp3-data").unwrap();
        let outside = std::env::temp_dir().join(format!("lofi_outside_{}.mp3", std::process::id()));
        fs::write(&outside, b"outside").unwrap();

        assert!(storage.is_path_allowed(&track, &cfg));
        assert!(!storage.is_path_allowed(&outside, &cfg));

        let snap = storage.scan_library(&cfg);
        assert_eq!(snap.music.len(), 1);
        assert_eq!(snap.music[0].title, "midnight-rain");

        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&base);
    }
}
