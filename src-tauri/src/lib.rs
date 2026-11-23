mod media_protocol;
mod models;
mod plugins;
mod storage;

use crate::models::{
    AppConfig, HistoryEntry, LibrarySnapshot, MediaItem, MediaKind, PluginEqTransformResult,
    PluginRegistrySnapshot, ScenePreset, SystemMetrics,
};
use crate::plugins::{PluginEvent, PluginManager};
use crate::storage::{build_media_item, now_unix, strip_windows_verbatim_prefix, LocalStorage};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State};

struct AppState {
    storage: LocalStorage,
    config: Mutex<AppConfig>,
    scenes: Mutex<Vec<ScenePreset>>,
    history: Mutex<Vec<HistoryEntry>>,
    plugins: Mutex<PluginManager>,
    started_at: Instant,
}

impl AppState {
    fn plugin_dirs(&self, cfg: &AppConfig) -> Vec<PathBuf> {
        let mut dirs = vec![self.storage.plugins_dir.clone()];
        dirs.extend(cfg.custom_plugin_dirs.iter().map(PathBuf::from));
        dirs
    }
}

#[tauri::command]
fn get_config(state: State<'_, Arc<AppState>>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(config: AppConfig, state: State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    state.storage.save_config(&config)?;
    *state.config.lock().unwrap() = config.clone();
    Ok(config)
}

#[tauri::command]
fn scan_library(state: State<'_, Arc<AppState>>) -> LibrarySnapshot {
    let cfg = state.config.lock().unwrap().clone();
    state.storage.scan_library(&cfg)
}

