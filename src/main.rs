mod core;
mod settings;
mod export;
#[cfg(feature = "web")]
mod web;

fn main() {
    #[cfg(feature = "web")]
    let args: Vec<String> = std::env::args().collect();

    #[cfg(feature = "web")]
    {
        if args.iter().any(|a| a == "--web") {
            crate::web::server::run();
            return;
        }
        if args.iter().any(|a| a == "--companion") {
            crate::web::companion::run();
            return;
        }
    }

    eprintln!("Desktop mode is not yet available in this build.");
    eprintln!("Run with --web to start the web server on :8181.");
    eprintln!("Run with --companion to start the Illustrator bridge on :7432.");
}
