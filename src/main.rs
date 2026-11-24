#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod models;
mod plugins;
mod storage;
mod ui;
mod video;

use std::path::PathBuf;
use ui::{KissaApp, StartupOverrides};

fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/kissa-icon.png");
    let img = image::load_from_memory(bytes).ok()?.thumbnail(64, 64).to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn parse_startup_overrides() -> StartupOverrides {
    let mut ov = StartupOverrides {
        video_path: None,
        audio_path: None,
        auto_play: false,
        rain_level: None,
        apply_wasm_tape: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--video" => {
                if let Some(p) = args.next() {
                    ov.video_path = Some(PathBuf::from(p));
                }
            }
            "--audio" => {
                if let Some(p) = args.next() {
                    ov.audio_path = Some(PathBuf::from(p));
                }
            }
            "--play" => ov.auto_play = true,
            "--rain" => {
                if let Some(v) = args.next().and_then(|s| s.parse::<f32>().ok()) {
                    ov.rain_level = Some(v.clamp(0.0, 1.0));
                }
            }
            "--wasm-tape" => ov.apply_wasm_tape = true,
            _ => {}
        }
    }
    ov
}

fn main() -> eframe::Result<()> {
    let overrides = parse_startup_overrides();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Kissa")
        .with_inner_size([1280.0, 760.0])
        .with_min_inner_size([840.0, 520.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        vsync: true,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Kissa",
        options,
        Box::new(move |cc| Ok(Box::new(KissaApp::new(cc, overrides)))),
    )
}
