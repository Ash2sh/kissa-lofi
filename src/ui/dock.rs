use crate::models::DockModuleSpec;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Painter, Pos2, Rect,
    Sense, Shape, Stroke, Vec2,
};

pub struct DockViewState<'a> {
    pub mood: &'a str,
    pub title: &'a str,
    pub subtitle: &'a str,
    pub status_line: &'a str,
    pub playing: bool,
    pub volume: f32,
    pub muted: bool,
    pub modules: &'a [DockModuleSpec],
    pub active_module_id: Option<&'a str>,
    pub scenes_dot: bool,
    pub sounds_active: bool,
    pub focus_active: bool,
    pub sleep_active: bool,
}

#[derive(Default)]
pub struct DockEvents {
    pub toggle_play: bool,
    pub toggle_mute: bool,
    pub new_volume: Option<f32>,
    pub clicked_module: Option<DockModuleSpec>,
    pub track_clicked: bool,
}

pub fn show_dock(ctx: &egui::Context, state: &DockViewState<'_>) -> DockEvents {
    let mut events = DockEvents::default();

    egui::Area::new(egui::Id::new("kissa_bottom_dock"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -20.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(12, 13, 19, 226))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                ))
                .corner_radius(CornerRadius::same(18))
                .inner_margin(Margin::symmetric(14, 10))
                .shadow(egui::Shadow {
                    offset: [0, 12],
                    blur: 32,
                    spread: 2,
                    color: Color32::from_rgba_unmultiplied(0, 0, 0, 145),
                })
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;

                        // 1. Track / Station Thumbnail + Metadata
                        let track_resp = ui
                            .horizontal_centered(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;
                                // Rounded blue gradient album tile
                                let (thumb_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
                                paint_album_thumb(ui.painter(), thumb_rect, state.playing);

                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.set_min_width(136.0);
                                    ui.set_max_width(172.0);
                                    ui.label(
                                        egui::RichText::new(format!("((o)) {}", state.mood))
                                            .size(9.5)
                                            .strong()
                                            .color(Color32::from_rgb(175, 185, 208)),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(state.title)
                                                .size(13.0)
                                                .strong()
                                                .color(Color32::from_rgb(245, 247, 251)),
                                        )
                                        .truncate(),
                                    );
                                    ui.label(egui::RichText::new(state.subtitle).size(10.0).color(
                                        Color32::from_rgba_unmultiplied(210, 218, 235, 130),
                                    ));
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(state.status_line).size(9.8).color(
                                                Color32::from_rgba_unmultiplied(210, 218, 235, 155),
                                            ),
                                        )
                                        .truncate(),
                                    );
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if track_resp.clicked() {
                            events.track_clicked = true;
                        }

                        // 2. White Circular Play/Pause Button
                        let (play_rect, play_resp) =
                            ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
                        let play_bg = if play_resp.hovered() {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(245, 248, 252)
                        };
                        ui.painter()
                            .circle_filled(play_rect.center(), 19.0, play_bg);
                        paint_play_pause_glyph(ui.painter(), play_rect.center(), state.playing);
                        if play_resp
                            .on_hover_text(if state.playing {
                                "Pause (Space)"
                            } else {
                                "Play (Space)"
                            })
                            .clicked()
                        {
                            events.toggle_play = true;
                        }

                        // 3. Volume Mute + Sleek Slider
                        ui.horizontal_centered(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let (mute_rect, mute_resp) =
                                ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                            paint_speaker_icon(
                                ui.painter(),
                                mute_rect,
                                state.muted || state.volume <= 0.01,
                                if mute_resp.hovered() {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(200, 208, 224)
                                },
                            );
                            if mute_resp.on_hover_text("Mute / Unmute (M)").clicked() {
                                events.toggle_mute = true;
                            }

                            let mut vol_pct = if state.muted {
                                0.0
                            } else {
                                state.volume * 100.0
                            };
                            let slider = egui::Slider::new(&mut vol_pct, 0.0..=100.0)
                                .show_value(false)
                                .trailing_fill(true);
                            ui.style_mut().spacing.slider_width = 86.0;
                            if ui.add(slider).changed() {
                                events.new_volume = Some((vol_pct / 100.0).clamp(0.0, 1.0));
                            }
                        });

                        // Subtle divider
                        let (div_rect, _) =
                            ui.allocate_exact_size(Vec2::new(1.0, 32.0), Sense::hover());
                        ui.painter().rect_filled(
                            div_rect,
                            0.0,
                            Color32::from_rgba_unmultiplied(255, 255, 255, 24),
                        );

                        // 4. Dynamic Unified Dock Plugin Buttons (Built-ins + External Wasm!)
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            for module in state.modules {
                                let is_active = state.active_module_id == Some(module.id.as_str());
                                let (btn_rect, resp) =
                                    ui.allocate_exact_size(Vec2::new(47.0, 44.0), Sense::click());

                                let bg = if is_active {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 36)
                                } else if resp.hovered() {
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 20)
                                } else {
                                    Color32::TRANSPARENT
                                };
                                if bg != Color32::TRANSPARENT {
                                    ui.painter()
                                        .rect_filled(btn_rect, CornerRadius::same(10), bg);
                                }
                                if is_active {
                                    ui.painter().rect_stroke(
                                        btn_rect,
                                        CornerRadius::same(10),
                                        Stroke::new(
                                            1.0,
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 45),
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                }

                                let fg = if is_active || resp.hovered() {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgba_unmultiplied(235, 240, 252, 205)
                                };

                                let icon_center =
                                    Pos2::new(btn_rect.center().x, btn_rect.top() + 15.0);
                                paint_dock_icon(ui.painter(), icon_center, &module.icon, fg);

                                ui.painter().text(
                                    Pos2::new(btn_rect.center().x, btn_rect.bottom() - 8.5),
                                    Align2::CENTER_CENTER,
                                    &module.label,
                                    FontId::proportional(10.5),
                                    fg,
                                );

                                // Status dots (green on Scenes, blue on active Sounds/Focus/Sleep, purple on external Wasm)
                                let dot_color = if module.panel_kind == "plugin" {
                                    Some(Color32::from_rgb(167, 139, 250))
                                } else if module.id == "scenes" && state.scenes_dot {
                                    Some(Color32::from_rgb(132, 204, 22))
                                } else if (module.id == "sounds" && state.sounds_active)
                                    || (module.id == "focus" && state.focus_active)
                                    || (module.id == "sleep" && state.sleep_active)
                                {
                                    Some(Color32::from_rgb(56, 189, 248))
                                } else {
                                    None
                                };
                                if let Some(dc) = dot_color {
                                    ui.painter().circle_filled(
                                        Pos2::new(btn_rect.right() - 7.0, btn_rect.top() + 6.5),
                                        3.3,
                                        dc,
                                    );
                                }

                                let tip = match &module.shortcut {
                                    Some(k) => format!("{} ({})", module.label, k),
                                    None => format!("{} (Wasm Plugin)", module.label),
                                };
                                if resp.on_hover_text(tip).clicked() {
                                    events.clicked_module = Some(module.clone());
                                }
                            }
                        });
                    });
                });
        });

    events
}

