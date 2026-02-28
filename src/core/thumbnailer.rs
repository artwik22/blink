use std::path::Path;
use std::process::Command;
use std::fs;

pub struct Thumbnailer;

impl Thumbnailer {
    pub fn get_thumbnail_bytes(path: &Path, size: i32) -> Option<Vec<u8>> {
        if Self::is_image(path) {
            Self::get_image_bytes(path, size)
        } else if Self::is_video(path) {
            Self::get_video_bytes(path, size)
        } else {
            None
        }
    }

    fn is_image(path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico")
    }

    fn is_video(path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv")
    }

    fn get_image_bytes(path: &Path, _size: i32) -> Option<Vec<u8>> {
        // Just read the file for now, GTK can handle the resizing on the main thread
        // Or we could use the 'image' crate to resize here if needed for performance.
        // For simplicity and since GTK 4 is good at this, let's just send the bytes.
        fs::read(path).ok()
    }

    fn get_video_bytes(path: &Path, size: i32) -> Option<Vec<u8>> {
        // Try using ffmpegthumbnailer
        let temp_thumb = format!("/tmp/blink_thumb_{}.jpg", 
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let temp_path = Path::new(&temp_thumb);

        let status = Command::new("ffmpegthumbnailer")
            .arg("-i").arg(path)
            .arg("-o").arg(&temp_thumb)
            .arg("-s").arg(size.to_string())
            .status();

        let bytes = if let Ok(s) = status {
            if s.success() {
                fs::read(&temp_thumb).ok()
            } else {
                None
            }
        } else {
            None
        };

        // Clean up
        if temp_path.exists() {
            let _ = fs::remove_file(temp_path);
        }

        bytes
    }
}
