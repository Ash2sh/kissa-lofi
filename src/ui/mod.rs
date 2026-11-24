pub mod dock;
pub mod popovers;
pub mod scenes;

use crate::audio::AudioEngine;
use crate::models::{
    builtin_eq_presets, AppConfig, AudioSourceRef, DockModuleSpec, EqPreset, FocusTask,
    HistoryEntry, LibrarySnapshot, MediaItem, MediaKind, PluginHostCommand, PluginPanelView,
    PluginRegistrySnapshot, ScenePreset, VisualSource, AMBIENT_CHANNELS, BUILTIN_STATIONS,
};
use crate::plugins::{PluginEvent, PluginManager};
use crate::storage::{classify_media_path, now_unix, strip_windows_verbatim_prefix, LocalStorage};
use crate::ui::dock::{show_dock, DockViewState};
use crate::ui::popovers::{format_clock, show_active_popover};
use crate::ui::scenes::{SceneRenderer, BUILTIN_CANVAS_SCENES};
use crate::video::BackgroundMedia;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use egui::{
    Align2, Color32, CornerRadius, Frame, Key, Margin, RichText, Stroke, Vec2, ViewportCommand,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivePanel {
    Builtin(String),
    ExternalPlugin {
        plugin_id: String,
        module_id: String,
    },
    PluginManager,
}

pub struct StartupOverrides {
    pub video_path: Option<PathBuf>,
    pub audio_path: Option<PathBuf>,
    pub auto_play: bool,
    pub rain_level: Option<f32>,
    pub apply_wasm_tape: bool,
    pub screenshot_path: Option<PathBuf>,
}

pub struct KissaApp {
    pub storage: LocalStorage,
    pub config: AppConfig,
    pub library: LibrarySnapshot,
    pub scenes: Vec<ScenePreset>,
    pub history: Vec<HistoryEntry>,
    pub plugin_mgr: PluginManager,
    pub plugin_snap: PluginRegistrySnapshot,
    pub audio: AudioEngine,
    pub media: BackgroundMedia,
    pub renderer: SceneRenderer,

    pub active_panel: Option<ActivePanel>,
    pub active_plugin_panel: Option<PluginPanelView>,
    pub hud_hidden: bool,
    pub fullscreen: bool,

    // UI transient inputs & sub-tabs
    pub music_tab: usize,
    pub visuals_tab: usize,
    pub plugin_mgr_tab: usize,
    pub music_search: String,
    pub new_eq_name: String,
    pub new_scene_name: String,
    pub new_scene_mood: String,
    pub new_task_text: String,
    pub share_import_text: String,

    // Timers
    pub focus_running: bool,
    pub focus_is_break: bool,
    pub focus_remaining_sec: u32,
    pub sleep_running: bool,
    pub sleep_remaining_sec: u32,
    last_sec_tick: Instant,
    last_frame: Instant,

    // Toast
    toast_msg: Option<(String, Instant)>,
    pending_visual_init: bool,
    startup_overrides: Option<StartupOverrides>,
    screenshot_target: Option<PathBuf>,
    screenshot_requested: bool,
    frame_count: u32,
}

impl KissaApp {
    pub fn new(cc: &eframe::CreationContext<'_>, overrides: StartupOverrides) -> Self {
        apply_kissa_theme(&cc.egui_ctx);

        let storage = LocalStorage::new(LocalStorage::default_app_dir())
            .expect("Failed initializing Kissa local data directory");
        let config = storage.load_config();
        let scenes = storage.load_scenes();
        let history = storage.load_history();
        let library = storage.scan_library(&config);

        let _ = PluginManager::ensure_starter_wasm_plugins(&storage.plugins_dir);
        let mut plugin_mgr = PluginManager::new(&config.disabled_plugins);
        let mut plugin_dirs = vec![storage.plugins_dir.clone()];
        plugin_dirs.extend(config.custom_plugin_dirs.iter().map(PathBuf::from));
        plugin_mgr.reload(&plugin_dirs);
        let plugin_snap = plugin_mgr.snapshot();

        let mut audio = AudioEngine::new();
        audio.set_volume(config.volume);
        audio.set_muted(config.muted);
        audio.set_eq(config.eq_bands);
        audio.set_tape_cutoff(config.tape_cutoff_hz);
        audio.set_ambience(&config.ambience);

        // Prepare initial audio source
        if config.audio_source.kind == "local" {
            if let Some(track) = library
                .music
                .iter()
                .find(|m| m.id == config.audio_source.id)
            {
                let _ = audio.play_local_file(PathBuf::from(&track.file_path), false);
            } else {
                audio.select_station("focus-zone-relax-mellow", false);
            }
        } else {
            audio.select_station(&config.audio_source.id, false);
        }

        let focus_remaining_sec = config.focus_minutes.max(1) * 60;

        Self {
            storage,
            config,
            library,
            scenes,
            history,
            plugin_mgr,
            plugin_snap,
            audio,
            media: BackgroundMedia::new(),
            renderer: SceneRenderer::new(),
            active_panel: None,
            active_plugin_panel: None,
            hud_hidden: false,
            fullscreen: false,
            music_tab: 0,
            visuals_tab: 0,
            plugin_mgr_tab: 0,
            music_search: String::new(),
            new_eq_name: String::new(),
            new_scene_name: String::new(),
            new_scene_mood: "CUSTOM".to_string(),
            new_task_text: String::new(),
            share_import_text: String::new(),
            focus_running: false,
            focus_is_break: false,
            focus_remaining_sec,
            sleep_running: false,
            sleep_remaining_sec: 0,
            last_sec_tick: Instant::now(),
            last_frame: Instant::now(),
            toast_msg: None,
            pending_visual_init: true,
            screenshot_target: overrides.screenshot_path.clone(),
            screenshot_requested: false,
            frame_count: 0,
            startup_overrides: Some(overrides),
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast_msg = Some((msg.into(), Instant::now()));
    }

    pub fn save_config(&self) {
        let _ = self.storage.save_config(&self.config);
    }

    pub fn close_panel(&mut self) {
        self.active_panel = None;
        self.active_plugin_panel = None;
    }

    pub fn toggle_hud(&mut self) {
        self.hud_hidden = !self.hud_hidden;
        if self.hud_hidden {
            self.close_panel();
        }
    }

    pub fn toggle_plugin_manager(&mut self) {
        if self.active_panel == Some(ActivePanel::PluginManager) {
            self.close_panel();
        } else {
            self.hud_hidden = false;
            self.active_panel = Some(ActivePanel::PluginManager);
            self.active_plugin_panel = None;
        }
    }

    pub fn open_external_plugin(&mut self, plugin_id: &str, module_id: &str) {
        self.hud_hidden = false;
        self.active_plugin_panel = self
            .plugin_mgr
            .render_plugin_panel(plugin_id)
            .ok()
            .flatten();
        self.active_panel = Some(ActivePanel::ExternalPlugin {
            plugin_id: plugin_id.to_string(),
            module_id: module_id.to_string(),
        });
    }

    fn toggle_dock_module(&mut self, module: &DockModuleSpec) {
        let same = match &self.active_panel {
            Some(ActivePanel::Builtin(id)) => id == &module.id,
            Some(ActivePanel::ExternalPlugin { module_id, .. }) => module_id == &module.id,
            _ => false,
        };
        if same {
            self.close_panel();
            return;
        }
        self.hud_hidden = false;
        if module.panel_kind == "builtin" {
            self.active_plugin_panel = None;
            self.active_panel = Some(ActivePanel::Builtin(module.id.clone()));
        } else {
            self.open_external_plugin(&module.plugin_id, &module.id);
        }
    }

    pub fn apply_eq_preset(&mut self, preset: EqPreset) {
        self.config.eq_preset_id = preset.id;
        self.config.eq_bands = preset.bands;
        self.config.tape_cutoff_hz = preset.tape_cutoff_hz;
        self.audio.set_eq(self.config.eq_bands);
        self.audio.set_tape_cutoff(self.config.tape_cutoff_hz);
        self.save_config();
        self.toast(format!("EQ preset: {}", preset.name));
    }

    pub fn reset_eq(&mut self) {
        self.config.eq_preset_id = "flat".to_string();
        self.config.eq_bands = [0.0; 5];
        self.config.tape_cutoff_hz = 18500.0;
        self.audio.set_eq([0.0; 5]);
        self.audio.set_tape_cutoff(18500.0);
        self.save_config();
    }

    pub fn save_custom_eq_preset(&mut self, name: String) {
        let preset = EqPreset {
            id: format!("custom-{}", now_unix()),
            name: name.clone(),
            bands: self.config.eq_bands,
            tape_cutoff_hz: self.config.tape_cutoff_hz,
            source: "user".to_string(),
        };
        self.config.eq_preset_id = preset.id.clone();
        self.config.custom_eq_presets.push(preset);
        self.save_config();
        self.toast(format!("Saved EQ preset \"{name}\""));
    }

    pub fn run_wasm_eq_transform(&mut self, plugin_id: &str, intensity: f32) {
        match self
            .plugin_mgr
            .apply_eq_transform(plugin_id, self.config.eq_bands, intensity)
        {
            Ok(res) => {
                self.config.eq_bands = res.output_bands;
                self.config.tape_cutoff_hz = res.suggested_tape_cutoff_hz;
                self.config.eq_preset_id = format!("dsp:{plugin_id}");
                self.audio.set_eq(self.config.eq_bands);
                self.audio.set_tape_cutoff(self.config.tape_cutoff_hz);
                self.save_config();
                self.toast(format!("{}: {}", res.plugin_name, res.note));
            }
            Err(e) => self.toast(e.to_string()),
        }
    }

    pub fn invoke_plugin_action(&mut self, plugin_id: &str, action_id: &str, value: f32) {
        match self.plugin_mgr.invoke_plugin_action(
            plugin_id,
            action_id,
            value,
            self.config.eq_bands,
        ) {
            Ok(res) => {
                if let Some(panel) = res.panel {
                    self.active_plugin_panel = Some(panel);
                }
                for cmd in res.commands {
                    match cmd {
                        PluginHostCommand::SetEq {
                            bands,
                            tape_cutoff_hz,
                            label,
                        } => {
                            self.config.eq_bands = bands;
                            self.config.tape_cutoff_hz = tape_cutoff_hz;
                            self.config.eq_preset_id = if label == "Flat EQ" {
                                "flat".to_string()
                            } else {
                                format!("dsp:{plugin_id}")
                            };
                            self.audio.set_eq(bands);
                            self.audio.set_tape_cutoff(tape_cutoff_hz);
                            self.save_config();
                        }
                        PluginHostCommand::SetAmbience { channel, level } => {
                            self.set_ambience_level(&channel, level);
                        }
                        PluginHostCommand::SetVisual { visual_id } => {
                            if let Some(sc) =
                                BUILTIN_CANVAS_SCENES.iter().find(|s| s.id == visual_id)
                            {
                                self.select_canvas_scene(sc.id, sc.name);
                            }
                        }
                        PluginHostCommand::Toast { message } => self.toast(message),
                    }
                }
            }
            Err(e) => self.toast(e.to_string()),
        }
    }

    pub fn select_station(&mut self, station_id: &str, record_history: bool) {
        let st = BUILTIN_STATIONS
            .iter()
            .find(|s| s.id == station_id)
            .unwrap_or(&BUILTIN_STATIONS[0]);
        self.audio.select_station(st.id, true);
        self.config.audio_source = AudioSourceRef {
            kind: "station".to_string(),
            id: st.id.to_string(),
            label: st.name.to_string(),
        };
        self.save_config();
        if record_history {
            self.append_history(st.name, st.mood, "station", st.id);
        }
    }

    pub fn play_local_track(&mut self, item: &MediaItem, record_history: bool) {
        match self
            .audio
            .play_local_file(PathBuf::from(&item.file_path), true)
        {
            Ok(()) => {
                self.config.audio_source = AudioSourceRef {
                    kind: "local".to_string(),
                    id: item.id.clone(),
                    label: item.title.clone(),
                };
                self.save_config();
                if record_history {
                    self.append_history(&item.title, &item.subtitle, "track", &item.id);
                }
            }
            Err(e) => self.toast(e),
        }
    }

    pub fn step_track(&mut self, delta: isize) {
        if self.config.audio_source.kind == "local" && !self.library.music.is_empty() {
            let len = self.library.music.len();
            let cur = self
                .library
                .music
                .iter()
                .position(|m| m.id == self.config.audio_source.id)
                .unwrap_or(0);
            let next_idx = if self.config.shuffle && len > 1 {
                (now_unix() as usize + cur + 1) % len
            } else {
                (cur as isize + delta).rem_euclid(len as isize) as usize
            };
            let item = self.library.music[next_idx].clone();
            self.play_local_track(&item, true);
            return;
        }
        let len = BUILTIN_STATIONS.len();
        let cur = BUILTIN_STATIONS
            .iter()
            .position(|s| s.id == self.config.audio_source.id)
            .unwrap_or(0);
        let next = (cur as isize + delta).rem_euclid(len as isize) as usize;
        self.select_station(BUILTIN_STATIONS[next].id, true);
    }

    pub fn cycle_repeat(&mut self) {
        self.config.repeat_mode = match self.config.repeat_mode.as_str() {
            "all" => "one",
            "one" => "off",
            _ => "all",
        }
        .to_string();
        self.save_config();
    }

    pub fn select_canvas_scene(&mut self, id: &str, name: &str) {
        self.media.clear();
        self.config.visual = VisualSource {
            kind: "canvas".to_string(),
            id: id.to_string(),
            label: name.to_string(),
        };
        self.save_config();
        self.toast(format!("Background: {name}"));
    }

    pub fn select_local_visual(&mut self, item: &MediaItem) {
        self.config.visual = VisualSource {
            kind: if item.kind == MediaKind::Video {
                "video".to_string()
            } else {
                "image".to_string()
            },
            id: item.file_path.clone(),
            label: item.title.clone(),
        };
        self.pending_visual_init = true;
        self.save_config();
        self.toast(format!("Background: {}", item.title));
    }

    fn sync_visual_media(&mut self, ctx: &egui::Context) {
        if !self.pending_visual_init {
            return;
        }
        self.pending_visual_init = false;
        let path = PathBuf::from(&self.config.visual.id);
        match self.config.visual.kind.as_str() {
            "video" => {
                if let Err(e) = self.media.load_video(&path, self.config.visual_fx.fps_cap) {
                    self.toast(e);
                }
            }
            "image" => {
                if let Err(e) = self.media.load_image(ctx, &path) {
                    self.toast(e);
                }
            }
            _ => self.media.clear(),
        }
    }

    pub fn set_ambience_level(&mut self, id: &str, level: f32) {
        let clamped = level.clamp(0.0, 1.0);
        if clamped <= 0.005 {
            self.config.ambience.remove(id);
        } else {
            self.config.ambience.insert(id.to_string(), clamped);
        }
        self.audio.set_ambience(&self.config.ambience);
        self.save_config();
    }

    pub fn clear_ambience(&mut self) {
        self.config.ambience.clear();
        self.audio.set_ambience(&self.config.ambience);
        self.save_config();
    }

    pub fn apply_scene(&mut self, scene: ScenePreset) {
        self.config.active_scene_id = scene.id.clone();
        self.config.visual = scene.visual.clone();
        self.config.visual_fx = scene.visual_fx.clone();
        self.config.eq_preset_id = scene.eq_preset_id.clone();
        self.config.eq_bands = scene.eq_bands;
        self.config.tape_cutoff_hz = scene.tape_cutoff_hz;
        self.config.ambience = scene.ambience.clone();
        self.config.audio_source = scene.audio_source.clone();

        self.pending_visual_init = true;
        self.audio.set_eq(self.config.eq_bands);
        self.audio.set_tape_cutoff(self.config.tape_cutoff_hz);
        self.audio.set_ambience(&self.config.ambience);
        if scene.audio_source.kind == "station" {
            self.audio
                .select_station(&scene.audio_source.id, self.audio.is_playing());
        }
        self.save_config();
        self.append_history(&scene.name, &scene.mood, "scene", &scene.id);
        self.toast(format!("Scene: {}", scene.name));
    }

    pub fn save_current_scene(&mut self, name: String, mood: String) {
        let sc = ScenePreset {
            id: format!("user-scene-{}", now_unix()),
            name: name.clone(),
            mood,
            description: format!(
                "Custom mix: {} + {}",
                self.config.visual.label,
                self.current_eq_name()
            ),
            builtin: false,
            visual: self.config.visual.clone(),
            visual_fx: self.config.visual_fx.clone(),
            eq_preset_id: self.config.eq_preset_id.clone(),
            eq_bands: self.config.eq_bands,
            tape_cutoff_hz: self.config.tape_cutoff_hz,
            ambience: self.config.ambience.clone(),
            audio_source: self.config.audio_source.clone(),
            accent_rgb: [125, 211, 252],
        };
        self.config.active_scene_id = sc.id.clone();
        self.scenes.push(sc);
        let _ = self.storage.save_user_scenes(&self.scenes);
        self.save_config();
        self.toast(format!("Saved scene \"{name}\""));
    }

    pub fn delete_scene(&mut self, id: &str) {
        self.scenes.retain(|s| s.builtin || s.id != id);
        let _ = self.storage.save_user_scenes(&self.scenes);
        self.toast("Deleted scene");
    }

    pub fn toggle_plugin(&mut self, plugin_id: &str, enabled: bool) {
        if let Err(e) = self.plugin_mgr.set_enabled(plugin_id, enabled) {
            self.toast(e.to_string());
            return;
        }
        self.config.disabled_plugins = self.plugin_mgr.disabled_ids();
        self.plugin_snap = self.plugin_mgr.snapshot();
        self.save_config();
        if !enabled {
            if let Some(ActivePanel::ExternalPlugin {
                plugin_id: active_id,
                ..
            }) = &self.active_panel
            {
                if active_id == plugin_id {
                    self.close_panel();
                }
            }
        }
    }

    pub fn reload_plugins(&mut self) {
        let mut dirs = vec![self.storage.plugins_dir.clone()];
        dirs.extend(self.config.custom_plugin_dirs.iter().map(PathBuf::from));
        self.plugin_mgr.reload(&dirs);
        self.plugin_snap = self.plugin_mgr.snapshot();
        self.toast(format!(
            "Loaded {} plugins ({} on dock)",
            self.plugin_snap.plugins.len(),
            self.plugin_snap.dock_modules.len()
        ));
    }

    pub fn pick_and_install_wasm_plugin(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Install WebAssembly Dock Plugin")
            .add_filter("WebAssembly Plugin", &["wasm", "wat"])
            .pick_file()
        {
            self.install_plugin_file(&path);
        }
    }

    fn install_plugin_file(&mut self, src: &Path) {
        if let Some(name) = src.file_name() {
            let dest = self.storage.plugins_dir.join(name);
            if let Err(e) = fs::copy(src, &dest) {
                self.toast(format!("Install failed: {e}"));
                return;
            }
            self.reload_plugins();
            self.toast("Installed Wasm plugin onto dock");
        }
    }

    pub fn pick_and_add_directory(&mut self, category: &str) {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title(format!("Select local {category} folder"))
            .pick_folder()
        {
            let s = strip_windows_verbatim_prefix(&dir.to_string_lossy());
            let target = match category {
                "music" => &mut self.config.custom_music_dirs,
                "visuals" => &mut self.config.custom_visual_dirs,
                "sounds" => &mut self.config.custom_sound_dirs,
                "plugins" => &mut self.config.custom_plugin_dirs,
                _ => return,
            };
            if !target.iter().any(|x| x.eq_ignore_ascii_case(&s)) {
                target.push(s);
            }
            self.save_config();
            if category == "plugins" {
                self.reload_plugins();
            } else {
                self.rescan_library();
            }
        }
    }

    pub fn pick_local_files(&mut self, category: &str) {
        let mut dlg = rfd::FileDialog::new().set_title(format!("Select {category} files"));
        dlg = if category == "visuals" {
            dlg.add_filter(
                "Photos & Videos",
                &[
                    "mp4", "webm", "mov", "mkv", "jpg", "jpeg", "png", "webp", "gif",
                ],
            )
        } else {
            dlg.add_filter("Audio", &["mp3", "flac", "wav", "ogg", "m4a", "aac"])
        };
        if let Some(files) = dlg.pick_files() {
            for p in files {
                self.register_dropped_or_picked_path(&p);
            }
        }
    }

    fn register_dropped_or_picked_path(&mut self, path: &Path) {
        if let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
        {
            if ext == "wasm" || ext == "wat" {
                self.install_plugin_file(path);
                return;
            }
        }
        let Some((kind, ext)) = classify_media_path(path) else {
            self.toast("Unsupported file type");
            return;
        };
        let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let abs_str = strip_windows_verbatim_prefix(&abs.to_string_lossy());
        if !self
            .config
            .extra_files
            .iter()
            .any(|f| f.eq_ignore_ascii_case(&abs_str))
        {
            self.config.extra_files.push(abs_str.clone());
            self.save_config();
        }
        self.library = self.storage.scan_library(&self.config);
        if let Some(item) =
            crate::storage::build_media_item(Path::new(&abs_str), kind.clone(), &ext)
        {
            match kind {
                MediaKind::Video | MediaKind::Image => self.select_local_visual(&item),
                MediaKind::Audio | MediaKind::Ambient => self.play_local_track(&item, true),
            }
        }
    }

    pub fn rescan_library(&mut self) {
        self.library = self.storage.scan_library(&self.config);
        self.toast(format!(
            "Indexed {} tracks & {} visuals",
            self.library.music.len(),
            self.library.visuals.len()
        ));
    }

    pub fn append_history(&mut self, title: &str, subtitle: &str, kind: &str, ref_id: &str) {
        let entry = HistoryEntry {
            id: format!("hist-{}-{}", now_unix(), self.history.len()),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            kind: kind.to_string(),
            ref_id: ref_id.to_string(),
            timestamp_unix: now_unix(),
        };
        if self
            .history
            .first()
            .map(|e| (e.ref_id.as_str(), e.kind.as_str()))
            == Some((ref_id, kind))
        {
            self.history[0] = entry;
        } else {
            self.history.insert(0, entry);
            self.history.truncate(80);
        }
        let _ = self.storage.save_history(&self.history);
        if kind == "scene" {
            self.plugin_mgr
                .dispatch_event(&PluginEvent::SceneChanged { scene_index: 1 });
        } else {
            self.plugin_mgr.dispatch_event(&PluginEvent::TrackChanged {
                bpm_hint: 76,
                mood_code: 1,
            });
        }
    }

    pub fn replay_history(&mut self, entry: &HistoryEntry) {
        match entry.kind.as_str() {
            "station" => self.select_station(&entry.ref_id, false),
            "track" => {
                if let Some(item) = self
                    .library
                    .music
                    .iter()
                    .find(|m| m.id == entry.ref_id)
                    .cloned()
                {
                    self.play_local_track(&item, false);
                }
            }
            "scene" => {
                if let Some(sc) = self.scenes.iter().find(|s| s.id == entry.ref_id).cloned() {
                    self.apply_scene(sc);
                }
            }
            _ => {}
        }
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        let _ = self.storage.save_history(&self.history);
        self.toast("History cleared");
    }

    pub fn import_share_code(&mut self, raw: &str) {
        let json_str = if let Some(b64) = raw.strip_prefix("lofi-scene-v1:") {
            match B64.decode(b64.trim()) {
                Ok(bytes) => String::from_utf8(bytes).unwrap_or_default(),
                Err(_) => {
                    self.toast("Invalid scene code");
                    return;
                }
            }
        } else {
            raw.to_string()
        };

        let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) else {
            self.toast("Invalid scene JSON");
            return;
        };
        if let Some(v) = val
            .get("visual")
            .and_then(|x| serde_json::from_value(x.clone()).ok())
        {
            self.config.visual = v;
            self.pending_visual_init = true;
        }
        if let Some(fx) = val
            .get("visualFx")
            .and_then(|x| serde_json::from_value(x.clone()).ok())
        {
            self.config.visual_fx = fx;
        }
        if let Some(b) = val
            .get("eqBands")
            .and_then(|x| serde_json::from_value(x.clone()).ok())
        {
            self.config.eq_bands = b;
            self.audio.set_eq(b);
        }
        if let Some(hz) = val.get("tapeCutoffHz").and_then(|x| x.as_f64()) {
            self.config.tape_cutoff_hz = hz as f32;
            self.audio.set_tape_cutoff(hz as f32);
        }
        if let Some(amb) = val
            .get("ambience")
            .and_then(|x| serde_json::from_value(x.clone()).ok())
        {
            self.config.ambience = amb;
            self.audio.set_ambience(&self.config.ambience);
        }
        self.save_config();
        self.toast("Imported atmosphere preset");
    }

    pub fn add_focus_task(&mut self, text: String) {
        self.config.focus_tasks.insert(
            0,
            FocusTask {
                id: format!("task-{}", now_unix()),
                text,
                done: false,
            },
        );
        self.save_config();
    }

    pub fn toggle_focus_task(&mut self, id: &str) {
        if let Some(t) = self.config.focus_tasks.iter_mut().find(|t| t.id == id) {
            t.done = !t.done;
            self.save_config();
        }
    }

    pub fn delete_focus_task(&mut self, id: &str) {
        self.config.focus_tasks.retain(|t| t.id != id);
        self.save_config();
    }

    pub fn start_sleep(&mut self, minutes: u32) {
        self.sleep_running = true;
        self.sleep_remaining_sec = minutes * 60;
        self.audio.set_sleep_fade(1.0);
        self.toast(format!("Sleep timer set for {minutes} min"));
    }

    pub fn cancel_sleep(&mut self) {
        self.sleep_running = false;
        self.sleep_remaining_sec = 0;
        self.audio.set_sleep_fade(1.0);
        self.toast("Sleep timer cancelled");
    }

    fn current_track_display(&self) -> (String, String, String) {
        if self.config.audio_source.kind == "local" {
            let title = self
                .library
                .music
                .iter()
                .find(|m| m.id == self.config.audio_source.id)
                .map(|m| m.title.clone())
                .unwrap_or_else(|| self.config.audio_source.label.clone());
            return (
                "LOCAL LIBRARY".to_string(),
                title,
                format!("{} local tracks", self.library.music.len()),
            );
        }
        let st = BUILTIN_STATIONS
            .iter()
            .find(|s| s.id == self.config.audio_source.id)
            .unwrap_or(&BUILTIN_STATIONS[0]);
        (
            st.mood.to_string(),
            st.name.to_string(),
            format!("{} listening", st.listeners),
        )
    }

    fn current_eq_name(&self) -> String {
        let mut all = builtin_eq_presets();
        all.extend(self.plugin_snap.eq_presets.clone());
        all.extend(self.config.custom_eq_presets.clone());
        if let Some(p) = all.iter().find(|p| p.id == self.config.eq_preset_id) {
            return p.name.clone();
        }
        if let Some(pid) = self.config.eq_preset_id.strip_prefix("dsp:") {
            if let Some(pl) = self.plugin_snap.plugins.iter().find(|p| p.id == pid) {
                return format!("{} EQ", pl.name);
            }
        }
        "Custom EQ".to_string()
    }

    fn status_line(&self) -> String {
        let eq = self.current_eq_name();
        let active_amb: Vec<&str> = AMBIENT_CHANNELS
            .iter()
            .filter(|c| self.config.ambience.get(c.id).copied().unwrap_or(0.0) > 0.01)
            .map(|c| c.name)
            .collect();
        let amb = match active_amb.len() {
            0 => "No ambience".to_string(),
            1 => active_amb[0].to_string(),
            2 => format!("{}, {}", active_amb[0], active_amb[1]),
            n => format!("{} +{}", active_amb[0], n - 1),
        };
        format!("{eq} · {amb}")
    }

    fn handle_startup_overrides(&mut self) {
        let Some(ov) = self.startup_overrides.take() else {
            return;
        };
        if let Some(vid) = ov.video_path {
            if vid.is_file() {
                let title = vid
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("video")
                    .to_string();
                self.config.visual = VisualSource {
                    kind: "video".to_string(),
                    id: vid.to_string_lossy().to_string(),
                    label: title,
                };
                self.pending_visual_init = true;
            }
        }
        if let Some(aud) = ov.audio_path {
            if aud.is_file() {
                let title = aud
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("track")
                    .to_string();
                if self
                    .audio
                    .play_local_file(aud.clone(), ov.auto_play)
                    .is_ok()
                {
                    self.config.audio_source = AudioSourceRef {
                        kind: "local".to_string(),
                        id: "cli-audio".to_string(),
                        label: title,
                    };
                }
            }
        } else if ov.auto_play {
            self.audio.set_playing(true);
        }
        if let Some(r) = ov.rain_level {
            self.set_ambience_level("rain", r);
        }
        if ov.apply_wasm_tape {
            self.run_wasm_eq_transform("wasm-tape-lab", 0.75);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.close_panel();
            }
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(Key::Space) {
                self.audio.toggle_play();
            } else if i.key_pressed(Key::Escape) {
                if self.active_panel.is_some() {
                    self.close_panel();
                } else if self.hud_hidden {
                    self.hud_hidden = false;
                }
            } else if i.key_pressed(Key::M) {
                self.config.muted = !self.config.muted;
                self.audio.set_muted(self.config.muted);
                self.save_config();
            } else if i.key_pressed(Key::G) {
                self.toggle_plugin_manager();
            } else if i.key_pressed(Key::H) {
                self.toggle_hud();
            } else if i.key_pressed(Key::F) {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            } else if i.key_pressed(Key::ArrowUp) {
                self.config.volume = (self.config.volume + 0.05).min(1.0);
                self.audio.set_volume(self.config.volume);
                self.save_config();
            } else if i.key_pressed(Key::ArrowDown) {
                self.config.volume = (self.config.volume - 0.05).max(0.0);
                self.audio.set_volume(self.config.volume);
                self.save_config();
            } else if i.key_pressed(Key::N) {
                self.step_track(1);
            } else if i.key_pressed(Key::P) {
                self.step_track(-1);
            } else {
                let digit_keys = [
                    (Key::Num1, "1"),
                    (Key::Num2, "2"),
                    (Key::Num3, "3"),
                    (Key::Num4, "4"),
                    (Key::Num5, "5"),
                    (Key::Num6, "6"),
                    (Key::Num7, "7"),
                    (Key::Num8, "8"),
                    (Key::Num9, "9"),
                    (Key::Num0, "0"),
                ];
                for (key, s) in digit_keys {
                    if i.key_pressed(key) {
                        if let Some(m) = self
                            .plugin_snap
                            .dock_modules
                            .iter()
                            .find(|m| m.shortcut.as_deref() == Some(s))
                            .cloned()
                        {
                            self.toggle_dock_module(&m);
                        }
                    }
                }
            }
        });
    }

    fn tick_timers(&mut self) {
        if self.audio.local_finished() {
            if self.config.repeat_mode == "one" {
                self.audio.seek_local(0.0);
            } else if self.config.repeat_mode == "all" {
                self.step_track(1);
            } else {
                self.audio.set_playing(false);
            }
        }

        if self.last_sec_tick.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_sec_tick = Instant::now();

        if self.focus_running {
            self.focus_remaining_sec = self.focus_remaining_sec.saturating_sub(1);
            if self.focus_remaining_sec == 0 {
                self.focus_running = false;
                self.audio.play_chime();
                if !self.focus_is_break {
                    self.config.completed_focus_sessions += 1;
                    self.focus_is_break = true;
                    self.focus_remaining_sec = self.config.break_minutes.max(1) * 60;
                    self.save_config();
                    self.plugin_mgr
                        .dispatch_event(&PluginEvent::FocusSessionComplete {
                            total_sessions: self.config.completed_focus_sessions as i32,
                        });
                    self.toast("Focus session complete! Time for a short break.");
                } else {
                    self.focus_is_break = false;
                    self.focus_remaining_sec = self.config.focus_minutes.max(1) * 60;
                    self.toast("Break complete! Ready for the next focus block.");
                }
            }
        }

        if self.sleep_running {
            self.sleep_remaining_sec = self.sleep_remaining_sec.saturating_sub(1);
            if self.sleep_remaining_sec <= 30 {
                self.audio
                    .set_sleep_fade(self.sleep_remaining_sec as f32 / 30.0);
            }
            if self.sleep_remaining_sec == 0 {
                self.sleep_running = false;
                self.audio.set_playing(false);
                self.audio.set_sleep_fade(1.0);
                self.toast("Sleep timer finished — playback paused");
            }
        }
    }
}

