use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE: &str = "keybinds.conf";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeybindAction {
    ToggleHidden,
    OpenTerminal,
    SelectAll,
    Refresh,
    OpenWithMicro,
    Back,
    Forward,
    Up,
    Home,
    Copy,
    Cut,
    Paste,
    Delete,
    Rename,
}

#[derive(Clone, Debug)]
pub struct Keybind {
    pub key: String,
    pub modifiers: Vec<String>,
}

pub struct KeybindConfig;

impl KeybindConfig {
    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("blink")
            .join(CONFIG_FILE)
    }

    pub fn load() -> HashMap<KeybindAction, Keybind> {
        let config_path = Self::config_path();
        
        // Default keybinds
        let mut keybinds: HashMap<KeybindAction, Keybind> = HashMap::new();
        keybinds.insert(KeybindAction::ToggleHidden, Keybind {
            key: "h".to_string(),
            modifiers: vec!["Control".to_string()],
        });
        keybinds.insert(KeybindAction::OpenTerminal, Keybind {
            key: "h".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::SelectAll, Keybind {
            key: "a".to_string(),
            modifiers: vec!["Control".to_string()],
        });
        keybinds.insert(KeybindAction::Refresh, Keybind {
            key: "F5".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::OpenWithMicro, Keybind {
            key: "m".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::Back, Keybind {
            key: "Mouse8".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::Forward, Keybind {
            key: "Mouse9".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::Up, Keybind {
            key: "Up".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::Home, Keybind {
            key: "Home".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::Copy, Keybind {
            key: "c".to_string(),
            modifiers: vec!["Control".to_string()],
        });
        keybinds.insert(KeybindAction::Cut, Keybind {
            key: "x".to_string(),
            modifiers: vec!["Control".to_string()],
        });
        keybinds.insert(KeybindAction::Paste, Keybind {
            key: "v".to_string(),
            modifiers: vec!["Control".to_string()],
        });
        keybinds.insert(KeybindAction::Delete, Keybind {
            key: "Delete".to_string(),
            modifiers: vec![],
        });
        keybinds.insert(KeybindAction::Rename, Keybind {
            key: "F2".to_string(),
            modifiers: vec![],
        });

        // Load from file if exists
        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed.starts_with('#') {
                            continue;
                        }
                        
                        // Format: action=key:mod1,mod2
                        if let Some((action_str, keybind_str)) = trimmed.split_once('=') {
                            let action_str = action_str.trim();
                            let keybind_str = keybind_str.trim();
                            
                            let action = match action_str {
                                "toggle_hidden" => KeybindAction::ToggleHidden,
                                "open_terminal" => KeybindAction::OpenTerminal,
                                "select_all" => KeybindAction::SelectAll,
                                "refresh" => KeybindAction::Refresh,
                                "open_with_micro" => KeybindAction::OpenWithMicro,
                                "back" => KeybindAction::Back,
                                "forward" => KeybindAction::Forward,
                                "up" => KeybindAction::Up,
                                "home" => KeybindAction::Home,
                                "copy" => KeybindAction::Copy,
                                "cut" => KeybindAction::Cut,
                                "paste" => KeybindAction::Paste,
                                "delete" => KeybindAction::Delete,
                                "rename" => KeybindAction::Rename,
                                _ => continue,
                            };
                            
                            // Parse key:modifiers
                            if let Some((key, mods_str)) = keybind_str.split_once(':') {
                                let key = key.trim().to_string();
                                let modifiers: Vec<String> = if mods_str.trim().is_empty() {
                                    vec![]
                                } else {
                                    mods_str.split(',').map(|s| s.trim().to_string()).collect()
                                };
                                keybinds.insert(action, Keybind { key, modifiers });
                            } else {
                                // No modifiers
                                keybinds.insert(action, Keybind {
                                    key: keybind_str.to_string(),
                                    modifiers: vec![],
                                });
                            }
                        }
                    }
                }
                Err(_) => {
                    // If read fails, use defaults and save them
                    if let Err(e) = Self::save(&keybinds) {
                        eprintln!("Failed to save default keybinds: {}", e);
                    }
                }
            }
        } else {
            // Save defaults
            if let Err(e) = Self::save(&keybinds) {
                eprintln!("Failed to save default keybinds: {}", e);
            }
        }

        keybinds
    }

    pub fn save(keybinds: &HashMap<KeybindAction, Keybind>) -> Result<(), std::io::Error> {
        let config_path = Self::config_path();
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut content = String::from("# Blink keybinds configuration\n");
        content.push_str("# Format: action=key:modifier1,modifier2\n");
        content.push_str("# Modifiers: Control, Shift, Alt, Super\n");
        content.push_str("# Special keys: F1-F12, Up, Down, Left, Right, Home, End, Delete, etc.\n");
        content.push_str("# Mouse buttons: Mouse8, Mouse9\n\n");
        
        // Write in a consistent order
        let actions = vec![
            ("toggle_hidden", KeybindAction::ToggleHidden),
            ("open_terminal", KeybindAction::OpenTerminal),
            ("select_all", KeybindAction::SelectAll),
            ("refresh", KeybindAction::Refresh),
            ("open_with_micro", KeybindAction::OpenWithMicro),
            ("back", KeybindAction::Back),
            ("forward", KeybindAction::Forward),
            ("up", KeybindAction::Up),
            ("home", KeybindAction::Home),
            ("copy", KeybindAction::Copy),
            ("cut", KeybindAction::Cut),
            ("paste", KeybindAction::Paste),
            ("delete", KeybindAction::Delete),
            ("rename", KeybindAction::Rename),
        ];
        
        for (name, action) in actions {
            if let Some(keybind) = keybinds.get(&action) {
                let mods_str = if keybind.modifiers.is_empty() {
                    String::new()
                } else {
                    format!(":{}", keybind.modifiers.join(","))
                };
                content.push_str(&format!("{}={}{}\n", name, keybind.key, mods_str));
            }
        }
        
        fs::write(&config_path, content)
    }

    pub fn action_to_string(action: &KeybindAction) -> &'static str {
        match action {
            KeybindAction::ToggleHidden => "toggle_hidden",
            KeybindAction::OpenTerminal => "open_terminal",
            KeybindAction::SelectAll => "select_all",
            KeybindAction::Refresh => "refresh",
            KeybindAction::OpenWithMicro => "open_with_micro",
            KeybindAction::Back => "back",
            KeybindAction::Forward => "forward",
            KeybindAction::Up => "up",
            KeybindAction::Home => "home",
            KeybindAction::Copy => "copy",
            KeybindAction::Cut => "cut",
            KeybindAction::Paste => "paste",
            KeybindAction::Delete => "delete",
            KeybindAction::Rename => "rename",
        }
    }

    pub fn action_to_display_name(action: &KeybindAction) -> &'static str {
        match action {
            KeybindAction::ToggleHidden => "Toggle Hidden Files",
            KeybindAction::OpenTerminal => "Open Terminal",
            KeybindAction::SelectAll => "Select All",
            KeybindAction::Refresh => "Refresh",
            KeybindAction::OpenWithMicro => "Open with Micro",
            KeybindAction::Back => "Back",
            KeybindAction::Forward => "Forward",
            KeybindAction::Up => "Up (Parent)",
            KeybindAction::Home => "Home",
            KeybindAction::Copy => "Copy",
            KeybindAction::Cut => "Cut",
            KeybindAction::Paste => "Paste",
            KeybindAction::Delete => "Delete",
            KeybindAction::Rename => "Rename",
        }
    }

    pub fn keybind_to_string(keybind: &Keybind) -> String {
        if keybind.modifiers.is_empty() {
            keybind.key.clone()
        } else {
            format!("{}+{}", keybind.modifiers.join("+"), keybind.key)
        }
    }
}
