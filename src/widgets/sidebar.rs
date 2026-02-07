use gtk4::glib::{self, clone};
use gtk4::prelude::*;
use gtk4::{
    gio, Box as GtkBox, DropTarget, EventControllerKey, GestureClick, Image, Label, ListBox, ListBoxRow, 
    Orientation, PopoverMenu, ScrolledWindow, SelectionMode, Separator,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::fs::OpenOptions;
use std::io::Write;

use crate::core::{DriveScanner, PinnedFolderObject, PinnedFolderStore};

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

// ============================================================================
// Sidebar Item Types
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum SidebarItemType {
    StandardFolder,
    SystemFolder,
    Drive,
}

// ============================================================================
// NautilusSidebar - Main sidebar widget
// ============================================================================

#[derive(Clone)]
pub struct NautilusSidebar {
    container: GtkBox,
    pinned_list_box: ListBox,
    standard_list_box: ListBox,
    other_list_box: ListBox,
    pinned_store: PinnedFolderStore,
    on_location_selected: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
}

impl NautilusSidebar {
    /// Check if a path is a standard location (to avoid duplicates in pinned)
    fn is_standard_location(path: &std::path::Path) -> bool {
        let standard_paths = vec![
            dirs::home_dir(),
            dirs::document_dir(),
            dirs::download_dir(),
            dirs::audio_dir(),
            dirs::picture_dir(),
            dirs::video_dir(),
            Some(PathBuf::from("/")),
        ];
        
        standard_paths.iter()
            .filter_map(|p| p.as_ref())
            .any(|std_path| PinnedFolderStore::normalize_path(path) == 
                            PinnedFolderStore::normalize_path(std_path))
    }

    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["sidebar-container"])
            .build();

        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let main_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .build();

        // Create pinned folders store
        let pinned_store = PinnedFolderStore::new();

        // ===== Pinned Section Container =====
        let pinned_section = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["pinned-section"])
            .build();

        // ===== Pinned Label =====
        let pinned_label = Label::builder()
            .label("Pinned")
            .halign(gtk4::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(6)
            .css_classes(["dim-label", "caption"])
            .build();
        pinned_section.append(&pinned_label);

        let pinned_list_box = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();

        // Bind pinned store to list box using factory
        let on_location_selected: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = 
            Rc::new(RefCell::new(None));
        
        Self::bind_pinned_store(&pinned_list_box, &pinned_store, on_location_selected.clone());

        // Keyboard shortcuts for Pinned List (F2 to rename)
        {
            let pinned_store_clone = pinned_store.clone();
            let key_controller = EventControllerKey::new();
            
            key_controller.connect_key_pressed(move |controller, keyval, _, _| {
                if keyval == gtk4::gdk::Key::F2 {
                    if let Some(widget) = controller.widget() {
                        if let Ok(list_box) = widget.downcast::<ListBox>() {
                            if let Some(row) = list_box.selected_row() {
                                if let Some(path) = Self::get_row_path(&row) {
                                    Self::show_rename_dialog(&path, &row, &pinned_store_clone);
                                    return gtk4::glib::Propagation::Stop;
                                }
                            }
                        }
                    }
                }
                gtk4::glib::Propagation::Proceed
            });
            pinned_list_box.add_controller(key_controller);
        }

        pinned_section.append(&pinned_list_box);
        main_box.append(&pinned_section);

        // Add drop target for drag-and-drop pinning to the entire pinned section
        {
            let pinned_store_clone = pinned_store.clone();
            let pinned_section_clone = pinned_section.clone();
            
            let drop_target = DropTarget::new(
                glib::Type::INVALID, // We will set multiple supported types
                gtk4::gdk::DragAction::COPY | gtk4::gdk::DragAction::MOVE
            );
            
            // Support both FileList and individual GFile
            drop_target.set_types(&[
                gtk4::gdk::FileList::static_type(),
                gio::File::static_type(),
            ]);
            
            drop_target.connect_enter(clone!(
                #[strong] pinned_section_clone,
                move |_, _, _| {
                    pinned_section_clone.add_css_class("drop-highlight");
                    gtk4::gdk::DragAction::COPY
                }
            ));
            
            drop_target.connect_leave(clone!(
                #[strong] pinned_section_clone,
                move |_| {
                    pinned_section_clone.remove_css_class("drop-highlight");
                }
            ));
            
            drop_target.connect_motion(move |_, _, _| {
                gtk4::gdk::DragAction::COPY
            });
            
            drop_target.connect_drop(clone!(
                #[strong] pinned_section_clone,
                move |_, value, _, _| {
                    pinned_section_clone.remove_css_class("drop-highlight");
                    
                    let mut paths = Vec::new();
                    
                    if let Ok(file_list) = value.get::<gtk4::gdk::FileList>() {
                        paths = file_list.files().iter().filter_map(|f| f.path()).collect();
                    } else if let Ok(file) = value.get::<gio::File>() {
                        if let Some(path) = file.path() {
                            paths.push(path);
                        }
                    }
                    
                    if paths.is_empty() {
                        return false;
                    }
                    
                    let mut success = false;
                    for path in paths {
                        // Check if it's a directory
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if !metadata.is_dir() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                        
                        if pinned_store_clone.is_pinned(&path) {
                            continue;
                        }
                        
                        if Self::is_standard_location(&path) {
                            continue;
                        }
                        
                        if let Err(e) = pinned_store_clone.add(&path) {
                            eprintln!("Failed to pin folder: {}", e);
                        } else {
                            success = true;
                        }
                    }
                    
                    success
                }
            ));
            
            pinned_section.add_controller(drop_target);
        }

        // Subtle separator between Pinned and Standard sections
        let sep = Separator::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["sidebar-separator"])
            .build();
        main_box.append(&sep);

        // ===== Standard Folders Section =====
        let standard_list_box = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();

        Self::add_standard_location(&standard_list_box, "Home", "user-home-symbolic", 
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")), SidebarItemType::StandardFolder);

        if let Some(documents) = dirs::document_dir() {
            Self::add_standard_location(&standard_list_box, "Documents", "folder-documents-symbolic", 
                documents, SidebarItemType::StandardFolder);
        }
        
        if let Some(downloads) = dirs::download_dir() {
            Self::add_standard_location(&standard_list_box, "Downloads", "folder-download-symbolic", 
                downloads, SidebarItemType::StandardFolder);
        }
        
        if let Some(music) = dirs::audio_dir() {
            Self::add_standard_location(&standard_list_box, "Music", "folder-music-symbolic", 
                music, SidebarItemType::StandardFolder);
        }
        
        if let Some(pictures) = dirs::picture_dir() {
            Self::add_standard_location(&standard_list_box, "Pictures", "folder-pictures-symbolic", 
                pictures, SidebarItemType::StandardFolder);
        }
        
        if let Some(videos) = dirs::video_dir() {
            Self::add_standard_location(&standard_list_box, "Videos", "folder-videos-symbolic", 
                videos, SidebarItemType::StandardFolder);
        }

        Self::add_standard_location(&standard_list_box, "Trash", "user-trash-symbolic", 
            dirs::home_dir()
                .map(|h| h.join(".local/share/Trash/files"))
                .unwrap_or_else(|| PathBuf::from("/")), SidebarItemType::SystemFolder);

        main_box.append(&standard_list_box);

        // ===== Other Locations Section =====
        let other_label = Label::builder()
            .label("Other Locations")
            .halign(gtk4::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(6)
            .css_classes(["dim-label", "caption"])
            .build();
        main_box.append(&other_label);

        let other_list_box = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();

        Self::add_standard_location(&other_list_box, "Computer", "drive-harddisk-symbolic", 
            PathBuf::from("/"), SidebarItemType::Drive);

        // Add other drives
        let drives = DriveScanner::scan();
        for drive in &drives {
            if drive.mount_point == PathBuf::from("/") {
                continue;
            }
            if let Some(home) = dirs::home_dir() {
                if drive.mount_point == home {
                    continue;
                }
            }
            Self::add_standard_location(&other_list_box, &drive.name, &drive.icon_name, 
                drive.mount_point.clone(), SidebarItemType::Drive);
        }

        main_box.append(&other_list_box);
        scrolled.set_child(Some(&main_box));
        container.append(&scrolled);

        // Selection sync: ensure only one ListBox has a selection at a time
        {
            let pinned_lb = pinned_list_box.clone();
            let standard_lb = standard_list_box.clone();
            let other_lb = other_list_box.clone();

            pinned_lb.connect_row_selected(clone!(
                #[weak] standard_lb,
                #[weak] other_lb,
                move |_, row| {
                    if row.is_some() {
                        standard_lb.unselect_all();
                        other_lb.unselect_all();
                    }
                }
            ));

            standard_lb.connect_row_selected(clone!(
                #[weak] pinned_lb,
                #[weak] other_lb,
                move |_, row| {
                    if row.is_some() {
                        pinned_lb.unselect_all();
                        other_lb.unselect_all();
                    }
                }
            ));

            other_lb.connect_row_selected(clone!(
                #[weak] pinned_lb,
                #[weak] standard_lb,
                move |_, row| {
                    if row.is_some() {
                        pinned_lb.unselect_all();
                        standard_lb.unselect_all();
                    }
                }
            ));
        }

        // Connect standard list box row activation
        {
            let on_location_selected_clone = on_location_selected.clone();
            standard_list_box.connect_row_activated(move |_, row| {
                if let Some(path) = Self::get_row_path(row) {
                    if let Some(ref callback) = *on_location_selected_clone.borrow() {
                        callback(path);
                    }
                }
            });
        }

        // Connect other list box row activation
        {
            let on_location_selected_clone = on_location_selected.clone();
            other_list_box.connect_row_activated(move |_, row| {
                if let Some(path) = Self::get_row_path(row) {
                    if let Some(ref callback) = *on_location_selected_clone.borrow() {
                        callback(path);
                    }
                }
            });
        }

        // Setup context menus for standard and other locations
        Self::setup_standard_context_menu(&standard_list_box, &pinned_store);
        Self::setup_standard_context_menu(&other_list_box, &pinned_store);

        Self {
            container,
            pinned_list_box,
            standard_list_box,
            other_list_box,
            pinned_store,
            on_location_selected,
        }
    }

    /// Bind the pinned store to a ListBox using a factory function
    fn bind_pinned_store(
        list_box: &ListBox, 
        store: &PinnedFolderStore,
        on_location_selected: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>
    ) {
        let store_clone = store.clone();
        let on_location_selected_clone = on_location_selected.clone();
        
        list_box.bind_model(
            Some(store.store()),
            move |obj| {
                let pinned_obj = obj.clone().downcast::<PinnedFolderObject>()
                    .expect("Expected PinnedFolderObject");
                
                // #region agent log
                debug_log("B", "sidebar.rs:bind_pinned_store", "Factory called", serde_json::json!({
                    "path": pinned_obj.path().to_string_lossy(),
                    "name": pinned_obj.name(),
                    "obj_ptr": format!("{:p}", &pinned_obj)
                }));
                // #endregion
                
                // Skip standard locations
                if Self::is_standard_location(&pinned_obj.path()) {
                    // Return an invisible row for standard locations
                    let row = ListBoxRow::builder()
                        .visible(false)
                        .build();
                    return row.upcast();
                }
                
                let row = Self::create_sidebar_row(
                    &pinned_obj.name(), 
                    "folder-symbolic", 
                    &pinned_obj.path()
                );
                
                // #region agent log
                debug_log("B", "sidebar.rs:bind_pinned_store", "Before setup_pinned_row_context_menu", serde_json::json!({
                    "row_ptr": format!("{:p}", &row),
                    "path": pinned_obj.path().to_string_lossy()
                }));
                // #endregion
                
                // Setup context menu for pinned folder
                Self::setup_pinned_row_context_menu(&row, &pinned_obj, &store_clone);
                
                // #region agent log
                debug_log("B", "sidebar.rs:bind_pinned_store", "After setup_pinned_row_context_menu", serde_json::json!({
                    "row_ptr": format!("{:p}", &row),
                    "path": pinned_obj.path().to_string_lossy()
                }));
                // #endregion
                
                // Connect click for navigation
                let path = pinned_obj.path();
                let on_selected = on_location_selected_clone.clone();
                row.connect_activate(move |_| {
                    if let Some(ref callback) = *on_selected.borrow() {
                        callback(path.clone());
                    }
                });
                
                row.upcast()
            }
        );
        
        // Connect row activation for pinned list
        let on_location_selected_activation = on_location_selected.clone();
        let store_for_activation = store.store().clone();
        
        list_box.connect_row_activated(move |_, row| {
            let index = row.index();
            if index >= 0 {
                if let Some(obj) = store_for_activation.item(index as u32) {
                    if let Ok(pinned) = obj.downcast::<PinnedFolderObject>() {
                        if let Some(ref callback) = *on_location_selected_activation.borrow() {
                            callback(pinned.path());
                        }
                    }
                }
            }
        });
    }

    /// Create a sidebar row widget
    fn create_sidebar_row(name: &str, icon_name: &str, path: &std::path::Path) -> ListBoxRow {
        let row = ListBoxRow::builder()
            .css_classes(["sidebar-row"])
            .build();
        
        let hbox = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(4)
            .build();

        let icon = Image::builder()
            .icon_name(icon_name)
            .pixel_size(18)
            .css_classes(["sidebar-icon"])
            .build();

        let label = Label::builder()
            .label(name)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build();

        hbox.append(&icon);
        hbox.append(&label);
        row.set_child(Some(&hbox));
        
        // Store path in row data (unsafe but required for GTK data storage)
        unsafe {
            row.set_data("path", path.to_path_buf());
        }
        
        row
    }

    /// Add a standard location to a ListBox
    fn add_standard_location(
        list_box: &ListBox, 
        name: &str, 
        icon_name: &str, 
        path: PathBuf,
        item_type: SidebarItemType
    ) {
        let row = Self::create_sidebar_row(name, icon_name, &path);
        unsafe {
            row.set_data("item_type", item_type);
        }
        list_box.append(&row);
    }

    /// Get path from a row
    fn get_row_path(row: &ListBoxRow) -> Option<PathBuf> {
        unsafe { row.data::<PathBuf>("path").map(|p| p.as_ref().clone()) }
    }

    /// Get item type from a row
    fn get_row_item_type(row: &ListBoxRow) -> Option<SidebarItemType> {
        unsafe { row.data::<SidebarItemType>("item_type").map(|t| t.as_ref().clone()) }
    }

    /// Setup context menu for a pinned folder row
    fn setup_pinned_row_context_menu(
        row: &ListBoxRow, 
        pinned_obj: &PinnedFolderObject,
        store: &PinnedFolderStore
    ) {
        // #region agent log
        debug_log("A", "sidebar.rs:setup_pinned_row_context_menu", "Function entry", serde_json::json!({
            "row_ptr": format!("{:p}", row),
            "path": pinned_obj.path().to_string_lossy()
        }));
        // #endregion
        
        let gesture = GestureClick::builder().button(3).build();
        let current_popover: Rc<RefCell<Option<PopoverMenu>>> = Rc::new(RefCell::new(None));
        
        let path = pinned_obj.path();
        let store_clone = store.clone();
        
        gesture.connect_pressed(clone!(
            #[strong] current_popover,
            #[strong] path,
            #[strong] store_clone,
            #[weak] row,
            move |_, _, x, y| {
                // Close existing popover
            if let Some(ref mut popover) = *current_popover.borrow_mut() {
                popover.popdown();
                popover.unparent();
            }
            current_popover.borrow_mut().take();

                // Create menu
                let menu = gio::Menu::new();
                
                // Rename action
                let rename_item = gio::MenuItem::new(Some("Rename…"), None);
                rename_item.set_action_and_target_value(
                    Some("sidebar.rename-pinned"),
                    Some(&path.to_string_lossy().to_string().to_variant())
                );
                menu.append_item(&rename_item);
                
                // Unpin action
                let unpin_item = gio::MenuItem::new(Some("Unpin from Sidebar"), None);
                unpin_item.set_action_and_target_value(
                    Some("app.toggle-pin"),
                    Some(&path.to_string_lossy().to_string().to_variant())
                );
                menu.append_item(&unpin_item);

                let popover = PopoverMenu::from_model(Some(&menu));
                popover.set_parent(&row);
                popover.set_position(gtk4::PositionType::Bottom);
                popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                
                // Create action group for sidebar-specific actions
                let action_group = gio::SimpleActionGroup::new();
                
                // Rename action
                let rename_action = gio::SimpleAction::new(
                    "rename-pinned", 
                    Some(&String::static_variant_type())
                );
                let store_for_rename = store_clone.clone();
                let row_clone = row.clone();
                rename_action.connect_activate(move |_, param| {
                    if let Some(path_str) = param.and_then(|p| p.get::<String>()) {
                        let path = PathBuf::from(&path_str);
                        Self::show_rename_dialog(&path, &row_clone, &store_for_rename);
                    }
                });
                action_group.add_action(&rename_action);
                
                popover.insert_action_group("sidebar", Some(&action_group));
                
                let popover_clone = popover.clone();
                let current_popover_clone = current_popover.clone();
                popover.connect_closed(move |p| {
                    p.unparent();
                    current_popover_clone.borrow_mut().take();
                });
                
                *current_popover.borrow_mut() = Some(popover_clone.clone());
                popover_clone.popup();
            }
        ));
        
        // #region agent log
        debug_log("A", "sidebar.rs:setup_pinned_row_context_menu", "Before add_controller", serde_json::json!({
            "row_ptr": format!("{:p}", row),
            "gesture_ptr": format!("{:p}", &gesture),
            "has_widget": gesture.widget().is_some()
        }));
        // #endregion
        
        row.add_controller(gesture);
        
        // #region agent log
        debug_log("A", "sidebar.rs:setup_pinned_row_context_menu", "After add_controller", serde_json::json!({
            "row_ptr": format!("{:p}", row)
        }));
        // #endregion
    }

    /// Setup context menu for standard/other locations
    fn setup_standard_context_menu(list_box: &ListBox, _store: &PinnedFolderStore) {
        // #region agent log
        debug_log("C", "sidebar.rs:setup_standard_context_menu", "Function entry", serde_json::json!({
            "list_box_ptr": format!("{:p}", list_box)
        }));
        // #endregion
        
        let gesture = GestureClick::builder().button(3).build();
        
        // #region agent log
        let widget_after_create = gesture.widget();
        debug_log("D", "sidebar.rs:setup_standard_context_menu", "After creating gesture", serde_json::json!({
            "list_box_ptr": format!("{:p}", list_box),
            "gesture_ptr": format!("{:p}", &gesture),
            "has_widget": widget_after_create.is_some(),
            "widget_type": widget_after_create.as_ref().map(|w| format!("{:?}", w.type_()))
        }));
        // #endregion
        
        let current_popover: Rc<RefCell<Option<PopoverMenu>>> = Rc::new(RefCell::new(None));
        
        // #region agent log
        let widget_before_connect = gesture.widget();
        debug_log("F", "sidebar.rs:setup_standard_context_menu", "Before connect_pressed", serde_json::json!({
            "gesture_ptr": format!("{:p}", &gesture),
            "has_widget": widget_before_connect.is_some()
        }));
        // #endregion
        
        gesture.connect_pressed(clone!(
            #[strong] current_popover,
            move |gesture, _, x, y| {
                // Close existing popover
                if let Some(ref mut popover) = *current_popover.borrow_mut() {
                    popover.popdown();
                    popover.unparent();
                }
                current_popover.borrow_mut().take();

                // Find clicked row
                let Some(widget) = gesture.widget() else { return };
                let Some(picked) = widget.pick(x, y, gtk4::PickFlags::DEFAULT) else { return };
                
                let mut current_widget = Some(picked);
                let mut found_row: Option<ListBoxRow> = None;
                
                while let Some(w) = current_widget {
                    if let Ok(row) = w.clone().downcast::<ListBoxRow>() {
                        found_row = Some(row);
                        break;
                    }
                    current_widget = w.parent();
                }
                
                let Some(row) = found_row else { return };
                let Some(_path) = Self::get_row_path(&row) else { return };
                let item_type = Self::get_row_item_type(&row);
                
                // Don't show menu for system folders (like Trash)
                if item_type == Some(SidebarItemType::SystemFolder) {
                    return;
                }

                // No context menu items available
                return;
            }
        ));
        
        // #region agent log
        let widget_after_connect = gesture.widget();
        debug_log("F", "sidebar.rs:setup_standard_context_menu", "After connect_pressed", serde_json::json!({
            "gesture_ptr": format!("{:p}", &gesture),
            "has_widget": widget_after_connect.is_some(),
            "widget_type": widget_after_connect.as_ref().map(|w| format!("{:?}", w.type_()))
        }));
        // #endregion
        
        // #region agent log
        let widget_before = gesture.widget();
        let controllers_count = list_box.observe_controllers().n_items();
        
        // Check if this gesture is already in the list_box controllers
        let (gesture_already_in_list, existing_gesture_click_count, existing_gesture_has_widget) = {
            let mut found = false;
            let mut gesture_click_count = 0;
            let mut has_widget_count = 0;
            for i in 0..controllers_count {
                if let Some(controller) = list_box.observe_controllers().item(i) {
                    if let Ok(existing_gesture) = controller.downcast::<GestureClick>() {
                        gesture_click_count += 1;
                        if existing_gesture.widget().is_some() {
                            has_widget_count += 1;
                        }
                        if existing_gesture.as_ref() as *const _ == &gesture as *const _ {
                            found = true;
                            break;
                        }
                    }
                }
            }
            (found, gesture_click_count, has_widget_count)
        };
        
        debug_log("C", "sidebar.rs:setup_standard_context_menu", "Before add_controller", serde_json::json!({
            "list_box_ptr": format!("{:p}", list_box),
            "gesture_ptr": format!("{:p}", &gesture),
            "has_widget": widget_before.is_some(),
            "widget_type": widget_before.as_ref().map(|w| format!("{:?}", w.type_())),
            "list_box_controllers_count": controllers_count,
            "gesture_already_in_list": gesture_already_in_list,
            "existing_gesture_click_count": existing_gesture_click_count,
            "existing_gesture_has_widget": existing_gesture_has_widget
        }));
        // #endregion
        
        // Check if gesture already has a widget - this would cause the GTK assertion error
        if widget_before.is_some() {
            eprintln!("ERROR: Gesture already has widget before add_controller! widget={:?}", widget_before);
            return; // Don't add controller if it already has a widget
        }
        
        // Check if gesture is already in the list_box - this would also cause the GTK assertion error
        if gesture_already_in_list {
            eprintln!("ERROR: Gesture is already in list_box controllers! Not adding again.");
            return; // Don't add controller if it's already in the list
        }
        
        list_box.add_controller(gesture);
        
        // #region agent log
        debug_log("C", "sidebar.rs:setup_standard_context_menu", "After add_controller", serde_json::json!({
            "list_box_ptr": format!("{:p}", list_box),
            "controllers_count": list_box.observe_controllers().n_items()
        }));
        // #endregion
    }

    /// Show rename dialog for pinned folder
    fn show_rename_dialog(path: &std::path::Path, row: &ListBoxRow, store: &PinnedFolderStore) {
        use gtk4::Entry;
        use libadwaita as adw;
        use adw::prelude::*;
        
        // Get current name from store
        let current_name = {
            let normalized = PinnedFolderStore::normalize_path(path);
            let mut name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Folder".to_string());

            for i in 0..store.store().n_items() {
                if let Some(obj) = store.store().item(i) {
                    if let Ok(pinned) = obj.downcast::<PinnedFolderObject>() {
                        if PinnedFolderStore::normalize_path(&pinned.path()) == normalized {
                            name = pinned.name();
                            break;
                        }
                    }
                }
            }
            name
        };

        let window = row.root()
            .and_then(|root| root.downcast::<gtk4::ApplicationWindow>().ok());

        let dialog = adw::AlertDialog::builder()
            .heading("Rename Item")
            .body("Enter a new name for this item in the sidebar")
            .build();

        let entry = Entry::builder()
            .text(&current_name)
            .build();
        entry.set_activates_default(true);
        dialog.set_extra_child(Some(&entry));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("rename", "Rename");
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("rename"));
        dialog.set_close_response("cancel");

        let entry_clone = entry.clone();
        glib::idle_add_local_once(move || {
            entry_clone.grab_focus();
            entry_clone.select_region(0, -1);
        });

        let path_clone = path.to_path_buf();
        let store_clone = store.clone();
        let current_name_clone = current_name.clone();
        
        dialog.connect_response(None, move |dialog, response| {
            if response == "rename" {
                if let Some(entry) = dialog.extra_child().and_downcast::<Entry>() {
                    let new_name = entry.text().to_string().trim().to_string();
                    if !new_name.is_empty() && new_name != current_name_clone {
                        if let Err(e) = store_clone.rename(&path_clone, &new_name) {
                            eprintln!("Failed to rename: {}", e);
                        }
                    }
                }
            }
        });

        if let Some(win) = window {
            dialog.present(Some(&win));
        } else {
            dialog.present(None::<&gtk4::Window>);
        }
    }

    pub fn container(&self) -> &GtkBox {
        &self.container
    }

    pub fn pinned_store(&self) -> &PinnedFolderStore {
        &self.pinned_store
    }

    pub fn select_location(&self, _index: i32) {
        // Select first row in standard list box
        if let Some(row) = self.standard_list_box.row_at_index(0) {
            self.standard_list_box.select_row(Some(&row));
        }
    }

    pub fn connect_location_selected<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_location_selected.borrow_mut() = Some(Box::new(callback));
    }

    pub fn refresh(&self) {
        // The ListStore binding automatically updates the UI when the store changes
        // This method is kept for API compatibility but may not need to do anything
        // if we properly use the reactive model
        
        // Force a re-evaluation of the model binding
        self.pinned_list_box.invalidate_filter();
    }

    pub fn unpin_selected(&self) {
        if let Some(row) = self.pinned_list_box.selected_row() {
            if let Some(path) = Self::get_row_path(&row) {
                if let Err(e) = self.pinned_store.remove(&path) {
                    eprintln!("Failed to unpin: {}", e);
                } else {
                    println!("[DEBUG] Unpinned folder via method: {:?}", path);
                }
            }
        }
    }
}

impl Default for NautilusSidebar {
    fn default() -> Self {
        Self::new()
    }
}
