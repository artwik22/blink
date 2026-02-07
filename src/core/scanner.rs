use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: u64,
    pub modified: String,
    pub icon_name: String,
}

impl FileEntry {
    pub fn size_display(&self) -> String {
        if self.is_directory {
            return String::from("--");
        }

        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size >= GB {
            format!("{:.1} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.1} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.1} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} B", self.size)
        }
    }
}

pub struct Scanner;

impl Scanner {
    pub fn scan(path: &Path) -> Result<Vec<FileEntry>, std::io::Error> {
        Self::scan_with_hidden(path, false)
    }

    pub fn scan_with_hidden(path: &Path, show_hidden: bool) -> Result<Vec<FileEntry>, std::io::Error> {
        // Pre-allocate with reasonable capacity to reduce reallocations
        let mut entries = Vec::with_capacity(64);

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if not showing hidden
            if !show_hidden && file_name.starts_with('.') {
                continue;
            }

            let is_directory = metadata.is_dir();
            let size = if is_directory { 0 } else { metadata.len() };

            let modified = metadata
                .modified()
                .map(|t| {
                    let datetime: DateTime<Local> = t.into();
                    datetime.format("%Y-%m-%d %H:%M").to_string()
                })
                .unwrap_or_else(|_| String::from("Unknown"));

            let icon_name = Self::get_icon_name(&file_name, is_directory);

            entries.push(FileEntry {
                name: file_name,
                path: entry.path(),
                is_directory,
                size,
                modified,
                icon_name,
            });
        }

        // Sort: directories first, then alphabetically (case-insensitive)
        entries.sort_by_cached_key(|e| {
            (!e.is_directory, e.name.to_lowercase())
        });

        Ok(entries)
    }

    fn get_icon_name(name: &str, is_directory: bool) -> String {
        if is_directory {
            return String::from("folder");
        }

        let extension = name.rsplit('.').next().unwrap_or("").to_lowercase();

        match extension.as_str() {
            // Documents
            "pdf" => "application-pdf",
            "doc" | "docx" | "odt" => "x-office-document",
            "xls" | "xlsx" | "ods" => "x-office-spreadsheet",
            "ppt" | "pptx" | "odp" => "x-office-presentation",
            "txt" | "md" | "rst" => "text-x-generic",

            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => "image-x-generic",

            // Audio
            "mp3" | "wav" | "flac" | "ogg" | "m4a" => "audio-x-generic",

            // Video
            "mp4" | "mkv" | "avi" | "mov" | "webm" => "video-x-generic",

            // Archives
            "zip" | "tar" | "gz" | "bz2" | "xz" | "rar" | "7z" => "package-x-generic",

            // Code
            "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" => {
                "text-x-script"
            }
            "html" | "css" | "xml" | "json" | "yaml" | "yml" | "toml" => "text-x-script",

            // Executables
            "sh" | "bash" => "application-x-executable",
            "exe" | "msi" => "application-x-executable",
            "deb" | "rpm" | "AppImage" => "application-x-executable",

            _ => "text-x-generic",
        }
        .to_string()
    }
}
