use crate::models::{VisualFxSettings, VisualSource};
use crate::video::BackgroundMedia;
use egui::{Color32, CornerRadius, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

pub struct CanvasSceneDef {
    pub id: &'static str,
    pub name: &'static str,
    pub subtitle: &'static str,
    pub swatch: [Color32; 2],
}

pub const BUILTIN_CANVAS_SCENES: &[CanvasSceneDef] = &[
    CanvasSceneDef {
        id: "rainy-window-cat",
        name: "Rainy Window Cat",
        subtitle: "Open night window, city lights & rain",
        swatch: [Color32::from_rgb(29, 26, 75), Color32::from_rgb(81, 74, 158)],
    },
    CanvasSceneDef {
        id: "neon-rain-skyline",
        name: "Neon Rain Skyline",
        subtitle: "Rainy midnight towers & soft neon glow",
        swatch: [Color32::from_rgb(13, 19, 43), Color32::from_rgb(74, 28, 79)],
    },
    CanvasSceneDef {
        id: "midnight-starlight",
        name: "Midnight Starlight",
        subtitle: "Attic skylight & drifting shooting stars",
        swatch: [Color32::from_rgb(6, 9, 26), Color32::from_rgb(29, 26, 61)],
    },
    CanvasSceneDef {
        id: "warm-study-dusk",
        name: "Warm Study Dusk",
        subtitle: "Golden dusk clouds & drifting dust motes",
        swatch: [Color32::from_rgb(39, 21, 46), Color32::from_rgb(201, 122, 83)],
    },
];

struct Raindrop {
    x: f32,
    y: f32,
    len: f32,
    speed: f32,
    alpha: f32,
    width: f32,
}

struct Star {
    x: f32,
    y: f32,
    r: f32,
    phase: f32,
    speed: f32,
}

struct CityWindow {
    bx: f32,
    by: f32,
    bw: f32,
    bh: f32,
    lit: bool,
    warm: f32,
    phase: f32,
}

pub struct SceneRenderer {
    raindrops: Vec<Raindrop>,
    stars: Vec<Star>,
    windows: Vec<CityWindow>,
    elapsed: f32,
}

impl SceneRenderer {
    pub fn new() -> Self {
        let mut rng: u32 = 0x51A7_1234;
        let mut next_f = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng as f32) / (u32::MAX as f32)
        };

        let raindrops = (0..180)
            .map(|_| {
                let d = next_f();
                Raindrop {
                    x: next_f(),
                    y: next_f(),
                    len: 0.03 + d * 0.072,
                    speed: 0.55 + d * 0.9,
                    alpha: 0.11 + d * 0.27,
                    width: 0.8 + d * 0.9,
                }
            })
            .collect();

        let stars = (0..110)
            .map(|_| Star {
                x: next_f(),
                y: next_f() * 0.72,
                r: 0.7 + next_f() * 1.6,
                phase: next_f() * std::f32::consts::TAU,
                speed: 0.6 + next_f() * 1.7,
            })
            .collect();

        let mut windows = Vec::new();
        let blocks = [
            (0.295_f32, 0.41_f32, 0.085_f32, 0.52_f32, 3usize, 6usize),
            (0.72, 0.28, 0.145, 0.66, 4, 7),
        ];
        for (x, y, w, h, cols, rows) in blocks {
            let cw = w / (cols as f32 * 1.65);
            let ch = h / (rows as f32 * 1.85);
            for r in 0..rows {
                for c in 0..cols {
                    let lit = ((r * 7 + c * 13) % 5) != 0 && (r > 0 || c > 0);
                    windows.push(CityWindow {
                        bx: x + 0.012 + c as f32 * (w / cols as f32) * 0.9,
                        by: y + 0.03 + r as f32 * (h / rows as f32) * 0.88,
                        bw: cw,
                        bh: ch,
                        lit,
                        warm: 0.72 + ((r * 3 + c * 5) % 4) as f32 * 0.07,
                        phase: r as f32 * 1.3 + c as f32 * 2.1,
                    });
                }
            }
        }

        Self {
            raindrops,
            stars,
            windows,
            elapsed: 0.0,
        }
    }

    pub fn paint(
        &mut self,
        painter: &Painter,
        rect: Rect,
        dt: f32,
        source: &VisualSource,
        fx: &VisualFxSettings,
        media: &BackgroundMedia,
    ) {
        self.elapsed += dt;

        if source.kind != "canvas" {
            if let Some(tex) = media.texture() {
                paint_cover_texture(painter, rect, tex);
            } else {
                painter.rect_filled(rect, 0.0, Color32::from_rgb(9, 11, 18));
            }
            if fx.rain_overlay > 0.01 {
                self.paint_rain(painter, rect, dt, fx.rain_overlay, rect.left(), rect.right());
            }
        } else {
            match source.id.as_str() {
                "neon-rain-skyline" => self.paint_neon_skyline(painter, rect, dt, fx.rain_overlay),
                "midnight-starlight" => self.paint_starlight(painter, rect, dt, fx.rain_overlay),
                "warm-study-dusk" => self.paint_warm_dusk(painter, rect),
                _ => self.paint_rainy_window_cat(painter, rect, dt, fx.rain_overlay),
            }
        }

        // Post FX overlays (zero allocations)
        if fx.warm_tint > 0.01 {
            let a = (fx.warm_tint * 42.0).clamp(0.0, 80.0) as u8;
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(255, 166, 92, a));
        }
        if fx.dim > 0.005 {
            let a = (fx.dim * 210.0).clamp(0.0, 200.0) as u8;
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(6, 7, 14, a));
        }
        if fx.vignette > 0.02 {
            paint_soft_vignette(painter, rect, fx.vignette);
        }
    }

    fn paint_rainy_window_cat(
        &mut self,
        painter: &Painter,
        rect: Rect,
        dt: f32,
        rain_overlay: f32,
    ) {
        let t = self.elapsed;
        let w = rect.width();
        let h = rect.height();
        let ox = rect.left();
        let oy = rect.top();
        let win_l = ox + w * 0.255;
        let win_r = ox + w * 0.815;

        // 1. Twilight indigo-violet gradient sky
        paint_vertical_gradient(
            painter,
            rect,
            Color32::from_rgb(29, 26, 75),
            Color32::from_rgb(82, 74, 158),
            Color32::from_rgb(112, 98, 190),
        );

        // Soft moonlight haze top-left
        painter.circle_filled(
            Pos2::new(ox + w * 0.37, oy + h * 0.09),
            w * 0.16,
            Color32::from_rgba_unmultiplied(208, 214, 245, 36),
        );
        // Horizon glow behind city towers
        painter.circle_filled(
            Pos2::new(ox + w * 0.53, oy + h * 0.72),
            w * 0.22,
            Color32::from_rgba_unmultiplied(178, 162, 238, 52),
        );

        // Drifting storm cloud puffs along top (strictly convex circles + band)
        let drift = (t * 0.08).sin() * 14.0;
        let cloud_col = Color32::from_rgba_unmultiplied(26, 24, 72, 152);
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(win_l, oy), Pos2::new(win_r, oy + h * 0.16)),
            0.0,
            cloud_col,
        );
        for &(cx, cy, cr) in &[
            (0.35_f32, 0.14_f32, 0.09_f32),
            (0.47, 0.17, 0.11),
            (0.58, 0.14, 0.10),
            (0.70, 0.18, 0.12),
        ] {
            painter.circle_filled(
                Pos2::new(ox + w * cx + drift, oy + h * cy),
                w * cr,
                cloud_col,
            );
        }

        // 2. Hanging overhead wires & warm glowing bokeh bulbs
        let sway = (t * 0.85).sin() * 4.0;
        let wire_stroke = Stroke::new(1.8, Color32::from_rgba_unmultiplied(16, 14, 34, 195));
        for &(y1, cy, y2) in &[
            (0.03_f32, 0.14_f32, 0.16_f32),
            (0.09, 0.21, 0.19),
            (0.0, 0.06, 0.03),
        ] {
            let mut pts = Vec::with_capacity(14);
            for i in 0..=12 {
                let u = i as f32 / 12.0;
                let inv = 1.0 - u;
                let x = win_l + (win_r - win_l) * u;
                let y = oy + (inv * inv * y1 + 2.0 * inv * u * cy + u * u * y2) * h + sway * u * inv * 3.0;
                pts.push(Pos2::new(x, y));
            }
            painter.add(Shape::line(pts, wire_stroke));
        }

        for &(bx, by, br) in &[
            (0.42_f32, 0.23_f32, 5.0_f32),
            (0.495, 0.14, 7.5),
            (0.515, 0.115, 4.5),
            (0.69, 0.045, 7.0),
            (0.785, 0.175, 6.5),
        ] {
            let pulse = 0.78 + 0.22 * (t * 1.4 + bx * 15.0).sin();
            let c = Pos2::new(ox + w * bx, oy + h * by + sway * 0.35);
            painter.circle_filled(
                c,
                br * 2.8,
                Color32::from_rgba_unmultiplied(255, 236, 178, (55.0 * pulse) as u8),
            );
            painter.circle_filled(
                c,
                br * 1.3,
                Color32::from_rgba_unmultiplied(255, 242, 200, (135.0 * pulse) as u8),
            );
            painter.circle_filled(
                c,
                br * 0.65,
                Color32::from_rgba_unmultiplied(255, 250, 232, (235.0 * pulse) as u8),
            );
        }

        // 3. Distant city silhouettes
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(ox + w * 0.425, oy + h * 0.54), Vec2::new(w * 0.092, h * 0.46)),
            0.0,
            Color32::from_rgb(38, 33, 76),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(ox + w * 0.44, oy + h * 0.495), Vec2::new(w * 0.062, h * 0.06)),
            0.0,
            Color32::from_rgb(38, 33, 76),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(ox + w * 0.28, oy + h * 0.39), Vec2::new(w * 0.095, h * 0.61)),
            0.0,
            Color32::from_rgb(25, 21, 47),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(ox + w * 0.275, oy + h * 0.375), Vec2::new(w * 0.108, h * 0.022)),
            0.0,
            Color32::from_rgb(25, 21, 47),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(ox + w * 0.715, oy + h * 0.29), Vec2::new(w * 0.12, h * 0.71)),
            0.0,
            Color32::from_rgb(22, 19, 43),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(ox + w * 0.735, oy + h * 0.265), Vec2::new(w * 0.048, h * 0.035)),
            0.0,
            Color32::from_rgb(22, 19, 43),
        );

        // Glowing apartment windows
        for cw in &self.windows {
            if !cw.lit {
                continue;
            }
            let r = Rect::from_min_size(
                Pos2::new(ox + w * cw.bx, oy + h * cw.by),
                Vec2::new(w * cw.bw, h * cw.bh),
            );
            let flick = 0.92 + 0.08 * (t * 0.7 + cw.phase).sin();
            painter.rect_filled(
                r.expand(4.0),
                3.0,
                Color32::from_rgba_unmultiplied(255, 224, 145, (42.0 * flick) as u8),
            );
            painter.rect_filled(
                r,
                1.0,
                Color32::from_rgba_unmultiplied(255, 232, 168, (cw.warm * flick * 245.0) as u8),
            );
            let muntin = Stroke::new(1.1, Color32::from_rgba_unmultiplied(22, 18, 42, 190));
            painter.line_segment([r.center_top(), r.center_bottom()], muntin);
            painter.line_segment([r.left_center(), r.right_center()], muntin);
        }

        // 4. Falling rain streaks inside the window view
        self.paint_rain(painter, rect, dt, rain_overlay.max(0.18), win_l, win_r);

        // 5. Potted leafy plant on sill
        let cx = ox + w * 0.415;
        let pot_y = oy + h * 0.79;
        let pot_w = w * 0.165;
        let pot_h = h * 0.22;
        let breeze = (t * 1.3).sin() * 2.5;
        let leaf_cols = [
            Color32::from_rgb(28, 50, 34),
            Color32::from_rgb(44, 78, 46),
            Color32::from_rgb(61, 107, 55),
            Color32::from_rgb(91, 137, 68),
            Color32::from_rgb(129, 168, 88),
        ];
        for i in 0..38 {
            let ratio = i as f32 / 37.0;
            let angle = std::f32::consts::PI + ratio * std::f32::consts::PI;
            let lx = cx + angle.cos() * pot_w * (0.48 + ((i * 7) % 5) as f32 * 0.045) + breeze * 0.6;
            let ly = pot_y + 10.0 + angle.sin() * h * (0.09 + ((i * 11) % 6) as f32 * 0.015);
            let rad = 9.0 + (i % 4) as f32 * 4.2;
            painter.circle_filled(Pos2::new(lx, ly), rad, leaf_cols[i % leaf_cols.len()]);
        }
        let pot_pts = vec![
            Pos2::new(cx - pot_w * 0.48, pot_y),
            Pos2::new(cx + pot_w * 0.48, pot_y),
            Pos2::new(cx + pot_w * 0.37, pot_y + pot_h),
            Pos2::new(cx - pot_w * 0.37, pot_y + pot_h),
        ];
        painter.add(Shape::convex_polygon(
            pot_pts,
            Color32::from_rgb(98, 38, 34),
            Stroke::NONE,
        ));
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(cx - pot_w * 0.51, pot_y - 4.0), Vec2::new(pot_w * 1.02, 12.0)),
            2.0,
            Color32::from_rgb(122, 48, 40),
        );

        // 6. Perched black cat silhouette looking left into the rain
        let breath = (t * 1.2).sin() * 1.6;
        let ear_twitch = if (t * 0.45).sin() > 0.96 {
            (t * 18.0).sin() * 2.5
        } else {
            0.0
        };
        let cat_col = Color32::from_rgb(11, 10, 23);
        // Body polygon
        let body = vec![
            Pos2::new(ox + w * 0.642, oy + h),
            Pos2::new(ox + w * 0.646, oy + h * 0.72),
            Pos2::new(ox + w * 0.642, oy + h * 0.50 + breath),
            Pos2::new(ox + w * 0.725, oy + h * 0.44 + breath),
            Pos2::new(ox + w * 0.785, oy + h * 0.66),
            Pos2::new(ox + w * 0.818, oy + h),
        ];
        painter.add(Shape::convex_polygon(body, cat_col, Stroke::NONE));
        // Head + pointed ears
        let head_center = Pos2::new(ox + w * 0.675, oy + h * 0.415 + breath);
        painter.circle_filled(head_center, w * 0.048, cat_col);
        // Snout profile left
        painter.circle_filled(
            Pos2::new(ox + w * 0.625, oy + h * 0.455 + breath),
            w * 0.018,
            cat_col,
        );
        // Left front ear
        painter.add(Shape::convex_polygon(
            vec![
                Pos2::new(ox + w * 0.625, oy + h * 0.39 + breath),
                Pos2::new(ox + w * 0.612 + ear_twitch, oy + h * 0.275 + breath),
                Pos2::new(ox + w * 0.668, oy + h * 0.345 + breath),
            ],
            cat_col,
            Stroke::NONE,
        ));
        // Back ear
        painter.add(Shape::convex_polygon(
            vec![
                Pos2::new(ox + w * 0.662, oy + h * 0.35 + breath),
                Pos2::new(ox + w * 0.686, oy + h * 0.305 + breath),
                Pos2::new(ox + w * 0.72, oy + h * 0.385 + breath),
            ],
            cat_col,
            Stroke::NONE,
        ));
        // Whiskers
        let whisker_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(15, 13, 28, 210));
        let wx = ox + w * 0.618;
        let wy = oy + h * 0.462 + breath;
        for i in -1..=1 {
            painter.line_segment(
                [
                    Pos2::new(wx, wy + i as f32 * 3.0),
                    Pos2::new(wx - 16.0, wy + i as f32 * 6.0 + 1.0),
                ],
                whisker_stroke,
            );
        }

        // 7. Dark apartment interior walls & open casement window frames
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(ox, oy), Pos2::new(win_l, oy + h)),
            0.0,
            Color32::from_rgb(13, 12, 28),
        );
        painter.rect_stroke(
            Rect::from_min_size(Pos2::new(ox - 10.0, oy - 8.0), Vec2::new(w * 0.085, h * 0.21)),
            0.0,
            Stroke::new(8.0, Color32::from_rgb(25, 23, 48)),
            egui::StrokeKind::Middle,
        );
        // Open left sash glass reflection
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(win_l - w * 0.032, oy + h * 0.04), Pos2::new(win_l - 4.0, oy + h * 0.96)),
            0.0,
            Color32::from_rgba_unmultiplied(112, 118, 198, 28),
        );
        // Right dark room wall & open right casement sash
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(win_r, oy), Pos2::new(ox + w, oy + h)),
            0.0,
            Color32::from_rgb(11, 10, 24),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(win_r + 10.0, oy + h * 0.04), Vec2::new(w * 0.048, h * 0.92)),
            0.0,
            Color32::from_rgba_unmultiplied(105, 115, 200, 24),
        );
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(win_r + w * 0.068, oy + h * 0.48), Vec2::new(7.0, h * 0.10)),
            2.0,
            Color32::from_rgb(32, 29, 59),
        );
    }

    fn paint_neon_skyline(&mut self, painter: &Painter, rect: Rect, dt: f32, rain_overlay: f32) {
        let t = self.elapsed;
        let w = rect.width();
        let h = rect.height();
        let ox = rect.left();
        let oy = rect.top();

        paint_vertical_gradient(
            painter,
            rect,
            Color32::from_rgb(9, 12, 30),
            Color32::from_rgb(25, 21, 60),
            Color32::from_rgb(58, 28, 75),
        );
        painter.circle_filled(
            Pos2::new(ox + w * 0.52, oy + h * 0.72),
            w * 0.34,
            Color32::from_rgba_unmultiplied(236, 72, 153, 44),
        );

        let towers: &[(f32, f32, f32, Color32)] = &[
            (0.03, 0.10, 0.48, Color32::from_rgb(56, 189, 248)),
            (0.14, 0.08, 0.62, Color32::from_rgb(244, 114, 182)),
            (0.24, 0.12, 0.42, Color32::from_rgb(251, 191, 36)),
            (0.39, 0.09, 0.68, Color32::from_rgb(56, 189, 248)),
            (0.50, 0.13, 0.54, Color32::from_rgb(244, 114, 182)),
            (0.66, 0.10, 0.64, Color32::from_rgb(167, 139, 250)),
            (0.78, 0.09, 0.49, Color32::from_rgb(56, 189, 248)),
            (0.89, 0.10, 0.58, Color32::from_rgb(244, 114, 182)),
        ];

        for (i, &(tx, tw, th, accent)) in towers.iter().enumerate() {
            let bx = ox + tx * w;
            let bw = tw * w;
            let bh = th * h;
            let by = oy + h - bh;
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bx, by), Vec2::new(bw, bh)),
                0.0,
                Color32::from_rgb(18, 17, 42),
            );
            let pulse = (0.65 + 0.35 * (t * 1.2 + i as f32).sin()) * 220.0;
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(bx + 6.0, by + 6.0), Vec2::new(bw - 12.0, 3.0)),
                1.0,
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), pulse as u8),
            );
            for r in 0..9 {
                for c in 0..4 {
                    if (r * 5 + c * 3 + i * 7) % 4 != 0 {
                        continue;
                    }
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(bx + 10.0 + c as f32 * (bw / 4.6), by + 22.0 + r as f32 * 24.0),
                            Vec2::new(7.0, 10.0),
                        ),
                        1.0,
                        Color32::from_rgba_unmultiplied(255, 232, 180, 145),
                    );
                }
            }
        }

        self.paint_rain(painter, rect, dt, rain_overlay.max(0.28), ox, ox + w);
    }

    fn paint_starlight(&mut self, painter: &Painter, rect: Rect, dt: f32, rain_overlay: f32) {
        let t = self.elapsed;
        let w = rect.width();
        let h = rect.height();
        let ox = rect.left();
        let oy = rect.top();

        paint_vertical_gradient(
            painter,
            rect,
            Color32::from_rgb(5, 8, 22),
            Color32::from_rgb(14, 25, 56),
            Color32::from_rgb(30, 26, 66),
        );

        for s in &self.stars {
            let a = (0.25 + 0.75 * (0.5 + 0.5 * (t * s.speed + s.phase).sin())) * 240.0;
            painter.circle_filled(
                Pos2::new(ox + s.x * w, oy + s.y * h),
                s.r,
                Color32::from_rgba_unmultiplied(226, 236, 255, a as u8),
            );
        }

        let moon = Pos2::new(ox + w * 0.26, oy + h * 0.24);
        painter.circle_filled(moon, 72.0, Color32::from_rgba_unmultiplied(200, 220, 255, 26));
        painter.circle_filled(moon, 24.0, Color32::from_rgb(238, 244, 255));
        painter.circle_filled(moon + Vec2::new(9.0, -5.0), 21.0, Color32::from_rgb(11, 20, 44));

        // Occasional shooting star
        let cycle = t % 9.0;
        if cycle < 1.1 {
            let p = cycle / 1.1;
            let sx = ox + w * (0.78 - p * 0.32);
            let sy = oy + h * (0.12 + p * 0.20);
            painter.line_segment(
                [Pos2::new(sx, sy), Pos2::new(sx + 70.0, sy - 42.0)],
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, ((1.0 - p) * 220.0) as u8)),
            );
        }

        // Mountain silhouettes (strictly convex triangles + base)
        let mtn_col = Color32::from_rgb(9, 11, 27);
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(ox, oy + h * 0.77), Pos2::new(ox + w, oy + h)),
            0.0,
            mtn_col,
        );
        painter.add(Shape::convex_polygon(
            vec![
                Pos2::new(ox, oy + h * 0.78),
                Pos2::new(ox + w * 0.26, oy + h * 0.68),
                Pos2::new(ox + w * 0.54, oy + h * 0.78),
            ],
            mtn_col,
            Stroke::NONE,
        ));
        painter.add(Shape::convex_polygon(
            vec![
                Pos2::new(ox + w * 0.46, oy + h * 0.78),
                Pos2::new(ox + w * 0.78, oy + h * 0.66),
                Pos2::new(ox + w, oy + h * 0.78),
            ],
            mtn_col,
            Stroke::NONE,
        ));

        if rain_overlay > 0.05 {
            self.paint_rain(painter, rect, dt, rain_overlay * 0.6, ox, ox + w);
        }
    }

    fn paint_warm_dusk(&mut self, painter: &Painter, rect: Rect) {
        let t = self.elapsed;
        let w = rect.width();
        let h = rect.height();
        let ox = rect.left();
        let oy = rect.top();

        paint_vertical_gradient(
            painter,
            rect,
            Color32::from_rgb(29, 18, 41),
            Color32::from_rgb(138, 69, 68),
            Color32::from_rgb(226, 154, 98),
        );
        let sun = Pos2::new(ox + w * 0.5, oy + h * 0.72);
        painter.circle_filled(sun, w * 0.22, Color32::from_rgba_unmultiplied(255, 224, 165, 58));
        painter.circle_filled(sun, w * 0.09, Color32::from_rgba_unmultiplied(255, 238, 195, 115));

        for i in 0..34 {
            let mx = ox + ((i as f32 * 97.3 + (t * 0.4 + i as f32).sin() * 30.0).rem_euclid(w));
            let my = oy + ((i as f32 * 53.7 - t * (6.0 + (i % 4) as f32 * 2.0)).rem_euclid(h));
            painter.circle_filled(
                Pos2::new(mx, my),
                1.5 + (i % 3) as f32 * 0.7,
                Color32::from_rgba_unmultiplied(255, 232, 185, 78),
            );
        }

        painter.rect_filled(
            Rect::from_min_max(Pos2::new(ox, oy + h * 0.82), Pos2::new(ox + w, oy + h)),
            0.0,
            Color32::from_rgb(24, 13, 29),
        );
    }

    fn paint_rain(
        &mut self,
        painter: &Painter,
        rect: Rect,
        dt: f32,
        intensity: f32,
        min_x: f32,
        max_x: f32,
    ) {
        let count = ((self.raindrops.len() as f32) * intensity.clamp(0.0, 1.0)) as usize;
        let span = (max_x - min_x).max(1.0);
        let h = rect.height();
        let oy = rect.top();

        for d in self.raindrops.iter_mut().take(count) {
            d.y += d.speed * dt * (0.75 + intensity * 0.45);
            d.x += d.speed * dt * 0.085;
            if d.y > 1.1 {
                d.y -= 1.2;
            }
            if d.x > 1.05 {
                d.x -= 1.1;
            }

            let x = min_x + d.x * span;
            let y = oy + d.y * h;
            let dy = d.len * h;
            let dx = dy * 0.11;
            let alpha = (d.alpha * intensity * 255.0).clamp(10.0, 180.0) as u8;
            painter.line_segment(
                [Pos2::new(x, y), Pos2::new(x + dx, y + dy)],
                Stroke::new(d.width, Color32::from_rgba_unmultiplied(196, 206, 245, alpha)),
            );
        }
    }
}

