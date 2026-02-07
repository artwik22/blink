use gtk4::glib::{self, Object};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::gio;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

const PINNED_FILE: &str = ".blink_pinned";

// ============================================================================
// PinnedFolderObject - GObject wrapper for use in gio::ListStore
// ============================================================================

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct PinnedFolderObject {
        pub path: RefCell<String>,
        pub name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PinnedFolderObject {
        const NAME: &'static str = "BlinkPinnedFolderObject";
        type Type = super::PinnedFolderObject;
        type ParentType = Object;
    }

    impl ObjectImpl for PinnedFolderObject {}
}

glib::wrapper! {
    pub struct PinnedFolderObject(ObjectSubclass<imp::PinnedFolderObject>);
}

impl PinnedFolderObject {
    pub fn new(path: &Path, name: &str) -> Self {
        let obj: Self = Object::builder().build();
        obj.set_path(path);
        obj.set_name(name);
        obj
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.imp().path.borrow().clone())
    }

    pub fn path_string(&self) -> String {
        self.imp().path.borrow().clone()
    }

    pub fn set_path(&self, path: &Path) {
        *self.imp().path.borrow_mut() = path.to_string_lossy().to_string();
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn set_name(&self, name: &str) {
        *self.imp().name.borrow_mut() = name.to_string();
    }
}

// ============================================================================
// PinnedFolderStore - Wrapper around gio::ListStore with persistence
// ============================================================================

#[derive(Clone)]
pub struct PinnedFolderStore {
    store: gio::ListStore,
}

impl PinnedFolderStore {
    pub fn new() -> Self {
        let store = gio::ListStore::new::<PinnedFolderObject>();
        let instance = Self { store };
        instance.load_from_file();
        instance
    }

    pub fn store(&self) -> &gio::ListStore {
        &self.store
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("blink")
            .join(PINNED_FILE)
    }

    /// Normalize path for comparison
    pub fn normalize_path(path: &Path) -> PathBuf {
        if let Ok(canonical) = path.canonicalize() {
            return canonical;
        }
        
        let path_str = path.to_string_lossy().to_string();
        let trimmed = path_str.trim_end_matches('/');
        
        if trimmed.is_empty() {
            PathBuf::from("/")
        } else {
            PathBuf::from(trimmed)
        }
    }

    /// Load pinned folders from file into store
    fn load_from_file(&self) {
        let config_path = Self::config_path();
        
        if !config_path.exists() {
            return;
        }

        match fs::read_to_string(&config_path) {
            Ok(content) => {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    
                    let parts: Vec<&str> = trimmed.splitn(2, '|').collect();
                    if parts.is_empty() {
                        continue;
                    }
                    
                    let path_str = parts[0].trim();
                    let path = PathBuf::from(path_str);
                    
                    if !path.exists() {
                        continue;
                    }
                    
                    let name = if parts.len() > 1 {
                        parts[1].trim().to_string()
                    } else {
                        path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_string_lossy().to_string())
                    };
                    
                    let obj = PinnedFolderObject::new(&path, &name);
                    self.store.append(&obj);
                }
            }
            Err(e) => {
                eprintln!("Failed to read pinned folders: {}", e);
            }
        }
    }

    /// Save current store contents to file
    pub fn save_to_file(&self) -> Result<(), std::io::Error> {
        let config_path = Self::config_path();
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut content = String::from("# Blink pinned folders\n");
        
        for i in 0..self.store.n_items() {
            if let Some(obj) = self.store.item(i) {
                if let Ok(pinned) = obj.downcast::<PinnedFolderObject>() {
                    content.push_str(&format!("{}|{}\n", pinned.path_string(), pinned.name()));
                }
            }
        }
        
        fs::write(&config_path, content)
    }

    /// Check if a path is already pinned
    pub fn is_pinned(&self, path: &Path) -> bool {
        let normalized = Self::normalize_path(path);
        self.find_index(&normalized).is_some()
    }

    /// Find index of path in store
    fn find_index(&self, path: &Path) -> Option<u32> {
        let normalized = Self::normalize_path(path);
        
        for i in 0..self.store.n_items() {
            if let Some(obj) = self.store.item(i) {
                if let Ok(pinned) = obj.downcast::<PinnedFolderObject>() {
                    let stored_path = Self::normalize_path(&pinned.path());
                    if stored_path == normalized {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    /// Add a folder to pinned list
    pub fn add(&self, path: &Path) -> Result<(), std::io::Error> {
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {:?}", path),
            ));
        }

        let normalized = Self::normalize_path(path);
        
        if self.is_pinned(&normalized) {
            return Ok(());
        }
        
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        
        let obj = PinnedFolderObject::new(&normalized, &name);
        self.store.append(&obj);
        self.save_to_file()
    }

    /// Remove a folder from pinned list
    pub fn remove(&self, path: &Path) -> Result<(), std::io::Error> {
        let normalized = Self::normalize_path(path);
        
        if let Some(index) = self.find_index(&normalized) {
            self.store.remove(index);
            self.save_to_file()
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path not found in pinned list: {:?}", path),
            ))
        }
    }

    /// Toggle pin status of a folder
    pub fn toggle_pin(&self, path: &Path) -> Result<bool, std::io::Error> {
        let normalized = Self::normalize_path(path);
        
        if self.is_pinned(&normalized) {
            self.remove(&normalized)?;
            Ok(false)
        } else {
            self.add(&normalized)?;
            Ok(true)
        }
    }

    /// Rename a pinned folder's display name
    pub fn rename(&self, path: &Path, new_name: &str) -> Result<(), std::io::Error> {
        let normalized = Self::normalize_path(path);
        
        if let Some(index) = self.find_index(&normalized) {
            if let Some(obj) = self.store.item(index) {
                if let Ok(pinned) = obj.downcast::<PinnedFolderObject>() {
                    pinned.set_name(new_name);
                    return self.save_to_file();
                }
            }
        }
        
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path not found in pinned list: {:?}", path),
        ))
    }
}

impl Default for PinnedFolderStore {
    fn default() -> Self {
        Self::new()
    }
}
