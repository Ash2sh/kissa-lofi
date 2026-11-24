use crate::audio::dsp::EQ_BAND_LABELS;
use crate::models::{
    builtin_eq_presets, EqPreset, PluginKind, PluginWidget, AMBIENT_CHANNELS, BUILTIN_STATIONS,
};
use crate::ui::scenes::BUILTIN_CANVAS_SCENES;
use crate::ui::{ActivePanel, KissaApp};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use egui::{Align2, Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui, Vec2};

pub fn format_clock(total_sec: u32) -> String {
    let m = total_sec / 60;
    let s = total_sec % 60;
    format!("{m:02}:{s:02}")
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 * 1024 {
        format!("{} KB", (bytes / 1024).max(1))
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn show_active_popover(ctx: &egui::Context, app: &mut KissaApp) {
    let Some(target) = app.active_panel.clone() else {
        return;
    };

    let mut close_requested = false;

    egui::Area::new(egui::Id::new("kissa_popover"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -96.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(14, 15, 22, 240))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 28),
                ))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(Margin::same(16))
                .shadow(egui::Shadow {
                    offset: [0, 18],
                    blur: 44,
                    spread: 2,
                    color: Color32::from_rgba_unmultiplied(0, 0, 0, 165),
                })
                .show(ui, |ui| {
                    ui.set_width(610.0);
                    ui.set_max_height(460.0);

                    match target {
                        ActivePanel::PluginManager => {
                            if panel_header(
                                ui,
                                "Plugin Manager",
                                &format!(
                                    "{} active dock buttons via unified Rust LofiPlugin trait",
                                    app.plugin_snap.dock_modules.len()
                                ),
                            ) {
                                close_requested = true;
                            }
                            ScrollArea::vertical().max_height(390.0).show(ui, |ui| {
                                render_plugin_manager(ui, app);
                            });
                        }
                        ActivePanel::ExternalPlugin { plugin_id, .. } => {
                            let (title, sub) = app
                                .active_plugin_panel
                                .as_ref()
                                .map(|p| (p.title.clone(), p.subtitle.clone()))
                                .unwrap_or_else(|| ("Wasm Plugin".to_string(), "".to_string()));
                            if panel_header(ui, &title, &sub) {
                                close_requested = true;
                            }
                            ScrollArea::vertical().max_height(390.0).show(ui, |ui| {
                                render_external_plugin(ui, app, &plugin_id);
                            });
                        }
                        ActivePanel::Builtin(id) => {
                            let (title, sub) = builtin_panel_meta(&id);
                            if panel_header(ui, title, sub) {
                                close_requested = true;
                            }
                            ScrollArea::vertical().max_height(390.0).show(ui, |ui| {
                                match id.as_str() {
                                    "eq" => render_eq(ui, app),
                                    "music" => render_music(ui, app),
                                    "visuals" => render_visuals(ui, app),
                                    "sounds" => render_sounds(ui, app),
                                    "scenes" => render_scenes(ui, app),
                                    "history" => render_history(ui, app),
                                    "share" => render_share(ui, app),
                                    "keys" => render_keys(ui, app),
                                    "focus" => render_focus(ui, app),
                                    "sleep" => render_sleep(ui, app),
                                    _ => {}
                                }
                            });
                        }
                    }
                });
        });

    if close_requested {
        app.close_panel();
    }
}

fn builtin_panel_meta(id: &str) -> (&'static str, &'static str) {
    match id {
        "eq" => (
            "Equalizer & Tape Filter",
            "5-band native Rust parametric EQ + Wasm DSP hooks",
        ),
        "music" => (
            "Music & Local Library",
            "Procedural Rust lofi stations & streaming local audio",
        ),
        "visuals" => (
            "Visual Backgrounds",
            "Native GPU Canvas scenes + local photo/video wallpapers",
        ),
        "sounds" => (
            "Ambient Soundscapes",
            "8-layer procedural Rust ambient mixer",
        ),
        "scenes" => (
            "Atmosphere Scenes",
            "One-click snapshots combining visual, EQ, ambience & station",
        ),
        "history" => ("Listening History", "Stored privately on your machine"),
        "share" => (
            "Share & Import Presets",
            "Portable offline atmosphere codes",
        ),
        "keys" => ("Keyboard Shortcuts", "Quick distraction-free control"),
        "focus" => (
            "Focus Timer",
            "Pomodoro rhythm with chime & local session checklist",
        ),
        "sleep" => (
            "Sleep Timer",
            "Gradual 30-second audio fade-out & automatic pause",
        ),
        _ => ("Module", ""),
    }
}