fn paint_album_thumb(painter: &Painter, rect: Rect, playing: bool) {
    painter.rect_filled(
        rect,
        CornerRadius::same(10),
        if playing {
            Color32::from_rgb(47, 118, 246)
        } else {
            Color32::from_rgb(37, 99, 235)
        },
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 65)),
        egui::StrokeKind::Inside,
    );
    // White musical note glyph
    let c = rect.center();
    let s = Stroke::new(2.2, Color32::WHITE);
    painter.line_segment([c + Vec2::new(-4.0, 6.0), c + Vec2::new(-4.0, -7.0)], s);
    painter.line_segment([c + Vec2::new(6.0, 4.0), c + Vec2::new(6.0, -9.0)], s);
    painter.line_segment(
        [c + Vec2::new(-4.5, -7.0), c + Vec2::new(6.5, -9.0)],
        Stroke::new(3.0, Color32::WHITE),
    );
    painter.circle_filled(c + Vec2::new(-6.5, 6.5), 3.2, Color32::WHITE);
    painter.circle_filled(c + Vec2::new(3.5, 4.5), 3.2, Color32::WHITE);
}

fn paint_play_pause_glyph(painter: &Painter, c: Pos2, playing: bool) {
    let dark = Color32::from_rgb(16, 18, 26);
    if playing {
        painter.rect_filled(
            Rect::from_center_size(c + Vec2::new(-3.6, 0.0), Vec2::new(3.6, 12.0)),
            1.2,
            dark,
        );
        painter.rect_filled(
            Rect::from_center_size(c + Vec2::new(3.6, 0.0), Vec2::new(3.6, 12.0)),
            1.2,
            dark,
        );
    } else {
        painter.add(Shape::convex_polygon(
            vec![
                c + Vec2::new(-4.0, -6.5),
                c + Vec2::new(7.0, 0.0),
                c + Vec2::new(-4.0, 6.5),
            ],
            dark,
            Stroke::NONE,
        ));
    }
}

