//! clkr — a lightweight auto clicker.

// No console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod engine;
mod hotkeys;
mod i18n;
mod keys;
mod platform;
mod settings;
mod ui;

use settings::AppConfig;

fn main() -> eframe::Result {
    let path = AppConfig::path();
    let cfg = AppConfig::load(&path);
    let size = if cfg.compact { app::COMPACT_SIZE } else { app::FULL_SIZE };

    let viewport = egui::ViewportBuilder::default()
        .with_title("clkr")
        .with_app_id("clkr")
        .with_inner_size(size)
        .with_min_inner_size(if cfg.compact { app::COMPACT_SIZE } else { app::MIN_SIZE })
        .with_decorations(false)
        .with_icon(icon());
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native("clkr", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, cfg, path)))))
}

/// Window icon, stored as raw 64×64 RGBA to avoid pulling in an image decoder.
fn icon() -> egui::IconData {
    egui::IconData { rgba: include_bytes!("../assets/icon-64.rgba").to_vec(), width: 64, height: 64 }
}