fn panel_header(ui: &mut Ui, title: &str, subtitle: &str) -> bool {
    let mut closed = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .size(15.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.label(
                RichText::new(subtitle)
                    .size(11.5)
                    .color(Color32::from_rgba_unmultiplied(210, 218, 235, 130)),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                closed = true;
            }
        });
    });
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(8.0);
    closed
}

fn card_frame(active: bool) -> Frame {
    Frame::new()
        .fill(if active {
            Color32::from_rgba_unmultiplied(56, 189, 248, 34)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 10)
        })
        .stroke(Stroke::new(
            1.0,
            if active {
                Color32::from_rgba_unmultiplied(125, 211, 252, 120)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 20)
            },
        ))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(10))
}

// --- Dedicated Plugin Manager (Built-in Dock Plugins + External Wasm Plugins) ---
fn render_plugin_manager(ui: &mut Ui, app: &mut KissaApp) {
    ui.horizontal(|ui| {
        let builtin_cnt = app
            .plugin_snap
            .plugins
            .iter()
            .filter(|p| p.kind == PluginKind::Builtin)
            .count();
        let wasm_cnt = app
            .plugin_snap
            .plugins
            .iter()
            .filter(|p| p.kind == PluginKind::Wasm)
            .count();
        if ui
            .selectable_label(
                app.plugin_mgr_tab == 0,
                format!("Built-in Dock Plugins ({builtin_cnt})"),
            )
            .clicked()
        {
            app.plugin_mgr_tab = 0;
        }
        if ui
            .selectable_label(
                app.plugin_mgr_tab == 1,
                format!("External Wasm Plugins ({wasm_cnt})"),
            )
            .clicked()
        {
            app.plugin_mgr_tab = 1;
        }
    });
    ui.add_space(8.0);

    let plugins = app.plugin_snap.plugins.clone();
    if app.plugin_mgr_tab == 0 {
        ui.label(
            RichText::new("Every core dock button is registered as a built-in Rust LofiPlugin. Toggle any module on or off to customize your dock.")
                .size(11.5)
                .color(Color32::from_rgb(165, 175, 195)),
        );
        ui.add_space(6.0);
        for p in plugins
            .into_iter()
            .filter(|p| p.kind == PluginKind::Builtin)
        {
            card_frame(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&p.name).strong().color(Color32::WHITE));
                            if let Some(dm) = &p.dock_module {
                                if let Some(k) = &dm.shortcut {
                                    ui.label(
                                        RichText::new(format!("[{k}]"))
                                            .monospace()
                                            .size(11.0)
                                            .color(Color32::from_rgb(125, 211, 252)),
                                    );
                                }
                            }
                        });
                        ui.label(
                            RichText::new(&p.description)
                                .size(11.0)
                                .color(Color32::from_rgb(155, 165, 185)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let lbl = if p.enabled { "On Dock" } else { "Hidden" };
                        if ui.selectable_label(p.enabled, lbl).clicked() {
                            app.toggle_plugin(&p.id, !p.enabled);
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    } else {
        ui.horizontal(|ui| {
            if ui.button("+ Install .wasm / .wat").clicked() {
                app.pick_and_install_wasm_plugin();
            }
            if ui.button("Add Plugin Folder").clicked() {
                app.pick_and_add_directory("plugins");
            }
            if ui.button("Reload Runtime").clicked() {
                app.reload_plugins();
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "Plugins dir: {}",
                app.library.directories.default_plugins_dir
            ))
            .monospace()
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
        );
        ui.add_space(6.0);

        for p in plugins.into_iter().filter(|p| p.kind == PluginKind::Wasm) {
            card_frame(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&p.name).strong().color(Color32::WHITE));
                            ui.label(
                                RichText::new(format!("WASM v{}", p.version))
                                    .size(10.5)
                                    .color(Color32::from_rgb(196, 181, 253)),
                            );
                        });
                        ui.label(
                            RichText::new(&p.description)
                                .size(11.0)
                                .color(Color32::from_rgb(155, 165, 185)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let lbl = if p.enabled { "Enabled" } else { "Disabled" };
                        if ui.selectable_label(p.enabled, lbl).clicked() {
                            app.toggle_plugin(&p.id, !p.enabled);
                        }
                        if p.enabled {
                            if let Some(dm) = &p.dock_module {
                                if ui.button("Open").clicked() {
                                    app.open_external_plugin(&p.id, &dm.id);
                                }
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    }
}

// --- Dynamic Declarative Panel Rendered by External Wasm Dock Plugins ---
fn render_external_plugin(ui: &mut Ui, app: &mut KissaApp, plugin_id: &str) {
    let Some(panel) = app.active_plugin_panel.clone() else {
        ui.label("No panel view exposed by this plugin.");
        return;
    };

    for widget in panel.widgets {
        match widget {
            PluginWidget::Text { label, value } => {
                card_frame(false).show(ui, |ui| {
                    ui.label(
                        RichText::new(label.to_uppercase())
                            .size(10.0)
                            .color(Color32::from_rgb(145, 155, 175)),
                    );
                    ui.label(
                        RichText::new(value)
                            .size(12.0)
                            .color(Color32::from_rgb(220, 226, 240)),
                    );
                });
                ui.add_space(6.0);
            }
            PluginWidget::Metrics { items } => {
                ui.columns(items.len().max(1), |cols| {
                    for (idx, item) in items.iter().enumerate() {
                        card_frame(false).show(&mut cols[idx], |ui| {
                            ui.label(
                                RichText::new(&item.label)
                                    .size(10.5)
                                    .color(Color32::from_rgb(145, 155, 175)),
                            );
                            ui.label(
                                RichText::new(&item.value)
                                    .strong()
                                    .size(13.0)
                                    .color(Color32::WHITE),
                            );
                        });
                    }
                });
                ui.add_space(8.0);
            }
            PluginWidget::Slider {
                id,
                label,
                min,
                max,
                mut value,
                unit,
                ..
            } => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).color(Color32::from_rgb(200, 210, 228)));
                    let changed = ui
                        .add(egui::Slider::new(&mut value, min..=max).suffix(unit))
                        .changed();
                    if changed {
                        app.invoke_plugin_action(plugin_id, &id, value);
                    }
                });
                ui.add_space(6.0);
            }
            PluginWidget::Actions { title, buttons } => {
                if let Some(t) = title {
                    ui.label(
                        RichText::new(t.to_uppercase())
                            .size(10.5)
                            .color(Color32::from_rgb(145, 155, 175)),
                    );
                    ui.add_space(4.0);
                }
                ui.horizontal_wrapped(|ui| {
                    for b in buttons {
                        let btn = if b.style == "accent" {
                            egui::Button::new(
                                RichText::new(&b.label).color(Color32::from_rgb(186, 230, 253)),
                            )
                            .fill(Color32::from_rgba_unmultiplied(56, 189, 248, 52))
                        } else {
                            egui::Button::new(&b.label)
                        };
                        if ui.add(btn).clicked() {
                            app.invoke_plugin_action(plugin_id, &b.id, 0.0);
                        }
                    }
                });
            }
        }
    }
}

