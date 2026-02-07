use gtk4::prelude::*;
use gtk4::{gio, glib, CssProvider};
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

use crate::window::BlinkWindow;
use crate::core::ColorConfig;

const APP_ID: &str = "com.blink.fileexplorer";

pub struct BlinkApp {
    app: adw::Application,
    _css_provider: Rc<RefCell<Option<CssProvider>>>,
    _monitors: Vec<gio::FileMonitor>,
}

impl BlinkApp {
    pub fn new() -> Self {
        let app = adw::Application::builder().application_id(APP_ID).build();
        let css_provider = Rc::new(RefCell::new(None));

        let css_provider_clone = css_provider.clone();
        app.connect_startup(move |_| {
            load_css_with_colors(&css_provider_clone);
        });

        app.connect_activate(|app| {
            let window = BlinkWindow::new(app);
            window.present();
        });

        // Start monitoring for color changes
        let css_provider_monitor = css_provider.clone();
        let monitors = start_color_monitoring(css_provider_monitor);

        Self { 
            app,
            _css_provider: css_provider,
            _monitors: monitors,
        }
    }

    pub fn run(&self) -> glib::ExitCode {
        self.app.run()
    }
}

fn load_css_with_colors(css_provider_rc: &Rc<RefCell<Option<CssProvider>>>) {
    let config = ColorConfig::load();
    
    // Load base CSS
    let base_css = include_str!("style.css");
    
    // Replace Adwaita CSS variables with colors from colors.json
    let mut dynamic_css = base_css
        .replace("@define-color window_bg_color #242424", &format!("@define-color window_bg_color {}", config.background))
        .replace("@define-color window_fg_color #ffffff", &format!("@define-color window_fg_color {}", config.text))
        .replace("@define-color headerbar_bg_color #303030", &format!("@define-color headerbar_bg_color {}", config.primary))
        .replace("@define-color headerbar_fg_color #ffffff", &format!("@define-color headerbar_fg_color {}", config.text))
        .replace("@define-color card_bg_color #383838", &format!("@define-color card_bg_color {}", config.secondary))
        .replace("@define-color card_fg_color #ffffff", &format!("@define-color card_fg_color {}", config.text))
        .replace("@define-color accent_bg_color #3584e4", &format!("@define-color accent_bg_color {}", config.accent))
        .replace("@define-color accent_color #3584e4", &format!("@define-color accent_color {}", config.accent))
        .replace("@define-color sidebar_bg_color #2a2a2a", &format!("@define-color sidebar_bg_color {}", config.secondary))
        .replace("@define-color view_bg_color #1e1e1e", &format!("@define-color view_bg_color {}", config.background));
    
    // Apply rounding setting
    let rounding = config.rounding.as_deref().unwrap_or("rounded");
    if rounding == "sharp" {
        // Replace all border-radius values with 0px
        use regex::Regex;
        let re = Regex::new(r"border-radius:\s*[^;]+;").unwrap();
        dynamic_css = re.replace_all(&dynamic_css, "border-radius: 0px;").to_string();
    }
    
    let provider = CssProvider::new();
    provider.load_from_string(&dynamic_css);
    
    let display = gtk4::gdk::Display::default().expect("Could not connect to display");
    
    // Remove old provider if exists
    if let Some(old_provider) = css_provider_rc.borrow().as_ref() {
        gtk4::style_context_remove_provider_for_display(&display, old_provider);
    }
    
    // Add new provider
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    
    // Store provider reference
    *css_provider_rc.borrow_mut() = Some(provider);
}

fn start_color_monitoring(css_provider_rc: Rc<RefCell<Option<CssProvider>>>) -> Vec<gio::FileMonitor> {
    let mut monitors = Vec::new();
    let config_path = ColorConfig::get_config_path();
    
    // Monitor colors.json
    let file = gio::File::for_path(&config_path);
    if let Ok(monitor) = file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
        let css_provider_rc_clone = css_provider_rc.clone();
        monitor.connect_changed(move |_, _, _, event_type| {
            if matches!(event_type, gio::FileMonitorEvent::Changed | gio::FileMonitorEvent::ChangesDoneHint) {
                load_css_with_colors(&css_provider_rc_clone);
            }
        });
        monitors.push(monitor);
    }
    
    // Monitor /tmp/quickshell_color_change notification file
    let notification_file = gio::File::for_path("/tmp/quickshell_color_change");
    if let Ok(monitor) = notification_file.monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
        let css_provider_rc_clone = css_provider_rc.clone();
        monitor.connect_changed(move |_, _, _, event_type| {
            if matches!(event_type, gio::FileMonitorEvent::Changed | gio::FileMonitorEvent::ChangesDoneHint | gio::FileMonitorEvent::Created) {
                load_css_with_colors(&css_provider_rc_clone);
            }
        });
        monitors.push(monitor);
    }
    
    monitors
}
