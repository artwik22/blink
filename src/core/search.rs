use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::core::FileEntry;

pub struct GlobalSearch {
    results: Arc<Mutex<Vec<FileEntry>>>,
    is_searching: Arc<Mutex<bool>>,
}

impl GlobalSearch {
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(Vec::new())),
            is_searching: Arc::new(Mutex::new(false)),
        }
    }

    pub fn search(
        &self,
        query: &str,
        root_path: &Path,
        show_hidden: bool,
        on_progress: Option<Box<dyn Fn(usize) + Send + Sync>>,
        on_complete: Box<dyn Fn(Vec<FileEntry>) + Send + Sync>,
    ) {
        // Cancel previous search
        *self.is_searching.lock().unwrap() = false;
        
        // Start new search
        *self.is_searching.lock().unwrap() = true;
        self.results.lock().unwrap().clear();

        let query_lower = query.to_lowercase();
        let query_owned = query_lower.clone();
        let results = Arc::clone(&self.results);
        let is_searching = Arc::clone(&self.is_searching);
        let root_path = root_path.to_path_buf();
        let on_complete = Arc::new(Mutex::new(Some(on_complete)));

        thread::spawn(move || {
            let mut count = 0;
            Self::search_recursive(
                &root_path,
                &query_owned,
                show_hidden,
                &results,
                &is_searching,
                &mut count,
                on_progress.as_ref(),
            );

            // Call completion callback
            if let Some(callback) = on_complete.lock().unwrap().take() {
                let final_results = results.lock().unwrap().clone();
                callback(final_results);
            }
        });
    }

    fn search_recursive(
        path: &Path,
        query: &str,
        show_hidden: bool,
        results: &Arc<Mutex<Vec<FileEntry>>>,
        is_searching: &Arc<Mutex<bool>>,
        count: &mut usize,
        on_progress: Option<&Box<dyn Fn(usize) + Send + Sync>>,
    ) {
        // Check if search was cancelled
        if !*is_searching.lock().unwrap() {
            return;
        }

        // Limit search depth and results to reduce memory usage
        if *count > 5000 {
            return;
        }
        
        // Limit results in memory to prevent excessive RAM usage
        if results.lock().unwrap().len() > 5000 {
            return;
        }

        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries {
            if !*is_searching.lock().unwrap() {
                return;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if not showing hidden
            if !show_hidden && file_name.starts_with('.') {
                continue;
            }

            // Check if name matches query
            if file_name.to_lowercase().contains(query) {
                if let Ok(metadata) = entry.metadata() {
                    let is_directory = metadata.is_dir();
                    let size = if is_directory { 0 } else { metadata.len() };

                    let modified = metadata
                        .modified()
                        .map(|t| {
                            let datetime: chrono::DateTime<chrono::Local> = t.into();
                            datetime.format("%Y-%m-%d %H:%M").to_string()
                        })
                        .unwrap_or_else(|_| String::from("Unknown"));

                    let icon_name = Self::get_icon_name(&file_name, is_directory);

                    let file_entry = FileEntry {
                        name: file_name.clone(),
                        path: entry.path(),
                        is_directory,
                        size,
                        modified,
                        icon_name,
                    };

                    results.lock().unwrap().push(file_entry);
                    *count += 1;

                    if let Some(ref progress_cb) = on_progress {
                        progress_cb(*count);
                    }
                }
            }

            // Recurse into directories
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    // Skip certain system directories to avoid slow searches
                    let skip_dirs = [
                        "/proc", "/sys", "/dev", "/run", "/tmp", "/var/cache",
                        "/var/tmp", "/snap", "/.snapshots",
                    ];
                    let entry_path = entry.path();
                    if skip_dirs.iter().any(|d| entry_path.starts_with(d)) {
                        continue;
                    }

                    Self::search_recursive(
                        &entry.path(),
                        query,
                        show_hidden,
                        results,
                        is_searching,
                        count,
                        on_progress,
                    );
                }
            }
        }
    }

    fn get_icon_name(name: &str, is_directory: bool) -> String {
        if is_directory {
            return String::from("folder");
        }

        let extension = name.rsplit('.').next().unwrap_or("").to_lowercase();

        match extension.as_str() {
            "pdf" => "application-pdf",
            "doc" | "docx" | "odt" => "x-office-document",
            "xls" | "xlsx" | "ods" => "x-office-spreadsheet",
            "txt" | "md" | "rst" => "text-x-generic",
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => "image-x-generic",
            "mp3" | "wav" | "flac" | "ogg" | "m4a" => "audio-x-generic",
            "mp4" | "mkv" | "avi" | "mov" | "webm" => "video-x-generic",
            "zip" | "tar" | "gz" | "bz2" | "xz" | "rar" | "7z" => "package-x-generic",
            "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" => {
                "text-x-script"
            }
            "html" | "css" | "xml" | "json" | "yaml" | "yml" | "toml" => "text-x-script",
            "sh" | "bash" => "application-x-executable",
            _ => "text-x-generic",
        }
        .to_string()
    }

    pub fn cancel(&self) {
        *self.is_searching.lock().unwrap() = false;
    }
}