// --- 1. EQ ---
fn render_eq(ui: &mut Ui, app: &mut KissaApp) {
    let mut all_presets: Vec<EqPreset> = builtin_eq_presets();
    all_presets.extend(app.plugin_snap.eq_presets.clone());
    all_presets.extend(app.config.custom_eq_presets.clone());

    ui.horizontal_wrapped(|ui| {
        for preset in &all_presets {
            let active = app.config.eq_preset_id == preset.id;
            if ui.selectable_label(active, &preset.name).clicked() {
                app.apply_eq_preset(preset.clone());
            }
        }
        if ui.button("Reset Flat").clicked() {
            app.reset_eq();
        }
    });

    ui.add_space(10.0);
    card_frame(false).show(ui, |ui| {
        ui.columns(5, |cols| {
            for i in 0..5 {
                cols[i].vertical_centered(|ui| {
                    ui.label(
                        RichText::new(format!("{:+.1}dB", app.config.eq_bands[i]))
                            .monospace()
                            .size(11.0)
                            .color(Color32::from_rgb(125, 211, 252)),
                    );
                    let mut val = app.config.eq_bands[i];
                    if ui
                        .add(
                            egui::Slider::new(&mut val, -12.0..=12.0)
                                .vertical()
                                .show_value(false),
                        )
                        .changed()
                    {
                        app.config.eq_bands[i] = (val * 2.0).round() / 2.0;
                        app.config.eq_preset_id = "custom".to_string();
                        app.audio.set_eq(app.config.eq_bands);
                        app.save_config();
                    }
                    ui.label(
                        RichText::new(EQ_BAND_LABELS[i])
                            .size(11.0)
                            .color(Color32::from_rgb(155, 165, 185)),
                    );
                });
            }
        });
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Tape Low-Pass");
        let mut hz = app.config.tape_cutoff_hz;
        if ui
            .add(egui::Slider::new(&mut hz, 1600.0..=20000.0).suffix(" Hz"))
            .changed()
        {
            app.config.tape_cutoff_hz = hz;
            app.audio.set_tape_cutoff(hz);
            app.save_config();
        }
    });

    let dsp_plugins: Vec<_> = app
        .plugin_snap
        .plugins
        .iter()
        .filter(|p| p.enabled && p.capabilities.iter().any(|c| c == "eq-dsp"))
        .cloned()
        .collect();
    if !dsp_plugins.is_empty() {
        ui.add_space(8.0);
        ui.label(
            RichText::new("WASM EQ DSP PLUGINS")
                .size(10.5)
                .color(Color32::from_rgb(145, 155, 175)),
        );
        ui.horizontal_wrapped(|ui| {
            for p in dsp_plugins {
                if ui.button(format!("Apply {} (Wasm)", p.name)).clicked() {
                    app.run_wasm_eq_transform(&p.id, 0.75);
                }
            }
        });
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut app.new_eq_name);
        if ui.button("Save Preset").clicked() && !app.new_eq_name.trim().is_empty() {
            let name = app.new_eq_name.trim().to_string();
            app.new_eq_name.clear();
            app.save_custom_eq_preset(name);
        }
    });
}