fn paint_speaker_icon(painter: &Painter, rect: Rect, muted: bool, color: Color32) {
    let c = rect.center();
    let st = Stroke::new(1.5, color);
    painter.add(Shape::convex_polygon(
        vec![
            c + Vec2::new(-7.0, -2.5),
            c + Vec2::new(-3.5, -2.5),
            c + Vec2::new(0.5, -6.0),
            c + Vec2::new(0.5, 6.0),
            c + Vec2::new(-3.5, 2.5),
            c + Vec2::new(-7.0, 2.5),
        ],
        color,
        Stroke::NONE,
    ));
    if muted {
        painter.line_segment([c + Vec2::new(3.0, -3.0), c + Vec2::new(8.0, 3.0)], st);
        painter.line_segment([c + Vec2::new(8.0, -3.0), c + Vec2::new(3.0, 3.0)], st);
    } else {
        painter.line_segment([c + Vec2::new(3.5, -3.0), c + Vec2::new(5.0, 0.0)], st);
        painter.line_segment([c + Vec2::new(5.0, 0.0), c + Vec2::new(3.5, 3.0)], st);
        painter.line_segment([c + Vec2::new(6.2, -5.0), c + Vec2::new(8.2, 0.0)], st);
        painter.line_segment([c + Vec2::new(8.2, 0.0), c + Vec2::new(6.2, 5.0)], st);
    }
}