#[tauri::command]
fn add_library_directory(
    category: String,
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<LibrarySnapshot, String> {
    let cleaned = strip_windows_verbatim_prefix(path.trim());
    let dir = PathBuf::from(&cleaned);
    if !dir.is_dir() {
        return Err(format!("Directory does not exist: {cleaned}"));
    }

    let mut cfg = state.config.lock().unwrap();
    let target = match category.as_str() {
        "music" => &mut cfg.custom_music_dirs,
        "visuals" => &mut cfg.custom_visual_dirs,
        "sounds" => &mut cfg.custom_sound_dirs,
        "plugins" => &mut cfg.custom_plugin_dirs,
        _ => return Err(format!("Unknown directory category: {category}")),
    };

    if !target.iter().any(|existing| existing.eq_ignore_ascii_case(&cleaned)) {
        target.push(cleaned);
    }

    state.storage.save_config(&cfg)?;
    let cfg_clone = cfg.clone();
    drop(cfg);

    if category == "plugins" {
        let dirs = state.plugin_dirs(&cfg_clone);
        state.plugins.lock().unwrap().reload(&dirs);
    }

    Ok(state.storage.scan_library(&cfg_clone))
}

#[tauri::command]
fn remove_library_directory(
    category: String,
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<LibrarySnapshot, String> {
    let mut cfg = state.config.lock().unwrap();
    let target = match category.as_str() {
        "music" => &mut cfg.custom_music_dirs,
        "visuals" => &mut cfg.custom_visual_dirs,
        "sounds" => &mut cfg.custom_sound_dirs,
        "plugins" => &mut cfg.custom_plugin_dirs,
        _ => return Err(format!("Unknown directory category: {category}")),
    };

    target.retain(|existing| !existing.eq_ignore_ascii_case(&path));
    state.storage.save_config(&cfg)?;
    let cfg_clone = cfg.clone();
    drop(cfg);

    if category == "plugins" {
        let dirs = state.plugin_dirs(&cfg_clone);
        state.plugins.lock().unwrap().reload(&dirs);
    }

    Ok(state.storage.scan_library(&cfg_clone))
}

#[tauri::command]
fn register_local_file(
    file_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<MediaItem, String> {
    let cleaned = strip_windows_verbatim_prefix(file_path.trim());
    let path = PathBuf::from(&cleaned);
    if !path.is_file() {
        return Err(format!("File not found: {cleaned}"));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| "File has no extension".to_string())?;

    let kind = match ext.as_str() {
        "mp4" | "webm" | "mov" | "m4v" | "ogv" => MediaKind::Video,
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif" => MediaKind::Image,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" => MediaKind::Audio,
        _ => return Err(format!("Unsupported media extension: .{ext}")),
    };

    let item = build_media_item(&path, kind, &ext)
        .ok_or_else(|| "Failed to read media file metadata".to_string())?;

    let mut cfg = state.config.lock().unwrap();
    if !cfg
        .allowed_files
        .iter()
        .any(|f| f.eq_ignore_ascii_case(&item.file_path))
    {
        cfg.allowed_files.push(item.file_path.clone());
        state.storage.save_config(&cfg)?;
    }

    Ok(item)
}

#[tauri::command]
fn get_scenes(state: State<'_, Arc<AppState>>) -> Vec<ScenePreset> {
    state.scenes.lock().unwrap().clone()
}

#[tauri::command]
fn save_scene(scene: ScenePreset, state: State<'_, Arc<AppState>>) -> Result<Vec<ScenePreset>, String> {
    let mut scenes = state.scenes.lock().unwrap();
    let mut entry = scene;
    entry.builtin = false;
    if let Some(existing) = scenes.iter_mut().find(|s| s.id == entry.id && !s.builtin) {
        *existing = entry;
    } else {
        scenes.push(entry);
    }
    state.storage.save_user_scenes(&scenes)?;
    Ok(scenes.clone())
}

#[tauri::command]
fn delete_scene(scene_id: String, state: State<'_, Arc<AppState>>) -> Result<Vec<ScenePreset>, String> {
    let mut scenes = state.scenes.lock().unwrap();
    scenes.retain(|s| s.builtin || s.id != scene_id);
    state.storage.save_user_scenes(&scenes)?;
    Ok(scenes.clone())
}

#[tauri::command]
fn get_history(state: State<'_, Arc<AppState>>) -> Vec<HistoryEntry> {
    state.history.lock().unwrap().clone()
}

#[tauri::command]
fn append_history(
    title: String,
    subtitle: String,
    kind: String,
    ref_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<HistoryEntry>, String> {
    let mut history = state.history.lock().unwrap();
    let ts = now_unix();
    let entry = HistoryEntry {
        id: format!("hist-{ts}-{}", history.len()),
        title,
        subtitle,
        kind: kind.clone(),
        ref_id: ref_id.clone(),
        timestamp_unix: ts,
    };

    // Deduplicate consecutive identical entries
    if history.first().map(|e| (&e.ref_id, &e.kind)) == Some((&ref_id, &kind)) {
        history[0] = entry;
    } else {
        history.insert(0, entry);
        if history.len() > 80 {
            history.truncate(80);
        }
    }
    state.storage.save_history(&history)?;

    // Notify plugins of playback/scene lifecycle event
    let plugins = state.plugins.lock().unwrap();
    if kind == "scene" {
        plugins.dispatch_event(&PluginEvent::SceneChanged { scene_index: 1 });
    } else {
        plugins.dispatch_event(&PluginEvent::TrackChanged {
            bpm_hint: 76,
            mood_code: 1,
        });
    }

    Ok(history.clone())
}

#[tauri::command]
fn clear_history(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut history = state.history.lock().unwrap();
    history.clear();
    state.storage.save_history(&history)
}

#[tauri::command]
fn list_plugins(state: State<'_, Arc<AppState>>) -> PluginRegistrySnapshot {
    state.plugins.lock().unwrap().snapshot()
}

#[tauri::command]
fn reload_plugins(state: State<'_, Arc<AppState>>) -> PluginRegistrySnapshot {
    let cfg = state.config.lock().unwrap().clone();
    let dirs = state.plugin_dirs(&cfg);
    let mut mgr = state.plugins.lock().unwrap();
    mgr.reload(&dirs);
    mgr.snapshot()
}

#[tauri::command]
fn set_plugin_enabled(
    plugin_id: String,
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<PluginRegistrySnapshot, String> {
    let mut mgr = state.plugins.lock().unwrap();
    mgr.set_enabled(&plugin_id, enabled)
        .map_err(|e| e.to_string())?;
    let disabled = mgr.disabled_ids();
    let snapshot = mgr.snapshot();
    drop(mgr);

    let mut cfg = state.config.lock().unwrap();
    cfg.disabled_plugins = disabled;
    state.storage.save_config(&cfg)?;

    Ok(snapshot)
}

#[tauri::command]
fn install_wasm_plugin(
    source_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<PluginRegistrySnapshot, String> {
    let src = Path::new(source_path.trim());
    if !src.is_file() {
        return Err("Selected plugin file does not exist".to_string());
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if ext != "wasm" && ext != "wat" {
        return Err("Only .wasm or .wat WebAssembly plugins are supported".to_string());
    }
    let filename = src
        .file_name()
        .ok_or_else(|| "Invalid plugin file name".to_string())?;
    let dest = state.storage.plugins_dir.join(filename);
    fs::copy(src, &dest).map_err(|e| format!("Failed installing plugin: {e}"))?;

    let cfg = state.config.lock().unwrap().clone();
    let dirs = state.plugin_dirs(&cfg);
    let mut mgr = state.plugins.lock().unwrap();
    mgr.reload(&dirs);
    Ok(mgr.snapshot())
}

#[tauri::command]
fn run_plugin_eq_transform(
    plugin_id: String,
    bands: [f32; 5],
    intensity: f32,
    state: State<'_, Arc<AppState>>,
) -> Result<PluginEqTransformResult, String> {
    state
        .plugins
        .lock()
        .unwrap()
        .apply_eq_transform(&plugin_id, bands, intensity)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn notify_focus_complete(total_sessions: i32, state: State<'_, Arc<AppState>>) -> usize {
    state
        .plugins
        .lock()
        .unwrap()
        .dispatch_event(&PluginEvent::FocusSessionComplete { total_sessions })
}

#[tauri::command]
fn get_system_metrics(state: State<'_, Arc<AppState>>) -> SystemMetrics {
    let cfg = state.config.lock().unwrap().clone();
    let lib = state.storage.scan_library(&cfg);
    let (loaded_plugins, wasm_plugins) = state.plugins.lock().unwrap().counts();
    SystemMetrics {
        uptime_secs: state.started_at.elapsed().as_secs(),
        loaded_plugins,
        wasm_plugins,
        indexed_tracks: lib.music.len(),
        indexed_visuals: lib.visuals.len(),
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show / Hide Nocturne", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Nocturne")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("lofi", |ctx, request| {
            let state = ctx.app_handle().state::<Arc<AppState>>();
            let cfg = state.config.lock().unwrap().clone();
            media_protocol::handle_media_request(request, &state.storage, &cfg)
        })
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed resolving app data dir: {e}"))?;
            let storage = LocalStorage::new(data_dir)?;
            let config = storage.load_config();
            let scenes = storage.load_scenes();
            let history = storage.load_history();

            let _ = PluginManager::ensure_starter_wasm_plugins(&storage.plugins_dir);
            let mut plugin_mgr = PluginManager::new(&config.disabled_plugins);
            let mut plugin_dirs = vec![storage.plugins_dir.clone()];
            plugin_dirs.extend(config.custom_plugin_dirs.iter().map(PathBuf::from));
            plugin_mgr.reload(&plugin_dirs);

            let state = Arc::new(AppState {
                storage,
                config: Mutex::new(config),
                scenes: Mutex::new(scenes),
                history: Mutex::new(history),
                plugins: Mutex::new(plugin_mgr),
                started_at: Instant::now(),
            });

            app.manage(state);
            let _ = setup_tray(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            scan_library,
            add_library_directory,
            remove_library_directory,
            register_local_file,
            get_scenes,
            save_scene,
            delete_scene,
            get_history,
            append_history,
            clear_history,
            list_plugins,
            reload_plugins,
            set_plugin_enabled,
            install_wasm_plugin,
            run_plugin_eq_transform,
            notify_focus_complete,
            get_system_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lofi Desktop application");
}