// --- 2. Music ---
fn render_music(ui: &mut Ui, app: &mut KissaApp) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(app.music_tab == 0, "Lofi Stations")
            .clicked()
        {
            app.music_tab = 0;
        }
        if ui
            .selectable_label(
                app.music_tab == 1,
                format!("Local Library ({})", app.library.music.len()),
            )
            .clicked()
        {
            app.music_tab = 1;
        }
    });
    ui.add_space(8.0);

    if app.music_tab == 0 {
        for chunk in BUILTIN_STATIONS.chunks(2) {
            ui.columns(2, |cols| {
                for (i, st) in chunk.iter().enumerate() {
                    let active = app.config.audio_source.kind == "station"
                        && app.config.audio_source.id == st.id;
                    let resp = card_frame(active)
                        .show(&mut cols[i], |ui| {
                            ui.horizontal(|ui| {
                                let col = Color32::from_rgb(
                                    st.accent_rgb[0],
                                    st.accent_rgb[1],
                                    st.accent_rgb[2],
                                );
                                ui.label(RichText::new(st.mood).size(10.0).strong().color(col));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format!("{} BPM", st.bpm as i32))
                                                .size(10.5)
                                                .color(Color32::from_rgb(155, 165, 185)),
                                        );
                                    },
                                );
                            });
                            ui.label(RichText::new(st.name).strong().color(Color32::WHITE));
                            ui.label(
                                RichText::new(st.description)
                                    .size(11.0)
                                    .color(Color32::from_rgb(155, 165, 185)),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        app.select_station(st.id, true);
                    }
                }
            });
            ui.add_space(6.0);
        }
    } else {
        // Local library playback bar
        if app.config.audio_source.kind == "local" {
            let (cur, dur) = app.audio.local_progress();
            card_frame(false).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("⏮").clicked() {
                        app.step_track(-1);
                    }
                    if ui.button("⏭").clicked() {
                        app.step_track(1);
                    }
                    if ui.selectable_label(app.config.shuffle, "Shuffle").clicked() {
                        app.config.shuffle = !app.config.shuffle;
                        app.save_config();
                    }
                    if ui
                        .button(format!("Repeat: {}", app.config.repeat_mode))
                        .clicked()
                    {
                        app.cycle_repeat();
                    }
                    ui.label(
                        RichText::new(format_clock(cur as u32))
                            .monospace()
                            .size(11.0),
                    );
                    let mut pos = cur;
                    if ui
                        .add(egui::Slider::new(&mut pos, 0.0..=dur.max(1.0)).show_value(false))
                        .changed()
                    {
                        app.audio.seek_local(pos);
                    }
                    ui.label(
                        RichText::new(format_clock(dur as u32))
                            .monospace()
                            .size(11.0),
                    );
                });
            });
            ui.add_space(6.0);
        }

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.music_search)
                    .hint_text("Search local music..."),
            );
            if ui.button("+ Files").clicked() {
                app.pick_local_files("music");
            }
            if ui.button("+ Folder").clicked() {
                app.pick_and_add_directory("music");
            }
            if ui.button("Rescan").clicked() {
                app.rescan_library();
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "Music folder: {}",
                app.library.directories.default_music_dir
            ))
            .monospace()
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
        );
        ui.add_space(6.0);

        let q = app.music_search.to_lowercase();
        let tracks: Vec<_> = app
            .library
            .music
            .iter()
            .filter(|m| m.title.to_lowercase().contains(&q))
            .cloned()
            .collect();

        if tracks.is_empty() {
            ui.label(
                RichText::new("No local tracks indexed yet. Drop .mp3 / .flac / .wav / .ogg files onto Kissa or click + Files / + Folder.")
                    .color(Color32::from_rgb(155, 165, 185)),
            );
        } else {
            for t in tracks {
                let active =
                    app.config.audio_source.kind == "local" && app.config.audio_source.id == t.id;
                let resp = card_frame(active)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&t.title).strong().color(Color32::WHITE));
                                ui.label(
                                    RichText::new(format!(
                                        "{} · .{} · {}",
                                        t.subtitle,
                                        t.extension,
                                        format_bytes(t.size_bytes)
                                    ))
                                    .size(11.0)
                                    .color(Color32::from_rgb(155, 165, 185)),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(if active { "PLAYING" } else { "PLAY" })
                                            .size(10.5)
                                            .color(Color32::from_rgb(125, 211, 252)),
                                    );
                                },
                            );
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() {
                    app.play_local_track(&t, true);
                }
                ui.add_space(4.0);
            }
        }
    }
}