impl eframe::App for KissaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        self.handle_startup_overrides();
        self.sync_visual_media(ctx);
        self.media.poll_video_frame(ctx);
        self.tick_timers();
        self.handle_shortcuts(ctx);

        // Handle OS drag-and-drop onto window
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            self.register_dropped_or_picked_path(&path);
        }

        // Full-bleed atmospheric background stage
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(9, 11, 18)))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                self.renderer.paint(
                    ui.painter(),
                    rect,
                    dt,
                    &self.config.visual,
                    &self.config.visual_fx,
                    &self.media,
                );
            });

        // Subtle top HUD pills
        if !self.hud_hidden {
            // Left active timer pills
            if self.focus_running || self.sleep_running {
                egui::Area::new(egui::Id::new("kissa_top_left"))
                    .anchor(Align2::LEFT_TOP, Vec2::new(16.0, 14.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if self.focus_running
                                && hud_pill_btn(
                                    ui,
                                    &format!(
                                        "● {} {}",
                                        if self.focus_is_break {
                                            "Break"
                                        } else {
                                            "Focus"
                                        },
                                        format_clock(self.focus_remaining_sec)
                                    ),
                                    Color32::from_rgb(56, 189, 248),
                                )
                            {
                                self.active_panel = Some(ActivePanel::Builtin("focus".into()));
                            }
                            if self.sleep_running
                                && hud_pill_btn(
                                    ui,
                                    &format!("● Sleep {}", format_clock(self.sleep_remaining_sec)),
                                    Color32::from_rgb(167, 139, 250),
                                )
                            {
                                self.active_panel = Some(ActivePanel::Builtin("sleep".into()));
                            }
                        });
                    });
            }

            // Right utility pills: Plugins (G), Hide HUD (H), Fullscreen (F)
            egui::Area::new(egui::Id::new("kissa_top_right"))
                .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 14.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if hud_pill_btn(
                            ui,
                            &format!("Plugins ({})", self.plugin_snap.dock_modules.len()),
                            Color32::from_rgb(200, 210, 228),
                        ) {
                            self.toggle_plugin_manager();
                        }
                        if hud_pill_btn(ui, "Cinema", Color32::from_rgb(175, 185, 205)) {
                            self.toggle_hud();
                        }
                        if hud_pill_btn(ui, "⛶", Color32::from_rgb(175, 185, 205)) {
                            self.fullscreen = !self.fullscreen;
                            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
                        }
                    });
                });

            // Floating popover
            show_active_popover(ctx, self);

            // Bottom frosted-glass control dock
            let (mood, title, subtitle) = self.current_track_display();
            let status_line = self.status_line();
            let active_mod_id = match &self.active_panel {
                Some(ActivePanel::Builtin(id)) => Some(id.as_str()),
                Some(ActivePanel::ExternalPlugin { module_id, .. }) => Some(module_id.as_str()),
                _ => None,
            };
            let dock_events = show_dock(
                ctx,
                &DockViewState {
                    mood: &mood,
                    title: &title,
                    subtitle: &subtitle,
                    status_line: &status_line,
                    playing: self.audio.is_playing(),
                    volume: self.config.volume,
                    muted: self.config.muted,
                    modules: &self.plugin_snap.dock_modules,
                    active_module_id: active_mod_id,
                    scenes_dot: !self.config.active_scene_id.is_empty(),
                    sounds_active: self.config.ambience.values().any(|&v| v > 0.01),
                    focus_active: self.focus_running,
                    sleep_active: self.sleep_running,
                },
            );

            if dock_events.toggle_play {
                self.audio.toggle_play();
            }
            if dock_events.toggle_mute {
                self.config.muted = !self.config.muted;
                self.audio.set_muted(self.config.muted);
                self.save_config();
            }
            if let Some(v) = dock_events.new_volume {
                self.config.volume = v;
                if v > 0.0 && self.config.muted {
                    self.config.muted = false;
                    self.audio.set_muted(false);
                }
                self.audio.set_volume(v);
                self.save_config();
            }
            if let Some(m) = dock_events.clicked_module {
                self.toggle_dock_module(&m);
            }
            if dock_events.track_clicked {
                if let Some(m) = self
                    .plugin_snap
                    .dock_modules
                    .iter()
                    .find(|m| m.id == "music")
                    .cloned()
                {
                    self.toggle_dock_module(&m);
                }
            }
        } else {
            egui::Area::new(egui::Id::new("kissa_restore_hud"))
                .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -14.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    if hud_pill_btn(ui, "Show Controls (H)", Color32::from_rgb(190, 200, 220)) {
                        self.hud_hidden = false;
                    }
                });
        }

        // Toast notification
        if let Some((msg, at)) = &self.toast_msg {
            if at.elapsed() < Duration::from_secs(3) {
                egui::Area::new(egui::Id::new("kissa_toast"))
                    .anchor(Align2::RIGHT_TOP, Vec2::new(-18.0, 56.0))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(15, 18, 28, 240))
                            .stroke(Stroke::new(
                                1.0,
                                Color32::from_rgba_unmultiplied(255, 255, 255, 32),
                            ))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.label(RichText::new(msg).size(12.0).color(Color32::WHITE));
                            });
                    });
            } else {
                self.toast_msg = None;
            }
        }

        self.frame_count = self.frame_count.saturating_add(1);

        // Handle automated native screenshot capture (--screenshot <path>)
        if let Some(out_path) = self.screenshot_target.clone() {
            let captured = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(img) = captured {
                let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let _ = image::save_buffer(
                    &out_path,
                    &raw,
                    img.width() as u32,
                    img.height() as u32,
                    image::ColorType::Rgba8,
                );
                self.screenshot_target = None;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            } else if !self.screenshot_requested && self.frame_count >= 18 {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }

        // Adaptive FPS cap (ultra-low CPU/GPU footprint)
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if minimized && self.config.visual_fx.pause_when_hidden {
            ctx.request_repaint_after(Duration::from_millis(500));
        } else {
            let fps = self.config.visual_fx.fps_cap.clamp(12, 60);
            ctx.request_repaint_after(Duration::from_millis((1000 / fps) as u64));
        }
    }
}

fn hud_pill_btn(ui: &mut egui::Ui, label: &str, color: Color32) -> bool {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(10.0, 4.0);
        let btn = egui::Button::new(RichText::new(label).size(11.5).color(color))
            .fill(Color32::from_rgba_unmultiplied(12, 13, 20, 185))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 26),
            ))
            .corner_radius(CornerRadius::same(99));
        ui.add(btn).clicked()
    })
    .inner
}

fn apply_kissa_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgb(14, 15, 22);
    visuals.panel_fill = Color32::from_rgb(9, 11, 18);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
    visuals.widgets.inactive.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 14);
    visuals.widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 34);
    visuals.widgets.active.bg_fill = Color32::from_rgba_unmultiplied(125, 211, 252, 68);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(56, 189, 248, 65);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(125, 211, 252));
    ctx.set_visuals(visuals);
}
