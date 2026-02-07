mod app;
mod core;
mod widgets;
mod window;

use app::BlinkApp;
use libadwaita as adw;

fn main() {
    // Initialize libadwaita
    adw::init().expect("Failed to initialize libadwaita");
    
    let app = BlinkApp::new();
    let _ = app.run();
}