// --- 3. Visuals ---
fn render_visuals(ui: &mut Ui, app: &mut KissaApp) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(app.visuals_tab == 0, "Canvas Scenes")
            .clicked()
        {
            app.visuals_tab = 0;
        }
        if ui
            .selectable_label(
                app.visuals_tab == 1,
                format!("Local Photos & Videos ({})", app.library.visuals.len()),
            )
            .clicked()
        {
            app.visuals_tab = 1;
        }
        if ui
            .selectable_label(app.visuals_tab == 2, "Atmosphere & Power")
            .clicked()
        {
            app.visuals_tab = 2;
        }
    });
    ui.add_space(8.0);

    if app.visuals_tab == 0 {
        for chunk in BUILTIN_CANVAS_SCENES.chunks(2) {
            ui.columns(2, |cols| {
                for (i, sc) in chunk.iter().enumerate() {
                    let active =
                        app.config.visual.kind == "canvas" && app.config.visual.id == sc.id;
                    let resp = card_frame(active)
                        .show(&mut cols[i], |ui| {
                            let (bar_r, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 24.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(bar_r, 6.0, sc.swatch[1]);
                            ui.label(RichText::new(sc.name).strong().color(Color32::WHITE));
                            ui.label(
                                RichText::new(sc.subtitle)
                                    .size(11.0)
                                    .color(Color32::from_rgb(155, 165, 185)),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        app.select_canvas_scene(sc.id, sc.name);
                    }
                }
            });
            ui.add_space(6.0);
        }
    } else if app.visuals_tab == 1 {
        ui.horizontal(|ui| {
            if ui.button("+ Pick Photo / Video").clicked() {
                app.pick_local_files("visuals");
            }
            if ui.button("+ Add Folder").clicked() {
                app.pick_and_add_directory("visuals");
            }
            if ui.button("Rescan").clicked() {
                app.rescan_library();
            }
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "Visuals folder: {}",
                app.library.directories.default_visuals_dir
            ))
            .monospace()
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
        );
        ui.add_space(6.0);

        let items = app.library.visuals.clone();
        if items.is_empty() {
            ui.label(
                RichText::new("No local photos or videos yet. Drop .mp4, .webm, .jpg, .png, or .webp files onto Kissa.")
                    .color(Color32::from_rgb(155, 165, 185)),
            );
        } else {
            for item in items {
                let active = app.config.visual.id == item.file_path;
                let resp = card_frame(active)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&item.title).strong().color(Color32::WHITE));
                                ui.label(
                                    RichText::new(format!(
                                        "{:?} · .{} · {}",
                                        item.kind,
                                        item.extension,
                                        format_bytes(item.size_bytes)
                                    ))
                                    .size(11.0)
                                    .color(Color32::from_rgb(155, 165, 185)),
                                );
                            });
                        });
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() {
                    app.select_local_visual(&item);
                }
                ui.add_space(4.0);
            }
        }
    } else {
        let mut changed = false;
        changed |= ui
            .add(
                egui::Slider::new(&mut app.config.visual_fx.rain_overlay, 0.0..=1.0)
                    .text("Rain Overlay"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut app.config.visual_fx.vignette, 0.0..=1.0).text("Vignette"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut app.config.visual_fx.dim, 0.0..=0.75).text("Dimmer"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut app.config.visual_fx.warm_tint, 0.0..=1.0).text("Warm Tint"),
            )
            .changed();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Ultra-Low Resource FPS Cap:");
            for fps in [15_u32, 30, 60] {
                if ui
                    .selectable_label(app.config.visual_fx.fps_cap == fps, format!("{fps} FPS"))
                    .clicked()
                {
                    app.config.visual_fx.fps_cap = fps;
                    changed = true;
                }
            }
        });
        if changed {
            app.save_config();
        }
    }
}

