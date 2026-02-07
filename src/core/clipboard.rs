use std::path::PathBuf;

use super::FileOperations;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClipboardMode {
    None,
    Copy,
    Cut,
}

#[allow(dead_code)]
pub struct Clipboard {
    paths: Vec<PathBuf>,
    mode: ClipboardMode,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            mode: ClipboardMode::None,
        }
    }

    pub fn copy(&mut self, paths: Vec<PathBuf>) {
        self.paths = paths;
        self.mode = ClipboardMode::Copy;
    }

    pub fn cut(&mut self, paths: Vec<PathBuf>) {
        self.paths = paths;
        self.mode = ClipboardMode::Cut;
    }

    pub fn paste(&mut self, destination: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if self.mode == ClipboardMode::None || self.paths.is_empty() {
            return Ok(());
        }

        for source in &self.paths {
            let file_name = source
                .file_name()
                .ok_or("Invalid file name")?
                .to_string_lossy();

            let mut dest_path = destination.join(file_name.as_ref());

            // Handle duplicate names
            let mut counter = 1;
            while dest_path.exists() {
                let stem = source
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let extension = source
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
                    .unwrap_or_default();

                let new_name = format!("{} ({}){}", stem, counter, extension);
                dest_path = destination.join(new_name);
                counter += 1;
            }

            match self.mode {
                ClipboardMode::Copy => {
                    FileOperations::copy_file(source, &dest_path)?;
                }
                ClipboardMode::Cut => {
                    FileOperations::move_file(source, &dest_path)?;
                }
                ClipboardMode::None => {}
            }
        }

        // Clear clipboard after cut
        if self.mode == ClipboardMode::Cut {
            self.paths.clear();
            self.mode = ClipboardMode::None;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    #[allow(dead_code)]
    pub fn mode(&self) -> ClipboardMode {
        self.mode
    }
    
    pub fn get_paths(&self) -> Vec<PathBuf> {
        self.paths.clone()
    }
    
    pub fn clear(&mut self) {
        self.paths.clear();
        self.mode = ClipboardMode::None;
    }
}
