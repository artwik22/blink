use std::fs;
use std::path::PathBuf;
use dirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColorConfig {
    #[serde(rename = "showHiddenFiles", skip_serializing_if = "Option::is_none")]
    show_hidden_files: Option<bool>,
    #[serde(rename = "sidebarVisible", skip_serializing_if = "Option::is_none")]
    sidebar_visible: Option<bool>,
}

#[allow(dead_code)]
pub struct SidebarPrefs;

impl SidebarPrefs {
    #[allow(dead_code)]
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("blink")
            .join(".sidebar_prefs")
    }

    fn colors_config_path() -> PathBuf {
        // 1. Try ~/.config/alloy/colors.json (Global Alloy Config)
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".config").join("alloy").join("colors.json");
            if path.exists() {
                return path;
            }
        }

        // 2. Check QUICKSHELL_PROJECT_PATH first
        if let Ok(project_path) = std::env::var("QUICKSHELL_PROJECT_PATH") {
            let path = PathBuf::from(project_path).join("colors.json");
            if path.exists() {
                return path;
            }
        }

        // 3. Fallback to ~/.config/sharpshell/colors.json
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".config").join("sharpshell").join("colors.json");
            if path.exists() {
                return path;
            }
        }

        // Last resort
        PathBuf::from("/tmp/sharpshell/colors.json")
    }

    pub fn show_hidden_files() -> bool {
        let colors_path = Self::colors_config_path();
        
        if !colors_path.exists() {
            return false; // Default: hidden
        }

        match fs::read_to_string(&colors_path) {
            Ok(content) => {
                match serde_json::from_str::<ColorConfig>(&content) {
                    Ok(config) => config.show_hidden_files.unwrap_or(false),
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
}