// --- 4. Sounds ---
fn render_sounds(ui: &mut Ui, app: &mut KissaApp) {
    for chunk in AMBIENT_CHANNELS.chunks(2) {
        ui.columns(2, |cols| {
            for (i, ch) in chunk.iter().enumerate() {
                let mut level = app.config.ambience.get(ch.id).copied().unwrap_or(0.0);
                let active = level > 0.01;
                card_frame(active).show(&mut cols[i], |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(ch.name).strong().color(Color32::WHITE));
                            ui.label(
                                RichText::new(ch.description)
                                    .size(10.5)
                                    .color(Color32::from_rgb(155, 165, 185)),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let txt = if active {
                                format!("{}%", (level * 100.0).round() as i32)
                            } else {
                                "Off".to_string()
                            };
                            if ui.selectable_label(active, txt).clicked() {
                                app.set_ambience_level(ch.id, if active { 0.0 } else { 0.45 });
                            }
                        });
                    });
                    if ui
                        .add(egui::Slider::new(&mut level, 0.0..=1.0).show_value(false))
                        .changed()
                    {
                        app.set_ambience_level(ch.id, level);
                    }
                });
            }
        });
        ui.add_space(6.0);
    }

    if app.config.ambience.values().any(|&v| v > 0.01) && ui.button("Mute All Ambience").clicked() {
        app.clear_ambience();
    }
}

