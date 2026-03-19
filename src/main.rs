mod core;
mod export;
mod gui;
mod settings;

#[cfg(feature = "web")]
mod web;

use eframe::egui;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().skip(1).collect();

    #[cfg(feature = "web")]
    {
        if args.iter().any(|a| a == "--web") {
            web::server::run();
            return;
        }
        if args.iter().any(|a| a == "--companion") {
            web::companion::run();
            return;
        }
    }

    run_desktop();
}

fn run_desktop() {
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
    .expect("Failed to run desktop app");
}