pub fn paint_dock_icon(painter: &Painter, c: Pos2, icon: &str, color: Color32) {
    let st = Stroke::new(1.45, color);
    match icon {
        "eq" => {
            for &(x, knob_y) in &[(-5.0_f32, 1.5_f32), (0.0, -2.0), (5.0, 2.5)] {
                painter.line_segment([c + Vec2::new(x, -6.5), c + Vec2::new(x, 6.5)], st);
                painter.line_segment(
                    [
                        c + Vec2::new(x - 2.0, knob_y),
                        c + Vec2::new(x + 2.0, knob_y),
                    ],
                    Stroke::new(2.0, color),
                );
            }
        }
        "music" => {
            painter.line_segment([c + Vec2::new(-2.5, 4.5), c + Vec2::new(-2.5, -5.0)], st);
            painter.line_segment([c + Vec2::new(5.0, 3.0), c + Vec2::new(5.0, -6.5)], st);
            painter.line_segment([c + Vec2::new(-2.5, -5.0), c + Vec2::new(5.0, -6.5)], st);
            painter.circle_stroke(c + Vec2::new(-4.5, 5.0), 2.2, st);
            painter.circle_stroke(c + Vec2::new(3.0, 3.5), 2.2, st);
        }
        "visuals" => {
            let r = Rect::from_center_size(c, Vec2::splat(13.0));
            painter.rect_stroke(r, 2.5, st, egui::StrokeKind::Middle);
            painter.circle_filled(c + Vec2::new(-2.5, -2.5), 1.3, color);
            painter.line_segment([c + Vec2::new(-5.5, 5.0), c + Vec2::new(0.0, 0.0)], st);
            painter.line_segment([c + Vec2::new(0.0, 0.0), c + Vec2::new(5.5, 5.0)], st);
        }
        "sounds" => {
            for dy in [-3.5_f32, 0.0, 3.5] {
                let pts = vec![
                    c + Vec2::new(-6.5, dy),
                    c + Vec2::new(-3.2, dy - 1.6),
                    c + Vec2::new(0.0, dy),
                    c + Vec2::new(3.2, dy + 1.6),
                    c + Vec2::new(6.5, dy),
                ];
                painter.add(Shape::line(pts, st));
            }
        }
        "scenes" => {
            // 4-point sparkle
            let pts = vec![
                c + Vec2::new(0.0, -6.5),
                c + Vec2::new(1.8, -1.8),
                c + Vec2::new(6.5, 0.0),
                c + Vec2::new(1.8, 1.8),
                c + Vec2::new(0.0, 6.5),
                c + Vec2::new(-1.8, 1.8),
                c + Vec2::new(-6.5, 0.0),
                c + Vec2::new(-1.8, -1.8),
                c + Vec2::new(0.0, -6.5),
            ];
            painter.add(Shape::line(pts, st));
        }
        "history" => {
            painter.circle_stroke(c, 6.0, st);
            painter.line_segment([c, c + Vec2::new(0.0, -3.5)], st);
            painter.line_segment([c, c + Vec2::new(2.8, 1.8)], st);
        }
        "share" => {
            painter.rect_stroke(
                Rect::from_center_size(c + Vec2::new(-2.6, 0.0), Vec2::new(7.5, 5.2)),
                2.6,
                st,
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                Rect::from_center_size(c + Vec2::new(2.6, 0.0), Vec2::new(7.5, 5.2)),
                2.6,
                st,
                egui::StrokeKind::Middle,
            );
        }
        "keys" => {
            let r = Rect::from_center_size(c, Vec2::new(14.0, 10.0));
            painter.rect_stroke(r, 2.2, st, egui::StrokeKind::Middle);
            for dx in [-3.5_f32, 0.0, 3.5] {
                painter.circle_filled(c + Vec2::new(dx, -1.8), 1.0, color);
            }
            painter.line_segment([c + Vec2::new(-3.5, 2.2), c + Vec2::new(3.5, 2.2)], st);
        }
        "focus" => {
            painter.circle_stroke(c + Vec2::new(0.0, 1.0), 5.6, st);
            painter.line_segment([c + Vec2::new(-2.5, -6.5), c + Vec2::new(2.5, -6.5)], st);
            painter.line_segment([c + Vec2::new(0.0, 1.0), c + Vec2::new(0.0, -2.0)], st);
            painter.line_segment([c + Vec2::new(0.0, 1.0), c + Vec2::new(2.6, 2.2)], st);
        }
        "sleep" => {
            painter.circle_stroke(c, 5.6, st);
            painter.circle_filled(c + Vec2::new(2.6, -2.0), 4.6, Color32::from_rgb(12, 13, 19));
        }
        "tape" => {
            let r = Rect::from_center_size(c, Vec2::new(14.5, 10.0));
            painter.rect_stroke(r, 2.2, st, egui::StrokeKind::Middle);
            painter.circle_stroke(c + Vec2::new(-3.4, 0.0), 1.8, st);
            painter.circle_stroke(c + Vec2::new(3.4, 0.0), 1.8, st);
            painter.line_segment([c + Vec2::new(-3.4, 2.0), c + Vec2::new(3.4, 2.0)], st);
        }
        "flame" => {
            let pts = vec![
                c + Vec2::new(0.0, -6.5),
                c + Vec2::new(4.5, 0.5),
                c + Vec2::new(2.5, 5.8),
                c + Vec2::new(-2.5, 5.8),
                c + Vec2::new(-4.5, 0.5),
                c + Vec2::new(0.0, -6.5),
            ];
            painter.add(Shape::line(pts, st));
        }
        _ => {
            // Default plugin hexagon/module icon
            let r = Rect::from_center_size(c, Vec2::splat(11.0));
            painter.rect_stroke(r, 3.0, st, egui::StrokeKind::Middle);
            painter.circle_filled(c, 2.0, color);
        }
    }
}