// --- 5. Scenes (Strictly Atmosphere Presets) ---
fn render_scenes(ui: &mut Ui, app: &mut KissaApp) {
    let scenes = app.scenes.clone();
    for chunk in scenes.chunks(2) {
        ui.columns(2, |cols| {
            for (i, sc) in chunk.iter().enumerate() {
                let active = app.config.active_scene_id == sc.id;
                let resp = card_frame(active)
                    .show(&mut cols[i], |ui| {
                        ui.horizontal(|ui| {
                            let col = Color32::from_rgb(
                                sc.accent_rgb[0],
                                sc.accent_rgb[1],
                                sc.accent_rgb[2],
                            );
                            ui.label(RichText::new(&sc.mood).size(10.0).strong().color(col));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !sc.builtin {
                                        if ui.small_button("Del").clicked() {
                                            app.delete_scene(&sc.id);
                                        }
                                    } else {
                                        ui.label(
                                            RichText::new("Curated")
                                                .size(10.0)
                                                .color(Color32::from_rgb(140, 150, 170)),
                                        );
                                    }
                                },
                            );
                        });
                        ui.label(RichText::new(&sc.name).strong().color(Color32::WHITE));
                        ui.label(
                            RichText::new(&sc.description)
                                .size(11.0)
                                .color(Color32::from_rgb(155, 165, 185)),
                        );
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() {
                    app.apply_scene(sc.clone());
                }
            }
        });
        ui.add_space(6.0);
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new("SAVE CURRENT ATMOSPHERE")
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
    );
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.new_scene_name).hint_text("Scene name..."));
        ui.add(
            egui::TextEdit::singleline(&mut app.new_scene_mood)
                .desired_width(90.0)
                .hint_text("MOOD"),
        );
        if ui.button("Save").clicked() && !app.new_scene_name.trim().is_empty() {
            let name = app.new_scene_name.trim().to_string();
            let mood = app.new_scene_mood.trim().to_uppercase();
            app.new_scene_name.clear();
            app.save_current_scene(
                name,
                if mood.is_empty() {
                    "CUSTOM".into()
                } else {
                    mood
                },
            );
        }
    });
}

// --- 6. History ---
fn render_history(ui: &mut Ui, app: &mut KissaApp) {
    if app.history.is_empty() {
        ui.label(
            RichText::new(
                "Your recent stations, local tracks, and scenes will appear here locally.",
            )
            .color(Color32::from_rgb(155, 165, 185)),
        );
        return;
    }
    if ui.button("Clear History").clicked() {
        app.clear_history();
        return;
    }
    ui.add_space(6.0);
    let entries = app.history.clone();
    for h in entries {
        let resp = card_frame(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&h.title).strong().color(Color32::WHITE));
                        ui.label(
                            RichText::new(format!("{} · {}", h.kind.to_uppercase(), h.subtitle))
                                .size(11.0)
                                .color(Color32::from_rgb(155, 165, 185)),
                        );
                    });
                });
            })
            .response
            .interact(egui::Sense::click());
        if resp.clicked() {
            app.replay_history(&h);
        }
        ui.add_space(4.0);
    }
}

// --- 7. Share ---
fn render_share(ui: &mut Ui, app: &mut KissaApp) {
    let json = serde_json::json!({
        "v": 1,
        "visual": if app.config.visual.kind == "canvas" { Some(&app.config.visual) } else { None },
        "visualFx": &app.config.visual_fx,
        "eqPresetId": &app.config.eq_preset_id,
        "eqBands": &app.config.eq_bands,
        "tapeCutoffHz": app.config.tape_cutoff_hz,
        "ambience": &app.config.ambience,
        "audioSource": if app.config.audio_source.kind == "station" { Some(&app.config.audio_source) } else { None },
    })
    .to_string();
    let code = format!("lofi-scene-v1:{}", B64.encode(json.as_bytes()));

    ui.label(
        RichText::new("EXPORT CURRENT ATMOSPHERE")
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
    );
    let mut ro = code.clone();
    ui.add(
        egui::TextEdit::multiline(&mut ro)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    if ui.button("Copy Scene Code").clicked() {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(code);
            app.toast("Scene code copied to clipboard");
        }
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new("IMPORT SHARED CODE OR JSON")
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
    );
    ui.add(
        egui::TextEdit::multiline(&mut app.share_import_text)
            .hint_text("Paste lofi-scene-v1:... or JSON here")
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    if ui.button("Apply Imported Scene").clicked() {
        let raw = app.share_import_text.trim().to_string();
        if !raw.is_empty() {
            app.import_share_code(&raw);
        }
    }
}

// --- 8. Keys ---
fn render_keys(ui: &mut Ui, app: &mut KissaApp) {
    let rows = [
        ("Space", "Play / Pause"),
        ("M", "Mute / Unmute"),
        ("Up / Down", "Volume Up / Down"),
        ("N / P", "Next / Previous Track"),
        ("1 - 9, 0", "Toggle Active Dock Plugins"),
        ("G", "Open Plugin Manager"),
        ("H", "Hide / Show HUD (Cinema)"),
        ("F", "Toggle Fullscreen"),
        ("Esc", "Close Active Panel"),
    ];
    for (k, desc) in rows {
        ui.horizontal(|ui| {
            ui.label(desc);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(k)
                        .monospace()
                        .color(Color32::from_rgb(125, 211, 252)),
                );
            });
        });
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Plugin Manager (G)").clicked() {
            app.toggle_plugin_manager();
        }
        if ui.button("Cinema Mode (H)").clicked() {
            app.toggle_hud();
        }
    });
}

