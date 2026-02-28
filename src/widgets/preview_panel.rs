use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Image, Label, Orientation, Picture, ScrolledWindow, Separator};
use std::path::Path;
use std::fs;

#[derive(Clone)]
pub struct PreviewPanel {
    container: GtkBox,
    stack: gtk4::Stack,
    image_picture: Picture,
    text_view: gtk4::TextView,
    text_scroll: ScrolledWindow,
    info_box: GtkBox,
    // Info labels
    name_label: Label,
    type_label: Label,
    size_label: Label,
    modified_label: Label,
    permissions_label: Label,
    location_label: Label,
}

impl PreviewPanel {
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .width_request(280)
            .css_classes(["preview-panel"])
            .build();

        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(150);

        // ===== Image Preview =====
        let image_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        let image_picture = Picture::builder()
            .can_shrink(true)
            .content_fit(gtk4::ContentFit::Contain)
            .css_classes(["preview-image"])
            .build();
        image_picture.set_size_request(248, 200);
        
        image_box.append(&image_picture);

        // Info section below image
        let image_info_box = Self::create_info_section();
        image_box.append(&image_info_box.0);

        // ===== Text Preview =====
        let text_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        let text_view = gtk4::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .monospace(true)
            .css_classes(["preview-text"])
            .build();
        text_view.set_left_margin(8);
        text_view.set_right_margin(8);
        text_view.set_top_margin(8);
        text_view.set_bottom_margin(8);

        let text_scroll = ScrolledWindow::builder()
            .vexpand(true)
            .min_content_height(150)
            .max_content_height(300)
            .css_classes(["preview-text-scroll"])
            .child(&text_view)
            .build();

        text_box.append(&text_scroll);

        let text_info_box = Self::create_info_section();
        text_box.append(&text_info_box.0);

        // ===== Generic File Info =====
        let info_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();

        // Large icon for generic files
        let generic_icon = Image::builder()
            .pixel_size(96)
            .halign(gtk4::Align::Center)
            .margin_top(20)
            .margin_bottom(20)
            .build();
        generic_icon.set_icon_name(Some("text-x-generic"));
        
        info_box.append(&generic_icon);

        let generic_info_box = Self::create_info_section();
        info_box.append(&generic_info_box.0);

        // ===== Empty state =====
        let empty_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .valign(gtk4::Align::Center)
            .vexpand(true)
            .build();
        
        let empty_icon = Image::builder()
            .icon_name("document-properties-symbolic")
            .pixel_size(48)
            .halign(gtk4::Align::Center)
            .css_classes(["dim-label"])
            .build();
        let empty_label = Label::builder()
            .label("Select a file to preview")
            .halign(gtk4::Align::Center)
            .css_classes(["dim-label"])
            .build();
        empty_box.append(&empty_icon);
        empty_box.append(&empty_label);

        // Add to stack
        stack.add_named(&image_box, Some("image"));
        stack.add_named(&text_box, Some("text"));
        stack.add_named(&info_box, Some("info"));
        stack.add_named(&empty_box, Some("empty"));
        stack.set_visible_child_name("empty");

