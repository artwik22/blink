use std::fs;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct FileOperations;

pub struct ProgressInfo {
    pub current_file: String,
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub files_copied: usize,
    pub total_files: usize,
}

impl FileOperations {
    pub fn copy_file(source: &Path, destination: &Path) -> io::Result<()> {
        if source.is_dir() {
            Self::copy_dir_recursive(source, destination)
        } else {
            fs::copy(source, destination)?;
            Ok(())
        }
    }

    fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
        fs::create_dir_all(destination)?;

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let dest_path = destination.join(entry.file_name());

            if source_path.is_dir() {
                Self::copy_dir_recursive(&source_path, &dest_path)?;
            } else {
                fs::copy(&source_path, &dest_path)?;
            }
        }

        Ok(())
    }

    // Calculate total size and file count for progress tracking
    pub fn calculate_total_size(paths: &[std::path::PathBuf]) -> (u64, usize) {
        let mut total_size = 0u64;
        let mut total_files = 0usize;
        
        for path in paths {
            Self::calculate_size_recursive(path, &mut total_size, &mut total_files);
        }
        
        (total_size, total_files)
    }
    
    fn calculate_size_recursive(path: &Path, total_size: &mut u64, total_files: &mut usize) {
        if let Ok(metadata) = path.metadata() {
            if metadata.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            Self::calculate_size_recursive(&entry.path(), total_size, total_files);
                        }
                    }
                }
            } else {
                *total_size += metadata.len();
                *total_files += 1;
            }
        }
    }

    // Async copy with progress reporting
    pub fn copy_file_with_progress(
        source: &Path,
        destination: &Path,
        progress: Option<Arc<Mutex<ProgressInfo>>>,
    ) -> io::Result<()> {
        if source.is_dir() {
            Self::copy_dir_recursive_with_progress(source, destination, progress)
        } else {
            Self::copy_single_file_with_progress(source, destination, progress)?;
            Ok(())
        }
    }

    fn copy_single_file_with_progress(
        source: &Path,
        destination: &Path,
        progress: Option<Arc<Mutex<ProgressInfo>>>,
    ) -> io::Result<()> {
        let source_file = fs::File::open(source)?;
        
        let mut dest_file = fs::File::create(destination)?;
        let mut source_reader = io::BufReader::new(source_file);
        
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        
        loop {
            let bytes_read = source_reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            dest_file.write_all(&buffer[..bytes_read])?;
            
            // Update progress
            if let Some(progress_info) = &progress {
                let mut prog = progress_info.lock().unwrap();
                prog.bytes_copied += bytes_read as u64;
                if prog.current_file.is_empty() {
                    prog.current_file = source.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
        }
        
        // Update file count
        if let Some(progress_info) = &progress {
            let mut prog = progress_info.lock().unwrap();
            prog.files_copied += 1;
        }
        
        Ok(())
    }

    fn copy_dir_recursive_with_progress(
        source: &Path,
        destination: &Path,
        progress: Option<Arc<Mutex<ProgressInfo>>>,
    ) -> io::Result<()> {
        fs::create_dir_all(destination)?;

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let source_path = entry.path();
            let dest_path = destination.join(entry.file_name());

            if source_path.is_dir() {
                Self::copy_dir_recursive_with_progress(&source_path, &dest_path, progress.clone())?;
            } else {
                // Update current file name
                if let Some(progress_info) = &progress {
                    let mut prog = progress_info.lock().unwrap();
                    prog.current_file = source_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                }
                
                Self::copy_single_file_with_progress(&source_path, &dest_path, progress.clone())?;
            }
        }

        Ok(())
    }

    pub fn move_file(source: &Path, destination: &Path) -> io::Result<()> {
        // Try rename first (faster for same filesystem)
        if fs::rename(source, destination).is_ok() {
            return Ok(());
        }

        // Fallback to copy + delete
        Self::copy_file(source, destination)?;
        if source.is_dir() {
            fs::remove_dir_all(source)?;
        } else {
            fs::remove_file(source)?;
        }

        Ok(())
    }

    // Async move with progress reporting
    pub fn move_file_with_progress(
        source: &Path,
        destination: &Path,
        progress: Option<Arc<Mutex<ProgressInfo>>>,
    ) -> io::Result<()> {
        // Try rename first (faster for same filesystem)
        if fs::rename(source, destination).is_ok() {
            // Update progress for rename (instant)
            if let Some(progress_info) = &progress {
                let mut prog = progress_info.lock().unwrap();
                prog.files_copied += 1;
                if source.is_file() {
                    if let Ok(metadata) = source.metadata() {
                        prog.bytes_copied += metadata.len();
                    }
                }
            }
            return Ok(());
        }

        // Fallback to copy + delete
        Self::copy_file_with_progress(source, destination, progress)?;
        if source.is_dir() {
            fs::remove_dir_all(source)?;
        } else {
            fs::remove_file(source)?;
        }

        Ok(())
    }

    pub fn delete(path: &Path) -> Result<(), trash::Error> {
        trash::delete(path)
    }

    pub fn rename(path: &Path, new_name: &str) -> io::Result<()> {
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "No parent directory")
        })?;
        let new_path = parent.join(new_name);
        fs::rename(path, new_path)
    }

    pub fn create_directory(path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    pub fn create_file(path: &Path) -> io::Result<()> {
        fs::File::create(path)?;
        Ok(())
    }
}