// --- 9. Focus ---
fn render_focus(ui: &mut Ui, app: &mut KissaApp) {
    card_frame(false).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                for (lbl, is_break, mins) in [
                    ("Focus 25m", false, 25),
                    ("Deep 50m", false, 50),
                    ("Break 5m", true, 5),
                    ("Long 15m", true, 15),
                ] {
                    if ui.button(lbl).clicked() {
                        app.focus_running = false;
                        app.focus_is_break = is_break;
                        app.focus_remaining_sec = mins * 60;
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(format_clock(app.focus_remaining_sec))
                    .size(38.0)
                    .strong()
                    .monospace()
                    .color(Color32::WHITE),
            );
            ui.label(
                RichText::new(format!(
                    "{} · Completed today: {}",
                    if app.focus_is_break {
                        "Break time"
                    } else {
                        "Focus session"
                    },
                    app.config.completed_focus_sessions
                ))
                .size(11.5)
                .color(Color32::from_rgb(155, 165, 185)),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button(if app.focus_running {
                        "Pause"
                    } else {
                        "Start Focus"
                    })
                    .clicked()
                {
                    app.focus_running = !app.focus_running;
                }
                if ui.button("Reset").clicked() {
                    app.focus_running = false;
                    let mins = if app.focus_is_break {
                        app.config.break_minutes
                    } else {
                        app.config.focus_minutes
                    };
                    app.focus_remaining_sec = mins * 60;
                }
            });
        });
    });

    ui.add_space(8.0);
    ui.label(
        RichText::new("SESSION GOALS")
            .size(10.5)
            .color(Color32::from_rgb(145, 155, 175)),
    );
    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut app.new_task_text).hint_text("Add a focus task..."),
        );
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (ui.button("Add").clicked() || enter) && !app.new_task_text.trim().is_empty() {
            let text = app.new_task_text.trim().to_string();
            app.new_task_text.clear();
            app.add_focus_task(text);
        }
    });
    ui.add_space(4.0);
    let tasks = app.config.focus_tasks.clone();
    for task in tasks {
        ui.horizontal(|ui| {
            let mut done = task.done;
            if ui.checkbox(&mut done, &task.text).changed() {
                app.toggle_focus_task(&task.id);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").clicked() {
                    app.delete_focus_task(&task.id);
                }
            });
        });
    }
}

// --- 10. Sleep ---
fn render_sleep(ui: &mut Ui, app: &mut KissaApp) {
    card_frame(false).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            let txt = if app.sleep_running {
                format_clock(app.sleep_remaining_sec)
            } else {
                "Off".to_string()
            };
            ui.label(
                RichText::new(txt)
                    .size(38.0)
                    .strong()
                    .monospace()
                    .color(Color32::WHITE),
            );
            ui.label(
                RichText::new("Audio & ambience smoothly fade out during the final 30 seconds.")
                    .size(11.5)
                    .color(Color32::from_rgb(155, 165, 185)),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for m in [15_u32, 30, 45, 60, 90] {
                    if ui.button(format!("{m} min")).clicked() {
                        app.start_sleep(m);
                    }
                }
                if app.sleep_running && ui.button("Cancel").clicked() {
                    app.cancel_sleep();
                }
            });
        });
    });
}