        // Scroll wrapper for the panel
        let scroll = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&stack)
            .build();

        container.append(&scroll);

        // We'll use the shared info labels from `generic_info_box` as the main ones to update
        // But we need to update all three sets. Simplification: use a single set of labels that
        // we move around? No, GTK widgets can only have one parent. Instead, we'll update all three.
        // For simplicity, store references to the generic info labels; we'll update all via update_file.

        Self {
            container,
            stack,
            image_picture,
            text_view,
            text_scroll,
            info_box,
            name_label: generic_info_box.1,
            type_label: generic_info_box.2,
            size_label: generic_info_box.3,
            modified_label: generic_info_box.4,
            permissions_label: generic_info_box.5,
            location_label: generic_info_box.6,
        }
    }

    /// Create an info section with labels for name, type, size, modified, permissions, location.
    /// Returns (container, name_label, type_label, size_label, modified_label, perm_label, location_label)
    fn create_info_section() -> (GtkBox, Label, Label, Label, Label, Label, Label) {
        let info_container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .build();

        let sep = Separator::new(Orientation::Horizontal);
        sep.add_css_class("preview-separator");
        info_container.append(&sep);

        let name_label = Self::create_info_row(&info_container, "Name");
        let type_label = Self::create_info_row(&info_container, "Type");
        let size_label = Self::create_info_row(&info_container, "Size");
        let modified_label = Self::create_info_row(&info_container, "Modified");
        let permissions_label = Self::create_info_row(&info_container, "Permissions");
        let location_label = Self::create_info_row(&info_container, "Location");

        (info_container, name_label, type_label, size_label, modified_label, permissions_label, location_label)
    }

    fn create_info_row(parent: &GtkBox, title: &str) -> Label {
        let row = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .margin_top(4)
            .build();

        let title_label = Label::builder()
            .label(title)
            .halign(gtk4::Align::Start)
            .css_classes(["caption", "dim-label"])
            .build();

        let value_label = Label::builder()
            .halign(gtk4::Align::Start)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .css_classes(["preview-info-value"])
            .build();

        row.append(&title_label);
        row.append(&value_label);
        parent.append(&row);

        value_label
    }

    pub fn container(&self) -> &GtkBox {
        &self.container
    }

    pub fn update_file(&self, path: &Path) {
        if !path.exists() {
            self.stack.set_visible_child_name("empty");
            return;
        }

        // Update common info labels
        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        self.name_label.set_text(&file_name);
        self.location_label.set_text(&path.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default());

        if let Ok(metadata) = fs::metadata(path) {
            // Size
            let size = if metadata.is_dir() {
                Self::calculate_dir_size(path)
            } else {
                metadata.len()
            };
            self.size_label.set_text(&Self::format_size(size));

            // Modified
            if let Ok(modified) = metadata.modified() {
                let datetime: chrono::DateTime<chrono::Local> = modified.into();
                self.modified_label.set_text(&datetime.format("%Y-%m-%d %H:%M:%S").to_string());
            }

            // Type
            let file_type = if metadata.is_dir() {
                "Folder".to_string()
            } else {
                Self::get_mime_type(&file_name)
            };
            self.type_label.set_text(&file_type);

            // Permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                self.permissions_label.set_text(&Self::format_permissions(mode));
            }
            #[cfg(not(unix))]
            {
                let ro = if metadata.permissions().readonly() { "Read Only" } else { "Read/Write" };
                self.permissions_label.set_text(ro);
            }
        }

        // Determine preview type
        if path.is_dir() {
            // Show folder info
            if let Ok(count) = fs::read_dir(path).map(|entries| entries.count()) {
                self.type_label.set_text(&format!("Folder ({} items)", count));
            }
            self.stack.set_visible_child_name("info");
        } else if Self::is_image(path) {
            // Show image preview
            self.image_picture.set_filename(Some(path));
            self.stack.set_visible_child_name("image");
        } else if Self::is_text_file(path) {
            // Show text preview
            if let Ok(content) = fs::read_to_string(path) {
                let preview: String = content.lines().take(80).collect::<Vec<_>>().join("\n");
                let buffer = self.text_view.buffer();
                buffer.set_text(&preview);
            } else {
                let buffer = self.text_view.buffer();
                buffer.set_text("(Cannot read file)");
            }
            self.stack.set_visible_child_name("text");
        } else if Self::is_video(path) {
            // Show video preview via thumbnailer
            if let Some(bytes) = crate::core::Thumbnailer::get_thumbnail_bytes(path, 248) {
                let loader = gtk4::gdk_pixbuf::PixbufLoader::new();
                if loader.write(&bytes).is_ok() && loader.close().is_ok() {
                    if let Some(pixbuf) = loader.pixbuf() {
                        let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);
                        self.image_picture.set_paintable(Some(&texture));
                        self.stack.set_visible_child_name("image");
                    } else {
                        self.stack.set_visible_child_name("info");
                    }
                } else {
                    self.stack.set_visible_child_name("info");
                }
            } else {
                self.stack.set_visible_child_name("info");
            }
        } else {
            self.stack.set_visible_child_name("info");
        }
    }

    pub fn clear(&self) {
        self.stack.set_visible_child_name("empty");
    }

    fn is_image(path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "tif")
    }

    fn is_text_file(path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(),
            "txt" | "md" | "rst" | "log" | "cfg" | "conf" | "ini" |
            "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "rb" | "php" | "swift" | "kt" |
            "html" | "css" | "scss" | "less" | "xml" | "json" | "yaml" | "yml" | "toml" |
            "sh" | "bash" | "zsh" | "fish" |
            "sql" | "graphql" |
            "makefile" | "cmake" | "dockerfile" |
            "gitignore" | "env" | "editorconfig" |
            "csv" | "tsv"
        )
    }

    fn is_video(path: &Path) -> bool {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(), "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv")
    }

    fn get_mime_type(name: &str) -> String {
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
        match ext.as_str() {
            "png" => "Image (PNG)",
            "jpg" | "jpeg" => "Image (JPEG)",
            "gif" => "Image (GIF)",
            "bmp" => "Image (BMP)",
            "svg" => "Image (SVG)",
            "webp" => "Image (WebP)",
            "pdf" => "Document (PDF)",
            "doc" | "docx" => "Document (Word)",
            "xls" | "xlsx" => "Spreadsheet (Excel)",
            "ppt" | "pptx" => "Presentation (PowerPoint)",
            "odt" => "Document (OpenDocument)",
            "ods" => "Spreadsheet (OpenDocument)",
            "odp" => "Presentation (OpenDocument)",
            "mp3" => "Audio (MP3)",
            "wav" => "Audio (WAV)",
            "flac" => "Audio (FLAC)",
            "ogg" => "Audio (OGG)",
            "m4a" => "Audio (M4A)",
            "mp4" => "Video (MP4)",
            "mkv" => "Video (MKV)",
            "avi" => "Video (AVI)",
            "mov" => "Video (MOV)",
            "webm" => "Video (WebM)",
            "zip" => "Archive (ZIP)",
            "tar" => "Archive (TAR)",
            "gz" => "Archive (GZIP)",
            "rar" => "Archive (RAR)",
            "7z" => "Archive (7-Zip)",
            "rs" => "Source Code (Rust)",
            "py" => "Source Code (Python)",
            "js" => "Source Code (JavaScript)",
            "ts" => "Source Code (TypeScript)",
            "c" => "Source Code (C)",
            "cpp" => "Source Code (C++)",
            "java" => "Source Code (Java)",
            "go" => "Source Code (Go)",
            "html" => "Web Page (HTML)",
            "css" => "Stylesheet (CSS)",
            "json" => "Data (JSON)",
            "yaml" | "yml" => "Data (YAML)",
            "toml" => "Data (TOML)",
            "xml" => "Data (XML)",
            "txt" => "Text File",
            "md" => "Markdown Document",
            "sh" | "bash" => "Shell Script",
            "deb" => "Package (Debian)",
            "rpm" => "Package (RPM)",
            "appimage" => "Application (AppImage)",
            _ => "File",
        }.to_string()
    }

    fn format_size(size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if size >= TB {
            format!("{:.2} TB", size as f64 / TB as f64)
        } else if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.1} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.1} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    fn calculate_dir_size(path: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        // Don't recurse too deeply for performance
                        total += Self::calculate_dir_size(&entry.path());
                    } else {
                        total += metadata.len();
                    }
                }
            }
        }
        total
    }

    #[cfg(unix)]
    fn format_permissions(mode: u32) -> String {
        let chars = [
            if mode & 0o400 != 0 { 'r' } else { '-' },
            if mode & 0o200 != 0 { 'w' } else { '-' },
            if mode & 0o100 != 0 { 'x' } else { '-' },
            if mode & 0o040 != 0 { 'r' } else { '-' },
            if mode & 0o020 != 0 { 'w' } else { '-' },
            if mode & 0o010 != 0 { 'x' } else { '-' },
            if mode & 0o004 != 0 { 'r' } else { '-' },
            if mode & 0o002 != 0 { 'w' } else { '-' },
            if mode & 0o001 != 0 { 'x' } else { '-' },
        ];
        let perm_str: String = chars.iter().collect();
        format!("{} ({:o})", perm_str, mode & 0o777)
    }
}

impl Default for PreviewPanel {
    fn default() -> Self {
        Self::new()
    }
}
