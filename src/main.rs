mod core;
mod export;
mod gui;
mod settings;

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ink Density Tool")
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Ink Density Tool",
        options,
        Box::new(|cc| Ok(Box::new(gui::app::InkDensityApp::new(cc)))),
    )
}
