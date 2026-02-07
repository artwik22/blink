use gtk4::glib::{self, Object};
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{
    gio, CustomFilter, DragSource, DropTarget, EventControllerKey, FilterListModel, GestureClick, GridView, Label, 
    ListItem, ListView, MultiSelection, PopoverMenu, SignalListItemFactory, Stack,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::fs::OpenOptions;
use std::io::Write;
use async_channel;

use crate::core::{FileEntry, FileOperations, Scanner};

// #region agent log
fn debug_log(hypothesis_id: &str, location: &str, message: &str, data: serde_json::Value) {
    let log_entry = serde_json::json!({
        "sessionId": "debug-session",
        "runId": "run1",
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/artwik/.config/alloy/.cursor/debug.log") {
        let _ = writeln!(file, "{}", log_entry);
    }
}
// #endregion

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Grid,
    List,
}

mod imp {
    use gtk4::glib;
    use gtk4::glib::Object;
    use gtk4::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct FileObject {
        pub name: RefCell<String>,
        pub path: RefCell<String>,
        pub is_directory: RefCell<bool>,
        pub size: RefCell<String>,
        pub modified: RefCell<String>,
        pub icon_name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileObject {
        const NAME: &'static str = "NautilusFileObject";
        type Type = super::FileObject;
        type ParentType = Object;
    }

    impl ObjectImpl for FileObject {}
}

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

impl FileObject {
    pub fn new(entry: &FileEntry) -> Self {
        let obj: Self = Object::builder().build();
        *obj.imp().name.borrow_mut() = entry.name.clone();
        *obj.imp().path.borrow_mut() = entry.path.to_string_lossy().to_string();
        *obj.imp().is_directory.borrow_mut() = entry.is_directory;
        *obj.imp().size.borrow_mut() = entry.size_display();
        *obj.imp().modified.borrow_mut() = entry.modified.clone();
        *obj.imp().icon_name.borrow_mut() = Self::get_nautilus_icon(&entry.name, entry.is_directory);
        obj
    }

    fn get_nautilus_icon(name: &str, is_directory: bool) -> String {
        if is_directory {
            // Use themed folder icons for special directories
            let name_lower = name.to_lowercase();
            match name_lower.as_str() {
                "documents" => "folder-documents",
                "downloads" => "folder-download",
                "music" => "folder-music",
                "pictures" => "folder-pictures",
                "videos" => "folder-videos",
                "desktop" => "user-desktop",
                "templates" => "folder-templates",
                "public" => "folder-publicshare",
                _ => "folder",
            }.to_string()
        } else {
            // File icons based on extension
            let extension = name.rsplit('.').next().unwrap_or("").to_lowercase();
            match extension.as_str() {
                // Documents
                "pdf" => "application-pdf",
                "doc" | "docx" | "odt" => "x-office-document",
                "xls" | "xlsx" | "ods" => "x-office-spreadsheet",
                "ppt" | "pptx" | "odp" => "x-office-presentation",
                "txt" | "md" | "rst" => "text-x-generic",
                
                // Images
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" | "ico" => "image-x-generic",
                
                // Audio
                "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => "audio-x-generic",
                
                // Video
                "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" => "video-x-generic",
                
                // Archives
                "zip" | "tar" | "gz" | "bz2" | "xz" | "rar" | "7z" => "application-x-archive",
                
                // Code
                "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "rb" | "php" => "text-x-script",
                "html" | "css" | "xml" | "json" | "yaml" | "yml" | "toml" => "text-x-generic",
                
                // Executables
                "sh" | "bash" => "application-x-executable",
                "exe" | "msi" => "application-x-ms-dos-executable",
                "deb" | "rpm" | "appimage" => "application-x-executable",
                
                _ => "text-x-generic",
            }.to_string()
        }
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(self.imp().path.borrow().clone())
    }

    pub fn is_directory(&self) -> bool {
        *self.imp().is_directory.borrow()
    }

    pub fn size(&self) -> String {
        self.imp().size.borrow().clone()
    }

    pub fn modified(&self) -> String {
        self.imp().modified.borrow().clone()
    }

    pub fn icon_name(&self) -> String {
        self.imp().icon_name.borrow().clone()
    }
}

#[derive(Clone)]
pub struct FileGridView {
    container: gtk4::Box,
    stack: Stack,
    list_view: ListView,
    grid_view: GridView,
    store: gio::ListStore,
    filter: CustomFilter,
    selection: MultiSelection,
    current_path: Rc<RefCell<PathBuf>>,
    all_entries: Rc<RefCell<Vec<FileEntry>>>,
    show_hidden: Rc<RefCell<bool>>,
    view_mode: Rc<RefCell<ViewMode>>,

    on_directory_activated: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    on_copy: Rc<RefCell<Option<Box<dyn Fn(Vec<PathBuf>)>>>>,
    on_cut: Rc<RefCell<Option<Box<dyn Fn(Vec<PathBuf>)>>>>,
    on_paste: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    on_delete: Rc<RefCell<Option<Box<dyn Fn(Vec<PathBuf>)>>>>,
    on_rename: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    on_pin: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    on_open_terminal: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    on_open_micro: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    current_scan_id: Rc<RefCell<u64>>,
}

impl FileGridView {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["nautilus-view"])
            .build();

        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(150);

        let store = gio::ListStore::new::<FileObject>();
        let filter = CustomFilter::new(|_| true);
        let filter_model = FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        let selection = MultiSelection::new(Some(filter_model));

        // Prepare on_pin callback for use in context menus
        let on_pin: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));

        // ===== GRID VIEW (Nautilus-style) =====
        let grid_factory = SignalListItemFactory::new();

        grid_factory.connect_setup(|_, item| {
            let item = item.downcast_ref::<ListItem>().unwrap();

            // Nautilus-style tile: vertical box with large icon and label
            let tile = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(6)
                .halign(gtk4::Align::Center)
                .valign(gtk4::Align::Start)
                .width_request(96)
                .height_request(96)
                .css_classes(["nautilus-tile"])
                .build();

            // Large icon (64px like Nautilus)
            let icon = gtk4::Image::builder()
                .pixel_size(64)
                .halign(gtk4::Align::Center)
                .css_classes(["nautilus-icon"])
                .build();

            // File name label
            let name_label = Label::builder()
                .halign(gtk4::Align::Center)
                .justify(gtk4::Justification::Center)
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::WordChar)
                .max_width_chars(12)
                .ellipsize(gtk4::pango::EllipsizeMode::Middle)
                .lines(2)
                .css_classes(["nautilus-label"])
                .build();

            tile.append(&icon);
            tile.append(&name_label);
            item.set_child(Some(&tile));
        });

        grid_factory.connect_bind(|_, item| {
            let item = item.downcast_ref::<ListItem>().unwrap();
            let file_obj = item.item().and_downcast::<FileObject>().unwrap();

            let tile = item.child().and_downcast::<gtk4::Box>().unwrap();
            let icon = tile.first_child().and_downcast::<gtk4::Image>().unwrap();
            let name_label = icon.next_sibling().and_downcast::<Label>().unwrap();

            icon.set_icon_name(Some(&file_obj.icon_name()));
            name_label.set_text(&file_obj.name());
        });

        let grid_view = GridView::builder()
            .model(&selection)
            .factory(&grid_factory)
            .min_columns(2)
            .max_columns(50)
            .css_classes(["nautilus-grid"])
            .build();

        // ===== LIST VIEW =====
        let list_factory = SignalListItemFactory::new();

        list_factory.connect_setup(|_, item| {
            let item = item.downcast_ref::<ListItem>().unwrap();

            let hbox = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .margin_start(12)
                .margin_end(12)
                .margin_top(6)
                .margin_bottom(6)
                .css_classes(["nautilus-list-row"])
                .build();

            let icon = gtk4::Image::builder()
                .pixel_size(32)
                .css_classes(["nautilus-list-icon"])
                .build();

            let name_label = Label::builder()
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .css_classes(["nautilus-list-name"])
                .build();

            let size_label = Label::builder()
                .halign(gtk4::Align::End)
                .width_chars(10)
                .css_classes(["dim-label", "nautilus-list-size"])
                .build();

            let date_label = Label::builder()
                .halign(gtk4::Align::End)
                .width_chars(16)
                .css_classes(["dim-label", "nautilus-list-date"])
                .build();

            hbox.append(&icon);
            hbox.append(&name_label);
            hbox.append(&size_label);
            hbox.append(&date_label);

            item.set_child(Some(&hbox));
        });

        list_factory.connect_bind(|_, item| {
            let item = item.downcast_ref::<ListItem>().unwrap();
            let file_obj = item.item().and_downcast::<FileObject>().unwrap();

            let hbox = item.child().and_downcast::<gtk4::Box>().unwrap();
            let icon = hbox.first_child().and_downcast::<gtk4::Image>().unwrap();
            let name_label = icon.next_sibling().and_downcast::<Label>().unwrap();
            let size_label = name_label.next_sibling().and_downcast::<Label>().unwrap();
            let date_label = size_label.next_sibling().and_downcast::<Label>().unwrap();

            icon.set_icon_name(Some(&file_obj.icon_name()));
            name_label.set_text(&file_obj.name());
            size_label.set_text(&file_obj.size());
            date_label.set_text(&file_obj.modified());
        });

        let list_view = ListView::builder()
            .model(&selection)
            .factory(&list_factory)
            .css_classes(["nautilus-list"])
            .build();

        // Add views to stack
        stack.add_named(&grid_view, Some("grid"));
        stack.add_named(&list_view, Some("list"));
        stack.set_visible_child_name("grid");

        container.append(&stack);

        let current_path = Rc::new(RefCell::new(PathBuf::new()));
        let all_entries = Rc::new(RefCell::new(Vec::new()));
        let show_hidden = Rc::new(RefCell::new(false));
        let view_mode = Rc::new(RefCell::new(ViewMode::Grid));

        let on_directory_activated: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        let on_copy: Rc<RefCell<Option<Box<dyn Fn(Vec<PathBuf>)>>>> = Rc::new(RefCell::new(None));
        let on_cut: Rc<RefCell<Option<Box<dyn Fn(Vec<PathBuf>)>>>> = Rc::new(RefCell::new(None));
        let on_paste: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let on_delete: Rc<RefCell<Option<Box<dyn Fn(Vec<PathBuf>)>>>> = Rc::new(RefCell::new(None));
        let on_rename: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        // on_pin is already created above for use in factories
        let on_open_terminal: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        let on_open_micro: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        let current_scan_id = Rc::new(RefCell::new(0u64));

        // Keyboard shortcuts for Grid and List views
        {
            let selection_clone = selection.clone();
            let on_delete_clone = on_delete.clone();
            let on_copy_clone = on_copy.clone();
            let on_cut_clone = on_cut.clone();
            let on_paste_clone = on_paste.clone();
            let on_rename_clone = on_rename.clone();
            let key_controller = EventControllerKey::new();
            
            key_controller.connect_key_pressed(move |_, key, _keycode, state| {
                // Helper to get selected paths
                let get_selected_paths = || {
                    let mut selected_paths = Vec::new();
                    let n_items = selection_clone.n_items();
                    for i in 0..n_items {
                        if selection_clone.is_selected(i) {
                            if let Some(item) = selection_clone.item(i) {
                                if let Ok(file_obj) = item.downcast::<FileObject>() {
                                    selected_paths.push(file_obj.path());
                                }
                            }
                        }
                    }
                    selected_paths
                };
                
                // Delete key
                if key == gtk4::gdk::Key::Delete {
                    let selected_paths = get_selected_paths();
                    if !selected_paths.is_empty() {
                        if let Some(ref callback) = *on_delete_clone.borrow() {
                            callback(selected_paths);
                            return glib::Propagation::Stop;
                        }
                    }
                }
                
                // F2 - Rename
                if key == gtk4::gdk::Key::F2 {
                    let selected_paths = get_selected_paths();
                    if selected_paths.len() == 1 {
                        if let Some(ref callback) = *on_rename_clone.borrow() {
                            callback(selected_paths[0].clone());
                            return glib::Propagation::Stop;
                        }
                    }
                }
                
                // Ctrl+C - Copy
                if key == gtk4::gdk::Key::c && state.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    let selected_paths = get_selected_paths();
                    if !selected_paths.is_empty() {
                        if let Some(ref callback) = *on_copy_clone.borrow() {
                            callback(selected_paths);
                            return glib::Propagation::Stop;
                        }
                    }
                }
                
                // Ctrl+X - Cut
                if key == gtk4::gdk::Key::x && state.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    let selected_paths = get_selected_paths();
                    if !selected_paths.is_empty() {
                        if let Some(ref callback) = *on_cut_clone.borrow() {
                            callback(selected_paths);
                            return glib::Propagation::Stop;
                        }
                    }
                }
                
                // Ctrl+V - Paste
                if key == gtk4::gdk::Key::v && state.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                    if let Some(ref callback) = *on_paste_clone.borrow() {
                        callback();
                        return glib::Propagation::Stop;
                    }
                }
                
                glib::Propagation::Proceed
            });
            
            grid_view.add_controller(key_controller.clone());
            list_view.add_controller(key_controller);
        }

        // Double-click activation for GRID VIEW
        {
            let on_directory_activated_clone = on_directory_activated.clone();
            let selection_clone = selection.clone();

            grid_view.connect_activate(move |_, position| {
                if let Some(item) = selection_clone.item(position) {
                    let file_obj = item.downcast::<FileObject>().unwrap();
                    if file_obj.is_directory() {
                        if let Some(ref callback) = *on_directory_activated_clone.borrow() {
                            callback(file_obj.path());
                        }
                    } else {
                        // Open file with default application
                        if let Err(e) = open::that(&file_obj.path()) {
                            eprintln!("Failed to open file: {}", e);
                        }
                    }
                }
            });
        }

        // Double-click activation for LIST VIEW
        {
            let on_directory_activated_clone = on_directory_activated.clone();
            let selection_clone = selection.clone();

            list_view.connect_activate(move |_, position| {
                if let Some(item) = selection_clone.item(position) {
                    let file_obj = item.downcast::<FileObject>().unwrap();
                    if file_obj.is_directory() {
                        if let Some(ref callback) = *on_directory_activated_clone.borrow() {
                            callback(file_obj.path());
                        }
                    } else {
                        if let Err(e) = open::that(&file_obj.path()) {
                            eprintln!("Failed to open file: {}", e);
                        }
                    }
                }
            });
        }

        // Context menu for GRID VIEW - using GMenu with app.toggle-pin action
        {
            let on_copy_clone = on_copy.clone();
            let on_cut_clone = on_cut.clone();
            let on_paste_clone = on_paste.clone();
            let on_delete_clone = on_delete.clone();
            let on_rename_clone = on_rename.clone();
            let selection_clone = selection.clone();
            let grid_view_clone = grid_view.clone();
            let current_popover: Rc<RefCell<Option<PopoverMenu>>> = Rc::new(RefCell::new(None));

            let gesture = GestureClick::builder().button(3).build();

            gesture.connect_pressed(move |_, _, x, y| {
                // Close any existing popover first
                if let Some(ref mut popover) = *current_popover.borrow_mut() {
                    popover.popdown();
                    popover.unparent();
                }
                current_popover.borrow_mut().take();

                let mut selected_paths = Vec::new();
                let n_items = selection_clone.n_items();
                for i in 0..n_items {
                    if selection_clone.is_selected(i) {
                        if let Some(item) = selection_clone.item(i) {
                            if let Ok(file_obj) = item.downcast::<FileObject>() {
                                selected_paths.push(file_obj.path());
                            }
                        }
                    }
                }

                // If no items selected, select item at click position
                if selected_paths.is_empty() && n_items > 0 {
                    if let Some(item) = selection_clone.item(0) {
                        if let Ok(file_obj) = item.downcast::<FileObject>() {
                            selected_paths.push(file_obj.path());
                        }
                    }
                }

                // Build menu using gio::Menu
                let menu = gio::Menu::new();
                
                if !selected_paths.is_empty() {
                    // File section
                    let file_section = gio::Menu::new();
                    file_section.append(Some("Open"), Some("file.open"));
                    if selected_paths.len() == 1 {
                        file_section.append(Some("Rename…"), Some("file.rename"));
                    }
                    menu.append_section(None, &file_section);
                    
                    // Edit section
                    let edit_section = gio::Menu::new();
                    edit_section.append(Some("Copy"), Some("file.copy"));
                    edit_section.append(Some("Cut"), Some("file.cut"));
                    menu.append_section(None, &edit_section);
                    
                    // Delete section
                    let delete_section = gio::Menu::new();
                    delete_section.append(Some("Move to Trash"), Some("file.delete"));
                    menu.append_section(None, &delete_section);
                } else {
                    let paste_section = gio::Menu::new();
                    paste_section.append(Some("Paste"), Some("file.paste"));
                    menu.append_section(None, &paste_section);
                }

                // Create action group for file-specific actions
                let action_group = gio::SimpleActionGroup::new();

                // Paste action
                {
                    let on_paste = on_paste_clone.clone();
                    let action = gio::SimpleAction::new("paste", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_paste.borrow() {
                            callback();
                        }
                    });
                    action_group.add_action(&action);
                }

                // Open action
                {
                    let paths = selected_paths.clone();
                    let action = gio::SimpleAction::new("open", None);
                    action.connect_activate(move |_, _| {
                        for path in &paths {
                            if let Err(e) = open::that(path) {
                                eprintln!("Failed to open: {}", e);
                            }
                        }
                    });
                    action_group.add_action(&action);
                }

                // Copy action
                {
                    let paths = selected_paths.clone();
                    let on_copy = on_copy_clone.clone();
                    let action = gio::SimpleAction::new("copy", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_copy.borrow() {
                            callback(paths.clone());
                        }
                    });
                    action_group.add_action(&action);
                }

                // Cut action
                {
                    let paths = selected_paths.clone();
                    let on_cut = on_cut_clone.clone();
                    let action = gio::SimpleAction::new("cut", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_cut.borrow() {
                            callback(paths.clone());
                        }
                    });
                    action_group.add_action(&action);
                }

                // Delete action
                {
                    let paths = selected_paths.clone();
                    let on_delete = on_delete_clone.clone();
                    let action = gio::SimpleAction::new("delete", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_delete.borrow() {
                            callback(paths.clone());
                        }
                    });
                    action_group.add_action(&action);
                }

                // Rename action
                {
                    let paths = selected_paths.clone();
                    let on_rename = on_rename_clone.clone();
                    let action = gio::SimpleAction::new("rename", None);
                    action.connect_activate(move |_, _| {
                        if let Some(path) = paths.first() {
                            if let Some(ref callback) = *on_rename.borrow() {
                                callback(path.clone());
                            }
                        }
                    });
                    action_group.add_action(&action);
                }

                // Create popover and attach action group
                let popover = PopoverMenu::from_model(Some(&menu));
                
                if grid_view_clone.parent().is_some() {
                    popover.set_parent(&grid_view_clone);
                }
                popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                
                popover.insert_action_group("file", Some(&action_group));
                
                let popover_clone = popover.clone();
                let current_popover_clone = current_popover.clone();
                popover.connect_closed(move |p: &PopoverMenu| {
                    p.unparent();
                    current_popover_clone.borrow_mut().take();
                });
                
                *current_popover.borrow_mut() = Some(popover_clone.clone());
                popover_clone.popup();
            });

            grid_view.add_controller(gesture);
        }

        // Context menu for LIST VIEW - using GMenu with app.toggle-pin action
        {
            let on_copy_clone = on_copy.clone();
            let on_cut_clone = on_cut.clone();
            let on_paste_clone = on_paste.clone();
            let on_delete_clone = on_delete.clone();
            let on_rename_clone = on_rename.clone();
            let selection_clone = selection.clone();
            let list_view_clone = list_view.clone();
            let current_popover: Rc<RefCell<Option<PopoverMenu>>> = Rc::new(RefCell::new(None));

            let gesture = GestureClick::builder().button(3).build();

            gesture.connect_pressed(move |_, _, x, y| {
                // Close any existing popover first
                if let Some(ref mut popover) = *current_popover.borrow_mut() {
                    popover.popdown();
                    popover.unparent();
                }
                current_popover.borrow_mut().take();

                let mut selected_paths = Vec::new();
                let n_items = selection_clone.n_items();
                for i in 0..n_items {
                    if selection_clone.is_selected(i) {
                        if let Some(item) = selection_clone.item(i) {
                            if let Ok(file_obj) = item.downcast::<FileObject>() {
                                selected_paths.push(file_obj.path());
                            }
                        }
                    }
                }

                if selected_paths.is_empty() && n_items > 0 {
                    if let Some(item) = selection_clone.item(0) {
                        if let Ok(file_obj) = item.downcast::<FileObject>() {
                            selected_paths.push(file_obj.path());
                        }
                    }
                }

                // Build menu using gio::Menu
                let menu = gio::Menu::new();
                
                if !selected_paths.is_empty() {
                    let file_section = gio::Menu::new();
                    file_section.append(Some("Open"), Some("file.open"));
                    if selected_paths.len() == 1 {
                        file_section.append(Some("Rename…"), Some("file.rename"));
                    }
                    menu.append_section(None, &file_section);
                    
                    let edit_section = gio::Menu::new();
                    edit_section.append(Some("Copy"), Some("file.copy"));
                    edit_section.append(Some("Cut"), Some("file.cut"));
                    menu.append_section(None, &edit_section);
                    
                    let delete_section = gio::Menu::new();
                    delete_section.append(Some("Move to Trash"), Some("file.delete"));
                    menu.append_section(None, &delete_section);
                } else {
                    let paste_section = gio::Menu::new();
                    paste_section.append(Some("Paste"), Some("file.paste"));
                    menu.append_section(None, &paste_section);
                }

                // Create action group
                let action_group = gio::SimpleActionGroup::new();

                {
                    let on_paste = on_paste_clone.clone();
                    let action = gio::SimpleAction::new("paste", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_paste.borrow() {
                            callback();
                        }
                    });
                    action_group.add_action(&action);
                }

                {
                    let paths = selected_paths.clone();
                    let action = gio::SimpleAction::new("open", None);
                    action.connect_activate(move |_, _| {
                        for path in &paths {
                            let _ = open::that(path);
                        }
                    });
                    action_group.add_action(&action);
                }

                {
                    let paths = selected_paths.clone();
                    let on_copy = on_copy_clone.clone();
                    let action = gio::SimpleAction::new("copy", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_copy.borrow() {
                            callback(paths.clone());
                        }
                    });
                    action_group.add_action(&action);
                }

                {
                    let paths = selected_paths.clone();
                    let on_cut = on_cut_clone.clone();
                    let action = gio::SimpleAction::new("cut", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_cut.borrow() {
                            callback(paths.clone());
                        }
                    });
                    action_group.add_action(&action);
                }

                {
                    let paths = selected_paths.clone();
                    let on_delete = on_delete_clone.clone();
                    let action = gio::SimpleAction::new("delete", None);
                    action.connect_activate(move |_, _| {
                        if let Some(ref callback) = *on_delete.borrow() {
                            callback(paths.clone());
                        }
                    });
                    action_group.add_action(&action);
                }

                {
                    let paths = selected_paths.clone();
                    let on_rename = on_rename_clone.clone();
                    let action = gio::SimpleAction::new("rename", None);
                    action.connect_activate(move |_, _| {
                        if let Some(path) = paths.first() {
                            if let Some(ref callback) = *on_rename.borrow() {
                                callback(path.clone());
                            }
                        }
                    });
                    action_group.add_action(&action);
                }

                let popover = PopoverMenu::from_model(Some(&menu));
                
                if list_view_clone.parent().is_some() {
                    popover.set_parent(&list_view_clone);
                }
                popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                
                popover.insert_action_group("file", Some(&action_group));
                
                let popover_clone = popover.clone();
                let current_popover_clone = current_popover.clone();
                popover.connect_closed(move |p: &PopoverMenu| {
                    p.unparent();
                    current_popover_clone.borrow_mut().take();
                });
                
                *current_popover.borrow_mut() = Some(popover_clone.clone());
                popover_clone.popup();
            });

            list_view.add_controller(gesture);
        }

        // Drag source for GRID VIEW
        {
            let drag_source = DragSource::new();
            drag_source.set_actions(gtk4::gdk::DragAction::COPY | gtk4::gdk::DragAction::MOVE);
            
            let selection_clone = selection.clone();
            drag_source.connect_prepare(move |_, _, _| {
                let mut selected_items = Vec::new();
                let n_items = selection_clone.n_items();
                for i in 0..n_items {
                    if selection_clone.is_selected(i) {
                        if let Some(item) = selection_clone.item(i) {
                            if let Ok(file_obj) = item.downcast::<FileObject>() {
                                selected_items.push(file_obj);
                            }
                        }
                    }
                }
                
                if !selected_items.is_empty() {
                    let gfiles: Vec<gio::File> = selected_items
                        .iter()
                        .map(|obj| gio::File::for_path(&obj.path()))
                        .collect();
                    
                    let file_list = gtk4::gdk::FileList::from_array(&gfiles);
                    let content = gtk4::gdk::ContentProvider::for_value(&file_list.to_value());
                    Some(content)
                } else {
                    None
                }
            });
            
            grid_view.add_controller(drag_source);
        }

        // Drag source for LIST VIEW
        {
            let drag_source = DragSource::new();
            drag_source.set_actions(gtk4::gdk::DragAction::COPY | gtk4::gdk::DragAction::MOVE);
            
            let selection_clone = selection.clone();
            drag_source.connect_prepare(move |_, _, _| {
                let mut selected_items = Vec::new();
                let n_items = selection_clone.n_items();
                for i in 0..n_items {
                    if selection_clone.is_selected(i) {
                        if let Some(item) = selection_clone.item(i) {
                            if let Ok(file_obj) = item.downcast::<FileObject>() {
                                selected_items.push(file_obj);
                            }
                        }
                    }
                }
                
                if !selected_items.is_empty() {
                    let gfiles: Vec<gio::File> = selected_items
                        .iter()
                        .map(|obj| gio::File::for_path(&obj.path()))
                        .collect();
                    
                    let file_list = gtk4::gdk::FileList::from_array(&gfiles);
                    let content = gtk4::gdk::ContentProvider::for_value(&file_list.to_value());
                    Some(content)
                } else {
                    None
                }
            });
            
            list_view.add_controller(drag_source);
        }

        // Drop target for GRID VIEW
        {
            let current_path_clone = current_path.clone();
            let store_clone = store.clone();
            let show_hidden_clone = show_hidden.clone();
            
            let drop_target = DropTarget::new(
                gtk4::gdk::FileList::static_type(), 
                gtk4::gdk::DragAction::COPY | gtk4::gdk::DragAction::MOVE
            );
            
            drop_target.connect_drop(move |_, value, _, _| {
                if let Ok(file_list) = value.get::<gtk4::gdk::FileList>() {
                    let files: Vec<PathBuf> = file_list.files()
                        .iter()
                        .filter_map(|f| f.path())
                        .collect();
                    
                    if files.is_empty() {
                        return false;
                    }
                    
                    let dest_dir = current_path_clone.borrow().clone();
                    let show_hidden_val = *show_hidden_clone.borrow();
                    
                    // Perform copy operation in background thread to avoid blocking UI
                    let files_clone = files.clone();
                    let store_clone_final = store_clone.clone();
                    let dest_dir_clone = dest_dir.clone();
                    
                    // Create channel for communication
                    let (tx, rx) = async_channel::unbounded::<()>();
                    
                    // Clone dest_dir for async closure
                    let dest_dir_for_async = dest_dir_clone.clone();
                    
                    // Listen for completion on UI thread
                    glib::spawn_future_local(async move {
                        if rx.recv().await.is_ok() {
                            // Defer rescan to let filesystem settle
                            glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                                if let Ok(entries) = Scanner::scan_with_hidden(&dest_dir_for_async, show_hidden_val) {
                                    store_clone_final.remove_all();
                                    for entry in &entries {
                                        store_clone_final.append(&FileObject::new(entry));
                                    }
                                }
                                glib::ControlFlow::Break
                            });
                        }
                    });
                    
                    std::thread::spawn(move || {
                        for source_file in &files_clone {
                            if let Some(file_name) = source_file.file_name() {
                                let mut dest_path = dest_dir_clone.join(file_name);
                                
                                if source_file == &dest_path {
                                    continue;
                                }
                                
                                let mut counter = 1;
                                while dest_path.exists() {
                                    let stem = source_file.file_stem()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    let extension = source_file.extension()
                                        .map(|e| format!(".{}", e.to_string_lossy()))
                                        .unwrap_or_default();
                                    
                                    let new_name = format!("{} ({}){}", stem, counter, extension);
                                    dest_path = dest_dir_clone.join(new_name);
                                    counter += 1;
                                }
                                
                                if let Err(e) = FileOperations::copy_file(source_file, &dest_path) {
                                    eprintln!("Failed to copy: {}", e);
                                }
                            }
                        }
                        
                        // Signal completion
                        let _ = tx.send_blocking(());
                    });
                    
                    true
                } else {
                    false
                }
            });
            
            grid_view.add_controller(drop_target);
        }

        // Drop target for LIST VIEW
        {
            let current_path_clone = current_path.clone();
            let store_clone = store.clone();
            let show_hidden_clone = show_hidden.clone();
            
            let drop_target = DropTarget::new(
                gtk4::gdk::FileList::static_type(), 
                gtk4::gdk::DragAction::COPY | gtk4::gdk::DragAction::MOVE
            );
            
            drop_target.connect_drop(move |_, value, _, _| {
                if let Ok(file_list) = value.get::<gtk4::gdk::FileList>() {
                    let files: Vec<PathBuf> = file_list.files()
                        .iter()
                        .filter_map(|f| f.path())
                        .collect();
                    
                    if files.is_empty() {
                        return false;
                    }
                    
                    let dest_dir = current_path_clone.borrow().clone();
                    let show_hidden_val = *show_hidden_clone.borrow();
                    
                    // Perform copy operation in background thread to avoid blocking UI
                    let files_clone = files.clone();
                    let store_clone_final = store_clone.clone();
                    let dest_dir_clone = dest_dir.clone();
                    
                    // Create channel for communication
                    let (tx, rx) = async_channel::unbounded::<()>();
                    
                    // Clone dest_dir for async closure
                    let dest_dir_for_async = dest_dir_clone.clone();
                    
                    // Listen for completion on UI thread
                    glib::spawn_future_local(async move {
                        if rx.recv().await.is_ok() {
                            // Defer rescan to let filesystem settle
                            glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                                if let Ok(entries) = Scanner::scan_with_hidden(&dest_dir_for_async, show_hidden_val) {
                                    store_clone_final.remove_all();
                                    for entry in &entries {
                                        store_clone_final.append(&FileObject::new(entry));
                                    }
                                }
                                glib::ControlFlow::Break
                            });
                        }
                    });
                    
                    std::thread::spawn(move || {
                        for source_file in &files_clone {
                            if let Some(file_name) = source_file.file_name() {
                                let mut dest_path = dest_dir_clone.join(file_name);
                                
                                if source_file == &dest_path {
                                    continue;
                                }
                                
                                let mut counter = 1;
                                while dest_path.exists() {
                                    let stem = source_file.file_stem()
                                        .map(|s| s.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    let extension = source_file.extension()
                                        .map(|e| format!(".{}", e.to_string_lossy()))
                                        .unwrap_or_default();
                                    
                                    let new_name = format!("{} ({}){}", stem, counter, extension);
                                    dest_path = dest_dir_clone.join(new_name);
                                    counter += 1;
                                }
                                
                                if let Err(e) = FileOperations::copy_file(source_file, &dest_path) {
                                    eprintln!("Failed to copy: {}", e);
                                }
                            }
                        }
                        
                        // Signal completion
                        let _ = tx.send_blocking(());
                    });
                    
                    true
                } else {
                    false
                }
            });
            
            list_view.add_controller(drop_target);
        }

        // Add keyboard shortcuts for 'f' (terminal) and 'm' (micro)
        {
            let selection_clone = selection.clone();
            let on_open_terminal_clone = on_open_terminal.clone();
            let on_open_micro_clone = on_open_micro.clone();
            let current_path_clone = current_path.clone();

            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, keyval, _, _| {
                // Get selected items
                let mut selected_paths = Vec::new();
                let n_items = selection_clone.n_items();
                for i in 0..n_items {
                    if selection_clone.is_selected(i) {
                        if let Some(item) = selection_clone.item(i) {
                            if let Ok(file_obj) = item.downcast::<FileObject>() {
                                selected_paths.push(file_obj.path());
                            }
                        }
                    }
                }

                // Handle 'f' key - open terminal in directory
                if keyval == gtk4::gdk::Key::f {
                    let target_path = if let Some(first_path) = selected_paths.first() {
                        if first_path.is_dir() {
                            first_path.clone()
                        } else {
                            first_path.parent().unwrap_or_else(|| std::path::Path::new("/")).to_path_buf()
                        }
                    } else {
                        current_path_clone.borrow().clone()
                    };

                    if let Some(ref callback) = *on_open_terminal_clone.borrow() {
                        callback(target_path);
                    }
                    return glib::Propagation::Stop;
                }

                // Handle 'm' key - open file in micro
                if keyval == gtk4::gdk::Key::m {
                    if let Some(first_path) = selected_paths.first() {
                        if !first_path.is_dir() {
                            if let Some(ref callback) = *on_open_micro_clone.borrow() {
                                callback(first_path.clone());
                            }
                        }
                    }
                    return glib::Propagation::Stop;
                }

                // Handle 'h' key - open terminal with cd to current path
                if keyval == gtk4::gdk::Key::h {
                    let target_path = current_path_clone.borrow().clone();
                    if let Some(ref callback) = *on_open_terminal_clone.borrow() {
                        callback(target_path);
                    }
                    return glib::Propagation::Stop;
                }

                glib::Propagation::Proceed
            });

            // Add controller to both views
            grid_view.add_controller(key_controller.clone());
            list_view.add_controller(key_controller);
        }

        Self {
            container,
            stack,
            list_view,
            grid_view,
            store,
            filter,
            selection,
            current_path,
            all_entries,
            show_hidden,
            view_mode,
            on_directory_activated,
            on_copy,
            on_cut,
            on_paste,
            on_delete,
            on_rename,
            on_pin,
            on_open_terminal,
            on_open_micro,
            current_scan_id,
        }
    }

    pub fn container(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn load_directory(&self, path: &Path) {
        // #region agent log
        debug_log("E", "file_view.rs:load_directory", "Function entry", serde_json::json!({
            "path": path.to_string_lossy(),
            "exists": path.exists(),
            "is_dir": path.is_dir()
        }));
        // #endregion
        
        self.selection.unselect_all();
        self.current_path.replace(path.to_path_buf());
        self.store.remove_all();

        // Increment scan ID to ignore previous pending scans
        let mut scan_id_guard = self.current_scan_id.borrow_mut();
        *scan_id_guard += 1;
        let scan_id = *scan_id_guard;
        drop(scan_id_guard);

        let path = path.to_path_buf();
        let show_hidden = *self.show_hidden.borrow();
        
        let (tx, rx) = async_channel::unbounded::<Result<Vec<FileEntry>, std::io::Error>>();
        
        let store = self.store.clone();
        let all_entries = self.all_entries.clone();
        let current_scan_id = self.current_scan_id.clone();
        
        // Spawn background thread for scanning
        std::thread::spawn(move || {
            let result = Scanner::scan_with_hidden(&path, show_hidden);
            let _ = tx.send_blocking(result);
        });
        
        // Receive result on UI thread
        glib::spawn_future_local(async move {
            if let Ok(result) = rx.recv().await {
                // Only process if this is still the latest scan
                if *current_scan_id.borrow() == scan_id {
                    match result {
                        Ok(entries) => {
                            // #region agent log
                            debug_log("E", "file_view.rs:load_directory", "Scan successful (async)", serde_json::json!({
                                "entry_count": entries.len()
                            }));
                            // #endregion
                            
                            all_entries.replace(entries.clone());
                            for entry in &entries {
                                store.append(&FileObject::new(entry));
                            }
                        }
                        Err(e) => {
                            // #region agent log
                            debug_log("E", "file_view.rs:load_directory", "Scan failed (async)", serde_json::json!({
                                "error": e.to_string()
                            }));
                            // #endregion
                            eprintln!("Failed to scan directory: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub fn refresh(&self) {
        let current = self.current_path.borrow().clone();
        self.load_directory(&current);
    }

    pub fn filter(&self, query: &str) {
        let query = query.to_lowercase();
        self.filter.set_filter_func(move |obj| {
            if query.is_empty() {
                return true;
            }
            let file_obj = obj.downcast_ref::<FileObject>().unwrap();
            file_obj.name().to_lowercase().contains(&query)
        });
    }

    pub fn toggle_view_mode(&self) {
        let current = *self.view_mode.borrow();
        let new_mode = match current {
            ViewMode::Grid => ViewMode::List,
            ViewMode::List => ViewMode::Grid,
        };
        self.view_mode.replace(new_mode);
        match new_mode {
            ViewMode::Grid => self.stack.set_visible_child_name("grid"),
            ViewMode::List => self.stack.set_visible_child_name("list"),
        }
    }

    pub fn is_grid_mode(&self) -> bool {
        *self.view_mode.borrow() == ViewMode::Grid
    }

    pub fn set_show_hidden(&self, show_hidden: bool) {
        *self.show_hidden.borrow_mut() = show_hidden;
    }

    pub fn connect_directory_activated<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_directory_activated.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_copy<F: Fn(Vec<PathBuf>) + 'static>(&self, callback: F) {
        *self.on_copy.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_cut<F: Fn(Vec<PathBuf>) + 'static>(&self, callback: F) {
        *self.on_cut.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_paste<F: Fn() + 'static>(&self, callback: F) {
        *self.on_paste.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_delete<F: Fn(Vec<PathBuf>) + 'static>(&self, callback: F) {
        *self.on_delete.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_rename<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_rename.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_pin<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_pin.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_open_terminal<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_open_terminal.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_open_micro<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_open_micro.borrow_mut() = Some(Box::new(callback));
    }

    pub fn rename_selected(&self) {
        // Helper to get selected paths (logic duplicated closely from key controller)
        let mut selected_paths = Vec::new();
        let n_items = self.selection.n_items();
        for i in 0..n_items {
            if self.selection.is_selected(i) {
                if let Some(item) = self.selection.item(i) {
                    if let Ok(file_obj) = item.downcast::<FileObject>() {
                        selected_paths.push(file_obj.path());
                    }
                }
            }
        }

        if selected_paths.len() == 1 {
            if let Some(ref callback) = *self.on_rename.borrow() {
                callback(selected_paths[0].clone());
            }
        }
    }
}

impl Default for FileGridView {
    fn default() -> Self {
        Self::new()
    }
}
