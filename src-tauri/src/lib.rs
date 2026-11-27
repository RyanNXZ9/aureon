pub mod adb;
pub mod commands;
pub mod device;
pub mod runner;
pub mod sqlite_token;
pub mod types;
pub mod ui;
#[cfg(windows)]
pub mod win_glass;

use eframe::egui;
use std::sync::Arc;

pub fn run() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Falha ao inicializar runtime Tokio");

    let handle = rt.handle().clone();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Auréon — Criado por ryan ")
        .with_decorations(false)
        .with_transparent(true)
        .with_inner_size([980.0, 650.0])
        .with_min_inner_size([860.0, 580.0]);

    if let Some(icon) = ui::load_window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        multisampling: 4,
        ..Default::default()
    };

    eframe::run_native(
        "Auréon — Criado por ryan ",
        options,
        Box::new(move |cc| Ok(Box::new(ui::AureonApp::new(cc, handle)))),
    )
    .expect("Falha ao iniciar interface nativa do Auréon");
}