fn paint_vertical_gradient(
    painter: &Painter,
    rect: Rect,
    top: Color32,
    mid: Color32,
    bottom: Color32,
) {
    let mut mesh = Mesh::default();
    let mid_y = rect.top() + rect.height() * 0.58;
    let verts = [
        (rect.left_top(), top),
        (rect.right_top(), top),
        (Pos2::new(rect.left(), mid_y), mid),
        (Pos2::new(rect.right(), mid_y), mid),
        (rect.left_bottom(), bottom),
        (rect.right_bottom(), bottom),
    ];
    for (pos, color) in verts {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    mesh.add_triangle(2, 3, 4);
    mesh.add_triangle(3, 5, 4);
    painter.add(Shape::mesh(mesh));
}

fn paint_soft_vignette(painter: &Painter, rect: Rect, strength: f32) {
    let a = (strength * 140.0).clamp(0.0, 160.0) as u8;
    let edge = Color32::from_rgba_unmultiplied(4, 5, 12, a);
    let clear = Color32::TRANSPARENT;
    let band_h = rect.height() * 0.18;
    paint_vertical_gradient(
        painter,
        Rect::from_min_max(rect.left_top(), Pos2::new(rect.right(), rect.top() + band_h)),
        edge,
        Color32::from_rgba_unmultiplied(4, 5, 12, a / 3),
        clear,
    );
    paint_vertical_gradient(
        painter,
        Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - band_h), rect.right_bottom()),
        clear,
        Color32::from_rgba_unmultiplied(4, 5, 12, a / 3),
        edge,
    );
}

fn paint_cover_texture(painter: &Painter, rect: Rect, tex: &egui::TextureHandle) {
    let tex_size = tex.size_vec2();
    if tex_size.x <= 0.0 || tex_size.y <= 0.0 {
        return;
    }
    let rect_aspect = rect.width() / rect.height().max(1.0);
    let tex_aspect = tex_size.x / tex_size.y;
    let uv = if rect_aspect > tex_aspect {
        let crop_h = tex_aspect / rect_aspect;
        let pad = (1.0 - crop_h) * 0.5;
        Rect::from_min_max(Pos2::new(0.0, pad), Pos2::new(1.0, 1.0 - pad))
    } else {
        let crop_w = rect_aspect / tex_aspect;
        let pad = (1.0 - crop_w) * 0.5;
        Rect::from_min_max(Pos2::new(pad, 0.0), Pos2::new(1.0 - pad, 1.0))
    };
    painter.image(tex.id(), rect, uv, Color32::WHITE);
    let _ = CornerRadius::ZERO;
}
