use gtk4::glib::{self, clone};
use gtk4::prelude::*;
use gtk4::{gio, Box as GtkBox, Orientation, ProgressBar, ScrolledWindow, Label};
use libadwaita as adw;
use adw::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;
use std::thread;
use std::time::Duration;
use async_channel;

use crate::core::{Clipboard, ClipboardMode, FileOperations, SidebarPrefs, ProgressInfo};
use crate::widgets::{FileGridView, NautilusHeaderBar, NautilusSidebar};

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

pub struct BlinkWindow {
    pub window: adw::ApplicationWindow,
}

impl BlinkWindow {
    pub fn new(app: &adw::Application) -> Self {
        // Create main window - Nautilus style
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Files")
            .default_width(1100)
            .default_height(700)
            .build();

        // Shared state
        let current_path: Rc<RefCell<PathBuf>> = Rc::new(RefCell::new(
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        ));
        let history: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
        let history_index: Rc<RefCell<i32>> = Rc::new(RefCell::new(-1));
        let clipboard: Rc<RefCell<Clipboard>> = Rc::new(RefCell::new(Clipboard::new()));

        // Create header bar (Nautilus style)
        let header_bar = NautilusHeaderBar::new();

        // Create navigation split view for sidebar + content
        let split_view = adw::NavigationSplitView::builder()
            .min_sidebar_width(200.0)
            .max_sidebar_width(280.0)
            .sidebar_width_fraction(0.22)
            .build();

        // Create sidebar (Nautilus style)
        let sidebar = NautilusSidebar::new();
        let sidebar_page = adw::NavigationPage::builder()
            .title("Places")
            .child(sidebar.container())
            .build();
        split_view.set_sidebar(Some(&sidebar_page));

        // Content area with file grid
        let content_box = GtkBox::new(Orientation::Vertical, 0);
        content_box.add_css_class("view");
        
        // Add header bar to content area so sidebar spans full height
        content_box.append(header_bar.container());

        // File grid view with scroll
        let file_view = FileGridView::new();
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(file_view.container())
            .build();
        scrolled.add_css_class("nautilus-scrolled");
        content_box.append(&scrolled);

        let content_page = adw::NavigationPage::builder()
            .title("Files")
            .child(&content_box)
            .build();
        split_view.set_content(Some(&content_page));

        window.set_content(Some(&split_view));

        // Shared state for hidden files to sync with monitor
        let last_show_hidden = Rc::new(RefCell::new(SidebarPrefs::show_hidden_files()));

        // Keyboard shortcuts
        {
            let file_view_clone = file_view.clone();
            let sidebar_clone = sidebar.clone();
            let last_show_hidden_clone = last_show_hidden.clone();
            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, keyval, modifiers, _| {
                // Ctrl+H to toggle hidden files
                if keyval == gtk4::gdk::Key::h && modifiers & gtk4::gdk::ModifierType::CONTROL_MASK.bits() != 0 {
                    let mut current_hidden = last_show_hidden_clone.borrow_mut();
                    let new_hidden = !*current_hidden;
                    *current_hidden = new_hidden;
                    
                    file_view_clone.set_show_hidden(new_hidden);
                    file_view_clone.refresh();
                    println!("Toggled hidden files (session): {}", new_hidden);
                    return gtk4::glib::Propagation::Stop;
                }
                
                // Key 'o' to unpin selected item in sidebar
                if keyval == gtk4::gdk::Key::o {
                    sidebar_clone.unpin_selected();
                    return gtk4::glib::Propagation::Stop;
                }

                // F2 to rename selected file
                if keyval == gtk4::gdk::Key::F2 {
                    file_view_clone.rename_selected();
                    return gtk4::glib::Propagation::Stop;
                }
                
                gtk4::glib::Propagation::Proceed
            });
            window.add_controller(key_controller);
        }

        // Initialize
        let initial_path = current_path.borrow().clone();
        
        // #region agent log
        debug_log("E", "window.rs:new", "Initial path", serde_json::json!({
            "path": initial_path.to_string_lossy(),
            "exists": initial_path.exists(),
            "is_dir": initial_path.is_dir()
        }));
        // #endregion
        
        // Load with hidden files setting from config
        let show_hidden = SidebarPrefs::show_hidden_files();
        file_view.set_show_hidden(show_hidden);
        
        // #region agent log
        debug_log("E", "window.rs:new", "Before load_directory", serde_json::json!({
            "path": initial_path.to_string_lossy()
        }));
        // #endregion
        
        file_view.load_directory(&initial_path);
        
        // #region agent log
        debug_log("E", "window.rs:new", "After load_directory", serde_json::json!({
            "path": initial_path.to_string_lossy()
        }));
        // #endregion
        header_bar.set_path(&initial_path);
        history.borrow_mut().push(initial_path.clone());
        *history_index.borrow_mut() = 0;

        // Select Home in sidebar initially
        sidebar.select_location(0);

        // =========================================================================
        // Register app.toggle-pin action with path parameter
        // =========================================================================
        {
            let pinned_store = sidebar.pinned_store().clone();
            let sidebar_clone = sidebar.clone();
            
            let toggle_pin_action = gio::SimpleAction::new(
                "toggle-pin",
                Some(&String::static_variant_type())
            );
            
            toggle_pin_action.connect_activate(clone!(
                #[strong] pinned_store,
                #[strong] sidebar_clone,
                move |_, param| {
                    if let Some(path_str) = param.and_then(|p| p.get::<String>()) {
                        let path = PathBuf::from(&path_str);
                        
                        match pinned_store.toggle_pin(&path) {
                            Ok(is_now_pinned) => {
                                if is_now_pinned {
                                    println!("[DEBUG] Pinned folder: {:?}", path);
                                } else {
                                    println!("[DEBUG] Unpinned folder: {:?}", path);
                                }
                                // Refresh sidebar to update UI
                                sidebar_clone.refresh();
                            }
                            Err(e) => {
                                eprintln!("Failed to toggle pin: {}", e);
                            }
                        }
                    }
                }
            ));
            
            app.add_action(&toggle_pin_action);
        }

        // =========================================================================
        // Register refresh_sidebar action
        // =========================================================================
        {
            let sidebar_clone = sidebar.clone();
            let refresh_action = gio::SimpleAction::new("refresh_sidebar", None);
            refresh_action.connect_activate(move |_, _| {
                sidebar_clone.refresh();
            });
            app.add_action(&refresh_action);
        }

        // Monitor for config changes (hidden files setting from fuse)
        {
            let file_view_monitor = file_view.clone();
            let last_show_hidden_monitor = last_show_hidden.clone();
            
            glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                let show_hidden = SidebarPrefs::show_hidden_files();
                let mut last_value = last_show_hidden_monitor.borrow_mut();
                
                // Only update if the value actually changed
                if show_hidden != *last_value {
                    *last_value = show_hidden;
                    file_view_monitor.set_show_hidden(show_hidden);
                    file_view_monitor.refresh();
                    println!("Hidden files visibility changed to: {}", show_hidden);
                }
                
                glib::ControlFlow::Continue
            });
        }


        // Add mouse button navigation (back/forward buttons)
        {
            // Create gesture for button 8 (back)
            let file_view_back = file_view.clone();
            let header_bar_back = header_bar.clone();
            let current_path_back = current_path.clone();
            let history_back = history.clone();
            let history_index_back = history_index.clone();

            let gesture_back = gtk4::GestureClick::builder().button(8).build();
            gesture_back.connect_pressed(move |_, _, _, _| {
                let mut idx = history_index_back.borrow_mut();
                if *idx > 0 {
                    *idx -= 1;
                    let hist = history_back.borrow();
                    if let Some(path) = hist.get(*idx as usize) {
                        current_path_back.replace(path.clone());
                        file_view_back.load_directory(path);
                        header_bar_back.set_path(path);
                    }
                }
            });
            window.add_controller(gesture_back);

            // Create gesture for button 9 (forward)
            let file_view_forward = file_view.clone();
            let header_bar_forward = header_bar.clone();
            let current_path_forward = current_path.clone();
            let history_forward = history.clone();
            let history_index_forward = history_index.clone();

            let gesture_forward = gtk4::GestureClick::builder().button(9).build();
            gesture_forward.connect_pressed(move |_, _, _, _| {
                let hist = history_forward.borrow();
                let mut idx = history_index_forward.borrow_mut();
                if (*idx as usize) < hist.len() - 1 {
                    *idx += 1;
                    if let Some(path) = hist.get(*idx as usize) {
                        current_path_forward.replace(path.clone());
                        file_view_forward.load_directory(path);
                        header_bar_forward.set_path(path);
                    }
                }
            });
            window.add_controller(gesture_forward);
        }

        // Helper function to navigate to a path
        let navigate_to = {
            let file_view = file_view.clone();
            let header_bar = header_bar.clone();
            let current_path = current_path.clone();
            let history = history.clone();
            let history_index = history_index.clone();

            move |path: PathBuf, add_to_history: bool| {
                current_path.replace(path.clone());
                file_view.load_directory(&path);
                header_bar.set_path(&path);

                if add_to_history {
                    let mut hist = history.borrow_mut();
                    let mut idx = history_index.borrow_mut();
                    *idx += 1;
                    hist.truncate(*idx as usize);
                    hist.push(path);
                }
            }
        };

        // Connect sidebar navigation
        {
            let navigate_to = navigate_to.clone();
            sidebar.connect_location_selected(move |path| {
                navigate_to(path, true);
            });
        }

        // Connect header bar breadcrumb navigation
        {
            let navigate_to = navigate_to.clone();
            header_bar.connect_path_clicked(move |path| {
                navigate_to(path, true);
            });
        }

        // Connect path entry (when user types path)
        {
            let navigate_to = navigate_to.clone();
            header_bar.connect_path_entered(move |path| {
                navigate_to(path, true);
            });
        }

        // Connect file view directory activation (double-click on folder)
        {
            let navigate_to = navigate_to.clone();
            file_view.connect_directory_activated(move |path| {
                navigate_to(path, true);
            });
        }

        // Connect search
        {
            let file_view = file_view.clone();
            header_bar.connect_search(move |query| {
                file_view.filter(&query);
            });
        }

        // Connect view toggle (grid/list)
        {
            let file_view = file_view.clone();
            let header_bar_clone = header_bar.clone();
            header_bar.connect_view_toggle(move || {
                file_view.toggle_view_mode();
                let is_grid = file_view.is_grid_mode();
                header_bar_clone.set_view_icon(is_grid);
            });
        }

        // Connect new folder
        {
            let current_path = current_path.clone();
            let file_view = file_view.clone();
            let window_weak = window.downgrade();

            header_bar.connect_new_folder(move || {
                let current = current_path.borrow().clone();
                let file_view = file_view.clone();
                
                if let Some(window) = window_weak.upgrade() {
                    let dialog = adw::AlertDialog::builder()
                        .heading("New Folder")
                        .body("Enter name for the new folder")
                        .build();

                    let entry = gtk4::Entry::builder()
                        .placeholder_text("Folder name")
                        .text("New Folder")
                        .build();
                    entry.add_css_class("nautilus-entry");
                    entry.set_activates_default(true);
                    dialog.set_extra_child(Some(&entry));

                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("create", "Create");
                    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
                    dialog.set_default_response(Some("create"));
                    dialog.set_close_response("cancel");

                    // Select all text and focus entry when dialog opens
                    let entry_clone = entry.clone();
                    glib::idle_add_local_once(move || {
                        entry_clone.grab_focus();
                        entry_clone.select_region(0, -1);
                    });

                    let current_clone = current.clone();
                    dialog.connect_response(None, move |dialog, response| {
                        if response == "create" {
                            if let Some(entry) = dialog.extra_child().and_downcast::<gtk4::Entry>() {
                                let name = entry.text();
                                if !name.is_empty() {
                                    let new_path = current_clone.join(name.as_str());
                                    if let Err(e) = FileOperations::create_directory(&new_path) {
                                        eprintln!("Failed to create folder: {}", e);
                                    } else {
                                        file_view.refresh();
                                    }
                                }
                            }
                        }
                    });

                    dialog.present(Some(&window));
                }
            });
        }

        // Connect file operations (copy, cut, paste, delete, rename)
        {
            let clipboard_clone = clipboard.clone();
            file_view.connect_copy(move |paths| {
                clipboard_clone.borrow_mut().copy(paths);
            });
        }

        {
            let clipboard_clone = clipboard.clone();
            file_view.connect_cut(move |paths| {
                clipboard_clone.borrow_mut().cut(paths);
            });
        }

        {
            let clipboard_clone = clipboard.clone();
            let current_path_clone = current_path.clone();
            let file_view_clone = file_view.clone();
            let window_clone = window.clone();
            file_view.connect_paste(move || {
                let dest = current_path_clone.borrow().clone();
                let clipboard_guard = clipboard_clone.borrow();
                let paths = clipboard_guard.get_paths();
                let mode = clipboard_guard.mode();
                drop(clipboard_guard);
                
                if mode == ClipboardMode::None || paths.is_empty() {
                    return;
                }
                
                // Calculate total size for progress
                let (total_size, total_files) = FileOperations::calculate_total_size(&paths);
                
                // Create progress dialog
                let progress_info = Arc::new(Mutex::new(ProgressInfo {
                    current_file: String::new(),
                    bytes_copied: 0,
                    total_bytes: total_size,
                    files_copied: 0,
                    total_files,
                }));
                
                let (progress_bar, status_label, file_label) = Self::create_progress_dialog(&window_clone, mode);
                let progress_info_clone = progress_info.clone();
                
                // Update UI periodically
                let progress_bar_weak = progress_bar.downgrade();
                let status_label_weak = status_label.downgrade();
                let file_label_weak = file_label.downgrade();
                
                glib::timeout_add_local(Duration::from_millis(100), move || {
                    let progress_info = progress_info_clone.lock().unwrap();
                    
                    if let Some(progress_bar) = progress_bar_weak.upgrade() {
                        if progress_info.total_bytes > 0 {
                            let fraction = progress_info.bytes_copied as f64 / progress_info.total_bytes as f64;
                            progress_bar.set_fraction(fraction);
                        } else if progress_info.total_files > 0 {
                            let fraction = progress_info.files_copied as f64 / progress_info.total_files as f64;
                            progress_bar.set_fraction(fraction);
                        }
                    }
                    
                    if let Some(status_label) = status_label_weak.upgrade() {
                        if progress_info.total_bytes > 0 {
                            let mb_copied = progress_info.bytes_copied as f64 / (1024.0 * 1024.0);
                            let mb_total = progress_info.total_bytes as f64 / (1024.0 * 1024.0);
                            status_label.set_text(&format!("{:.1} MB / {:.1} MB", mb_copied, mb_total));
                        } else {
                            status_label.set_text(&format!("{} / {} files", progress_info.files_copied, progress_info.total_files));
                        }
                    }
                    
                    if let Some(file_label) = file_label_weak.upgrade() {
                        if !progress_info.current_file.is_empty() {
                            file_label.set_text(&progress_info.current_file);
                        }
                    }
                    
                    // Continue updating if not complete
                    if progress_info.files_copied < progress_info.total_files {
                        glib::ControlFlow::Continue
                    } else {
                        glib::ControlFlow::Break
                    }
                });
                
                // Perform operation in background thread
                let dest_clone = dest.clone();
                let paths_clone = paths.clone();
                let mode_clone = mode;
                let progress_info_thread = progress_info.clone();
                let progress_bar_weak_final = progress_bar.downgrade();
                let file_view_clone_final = file_view_clone.clone();
                let clipboard_clone_final = clipboard_clone.clone();
                
                // Communication definitions
                #[derive(Debug, Clone)]
                enum ConflictResolution {
                    Replace,
                    Rename(String),
                    Cancel,
                }

                #[derive(Debug, Clone)]
                enum PasteEvent {
                    Conflict(PathBuf),
                    Finished(ClipboardMode),
                }
                
                // Worker -> UI (Events)
                let (event_tx, event_rx) = async_channel::unbounded::<PasteEvent>();
                // UI -> Worker (Resolution)
                let (resolve_tx, resolve_rx) = async_channel::unbounded::<ConflictResolution>();
                
                // Listen for events on UI thread
                let window_weak = window_clone.downgrade();
                
                glib::spawn_future_local(async move {
                    while let Ok(event) = event_rx.recv().await {
                        match event {
                            PasteEvent::Finished(mode) => {
                                // Completed
                                if mode == ClipboardMode::Cut {
                                    clipboard_clone_final.borrow_mut().clear();
                                }
                                
                                if let Some(progress_bar) = progress_bar_weak_final.upgrade() {
                                    if let Some(dialog) = progress_bar.parent().and_then(|p| p.parent()) {
                                        if let Some(dialog) = dialog.downcast_ref::<adw::Window>() {
                                            dialog.close();
                                        }
                                    }
                                }
                                
                                glib::timeout_add_local(Duration::from_millis(500), move || {
                                    file_view_clone_final.refresh();
                                    glib::ControlFlow::Break
                                });
                                break;
                            }
                            PasteEvent::Conflict(conflicting_path) => {
                                // Handle conflict
                                if let Some(window) = window_weak.upgrade() {
                                    let file_name = conflicting_path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "file".to_string());
                                    
                                    let (response_tx, response_rx) = async_channel::bounded(1);
                                    
                                    let dialog = adw::AlertDialog::builder()
                                        .heading("File Conflict")
                                        .body(&format!("\"{}\" already exists. What do you want to do?", file_name))
                                        .build();
                                    
                                    dialog.add_response("cancel", "Cancel");
                                    dialog.add_response("rename", "Rename");
                                    dialog.add_response("replace", "Replace");
                                    
                                    dialog.set_response_appearance("replace", adw::ResponseAppearance::Destructive);
                                    dialog.set_default_response(Some("rename"));
                                    dialog.set_close_response("cancel");
                                    
                                    let window_for_dialog = window.clone();
                                    let response_tx_clone = response_tx.clone();
                                    let file_name_clone = file_name.clone();
                                    
                                    dialog.connect_response(None, move |_dialog, response| {
                                        if response == "replace" {
                                            let _ = response_tx_clone.send_blocking(ConflictResolution::Replace);
                                        } else if response == "rename" {
                                            // Show rename input dialog
                                            let rename_dialog = adw::AlertDialog::builder()
                                                .heading("Rename")
                                                .body("Enter new name for the destination file")
                                                .build();
                                            
                                            let entry = gtk4::Entry::builder()
                                                .text(&file_name_clone)
                                                .activates_default(true)
                                                .build();
                                            entry.add_css_class("nautilus-entry");
                                            
                                            rename_dialog.set_extra_child(Some(&entry));
                                            rename_dialog.add_response("cancel", "Cancel");
                                            rename_dialog.add_response("apply", "Rename");
                                            rename_dialog.set_response_appearance("apply", adw::ResponseAppearance::Suggested);
                                            rename_dialog.set_default_response(Some("apply"));
                                            rename_dialog.set_close_response("cancel");
                                            
                                            let response_tx_inner = response_tx_clone.clone();
                                            
                                            rename_dialog.connect_response(None, move |d, res| {
                                                if res == "apply" {
                                                    if let Some(entry) = d.extra_child().and_downcast::<gtk4::Entry>() {
                                                        let new_name = entry.text().to_string();
                                                        if !new_name.is_empty() {
                                                            let _ = response_tx_inner.send_blocking(ConflictResolution::Rename(new_name));
                                                            return;
                                                        }
                                                    }
                                                }
                                                let _ = response_tx_inner.send_blocking(ConflictResolution::Cancel);
                                            });
                                            
                                            rename_dialog.present(Some(&window_for_dialog));
                                            
                                        } else {
                                            let _ = response_tx_clone.send_blocking(ConflictResolution::Cancel);
                                        }
                                    });
                                    
                                    dialog.present(Some(&window));
                                    
                                    // Wait for user response
                                    if let Ok(res) = response_rx.recv().await {
                                        let _ = resolve_tx.send(res).await;
                                    } else {
                                        let _ = resolve_tx.send(ConflictResolution::Cancel).await;
                                    }
                                } else {
                                    let _ = resolve_tx.send(ConflictResolution::Cancel).await;
                                }
                            }
                        }
                    }
                });
                
                // Spawn worker thread
                thread::spawn(move || {
                    for source in &paths_clone {
                        let file_name: String = match source.file_name() {
                            Some(name) => name.to_string_lossy().to_string(),
                            None => continue,
                        };
                        
                        let mut dest_path = dest_clone.join(&file_name);
                        let mut skip_file = false;

                        // Resolve conflicts
                        loop {
                            if !dest_path.exists() {
                                break;
                            }
                            
                            // Ask UI for help
                            if event_tx.send_blocking(PasteEvent::Conflict(dest_path.clone())).is_err() {
                                return;
                            }
                            
                            match resolve_rx.recv_blocking() {
                                Ok(ConflictResolution::Replace) => break,
                                Ok(ConflictResolution::Rename(new_name)) => {
                                    dest_path = dest_clone.join(new_name);
                                },
                                Ok(ConflictResolution::Cancel) | Err(_) => {
                                    skip_file = true;
                                    break; 
                                }
                            }
                        }
                        
                        if skip_file {
                            // Cancel operation (or just this file? User intent usually implies Abort)
                            let _ = event_tx.send_blocking(PasteEvent::Finished( mode_clone));
                            return; 
                        }
                        
                        match mode_clone {
                            ClipboardMode::Copy => {
                                if let Err(e) = FileOperations::copy_file_with_progress(
                                    source,
                                    &dest_path,
                                    Some(progress_info_thread.clone()),
                                ) {
                                    eprintln!("Copy error: {}", e);
                                }
                            }
                            ClipboardMode::Cut => {
                                if let Err(e) = FileOperations::move_file_with_progress(
                                    source,
                                    &dest_path,
                                    Some(progress_info_thread.clone()),
                                ) {
                                    eprintln!("Move error: {}", e);
                                }
                            }
                            ClipboardMode::None => {}
                        }
                    }
                    
                    // Signal completion
                    let _ = event_tx.send_blocking(PasteEvent::Finished(mode_clone));
                });
            });
        }

        {
            let file_view_clone = file_view.clone();
            let window_weak = window.downgrade();
            file_view.connect_delete(move |paths| {
                if let Some(window) = window_weak.upgrade() {
                    let file_view = file_view_clone.clone();
                    let paths_clone = paths.clone();
                    
                    let count = paths.len();
                    let message = if count == 1 {
                        format!("Move \"{}\" to trash?", paths[0].file_name().unwrap_or_default().to_string_lossy())
                    } else {
                        format!("Move {} items to trash?", count)
                    };

                    let dialog = adw::AlertDialog::builder()
                        .heading("Move to Trash")
                        .body(&message)
                        .build();

                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("trash", "Move to Trash");
                    dialog.set_response_appearance("trash", adw::ResponseAppearance::Destructive);

                    dialog.connect_response(None, move |_, response| {
                        if response == "trash" {
                            println!("[DEBUG] Deleting {} items", paths_clone.len());
                            for path in &paths_clone {
                                println!("[DEBUG] Moving to trash: {:?}", path);
                                if let Err(e) = FileOperations::delete(path) {
                                    eprintln!("Delete error: {}", e);
                                }
                            }
                            
                            // Refresh with a small delay to allow the filesystem to update
                            let file_view_delayed = file_view.clone();
                            glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                                file_view_delayed.refresh();
                            });
                        }
                    });

                    dialog.present(Some(&window));
                }
            });
        }

        // Pin callback now uses the app.toggle-pin action via file_view's context menu
        // The connect_pin is kept for backwards compatibility but the action is preferred
        {
            let pinned_store = sidebar.pinned_store().clone();
            let sidebar_clone = sidebar.clone();
            file_view.connect_pin(move |path| {
                match pinned_store.toggle_pin(&path) {
                    Ok(is_now_pinned) => {
                        if is_now_pinned {
                            println!("[DEBUG] Pinned folder: {:?}", path);
                        } else {
                            println!("[DEBUG] Unpinned folder: {:?}", path);
                        }
                        sidebar_clone.refresh();
                    }
                    Err(e) => {
                        eprintln!("Failed to toggle pin: {}", e);
                    }
                }
            });
        }

        {
            let file_view_clone = file_view.clone();
            let window_weak = window.downgrade();
            file_view.connect_rename(move |path| {
                if let Some(window) = window_weak.upgrade() {
                    let file_view = file_view_clone.clone();
                    let path_clone = path.clone();
                    
                    let current_name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let dialog = adw::AlertDialog::builder()
                        .heading("Rename")
                        .body("Enter new name")
                        .build();

                    let entry = gtk4::Entry::builder()
                        .text(&current_name)
                        .build();
                    entry.add_css_class("nautilus-entry");
                    entry.set_activates_default(true);
                    dialog.set_extra_child(Some(&entry));

                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("rename", "Rename");
                    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
                    dialog.set_default_response(Some("rename"));
                    dialog.set_close_response("cancel");

                    // Select all text and focus entry when dialog opens
                    let entry_clone = entry.clone();
                    glib::idle_add_local_once(move || {
                        entry_clone.grab_focus();
                        entry_clone.select_region(0, -1);
                    });

                    let current_name_clone = current_name.clone();
                    dialog.connect_response(None, move |dialog, response| {
                        if response == "rename" {
                            if let Some(entry) = dialog.extra_child().and_downcast::<gtk4::Entry>() {
                                let new_name = entry.text();
                                if !new_name.is_empty() && new_name.as_str() != current_name_clone {
                                    if let Err(e) = FileOperations::rename(&path_clone, &new_name) {
                                        eprintln!("Rename error: {}", e);
                                    } else {
                                        file_view.refresh();
                                    }
                                }
                            }
                        }
                    });

                    dialog.present(Some(&window));
                }
            });
        }

        // Connect open terminal (key 'f')
        {
            file_view.connect_open_terminal(move |path| {
                let dir = if path.is_dir() {
                    path
                } else {
                    path.parent().unwrap_or_else(|| std::path::Path::new("/")).to_path_buf()
                };
                
                // Open terminal in the directory - try common terminal emulators
                let dir_str = dir.to_string_lossy().to_string();
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
                
                // Try different terminal emulators
                let terminals = vec!["alacritty", "kitty", "gnome-terminal", "xterm", "urxvt", "terminator"];
                let mut opened = false;
                
                for term in terminals {
                    let result = match term {
                        "alacritty" => std::process::Command::new("alacritty")
                            .arg("--working-directory")
                            .arg(&dir_str)
                            .spawn(),
                        "kitty" => std::process::Command::new("kitty")
                            .arg("--directory")
                            .arg(&dir_str)
                            .spawn(),
                        "gnome-terminal" => std::process::Command::new("gnome-terminal")
                            .arg("--working-directory")
                            .arg(&dir_str)
                            .spawn(),
                        "xterm" => std::process::Command::new("xterm")
                            .arg("-e")
                            .arg("sh")
                            .arg("-c")
                            .arg(format!("cd '{}' && exec {}", dir_str, shell))
                            .spawn(),
                        "urxvt" => std::process::Command::new("urxvt")
                            .arg("-cd")
                            .arg(&dir_str)
                            .spawn(),
                        "terminator" => std::process::Command::new("terminator")
                            .arg("--working-directory")
                            .arg(&dir_str)
                            .spawn(),
                        _ => continue,
                    };
                    
                    if result.is_ok() {
                        opened = true;
                        break;
                    }
                }
                
                if !opened {
                    eprintln!("Failed to open terminal - no terminal emulator found");
                }
            });
        }

        // Connect open micro (key 'm')
        {
            file_view.connect_open_micro(move |path| {
                if !path.is_dir() {
                    let path_str = path.to_string_lossy().to_string();
                    // Open terminal with micro editor
                    let terminals = vec!["alacritty", "kitty", "gnome-terminal", "xterm", "urxvt", "terminator"];
                    let mut opened = false;
                    
                    for term in terminals {
                        let result = match term {
                            "alacritty" => std::process::Command::new("alacritty")
                                .arg("-e")
                                .arg("micro")
                                .arg(&path_str)
                                .spawn(),
                            "kitty" => std::process::Command::new("kitty")
                                .arg("micro")
                                .arg(&path_str)
                                .spawn(),
                            "gnome-terminal" => std::process::Command::new("gnome-terminal")
                                .arg("--")
                                .arg("micro")
                                .arg(&path_str)
                                .spawn(),
                            "xterm" => std::process::Command::new("xterm")
                                .arg("-e")
                                .arg("micro")
                                .arg(&path_str)
                                .spawn(),
                            "urxvt" => std::process::Command::new("urxvt")
                                .arg("-e")
                                .arg("micro")
                                .arg(&path_str)
                                .spawn(),
                            "terminator" => std::process::Command::new("terminator")
                                .arg("-e")
                                .arg(format!("micro '{}'", path_str))
                                .spawn(),
                            _ => continue,
                        };
                        
                        if result.is_ok() {
                            opened = true;
                            break;
                        }
                    }
                    
                    if !opened {
                        // Fallback: try to open micro directly
                        if let Err(e) = std::process::Command::new("micro")
                            .arg(&path_str)
                            .spawn()
                        {
                            eprintln!("Failed to open micro: {}", e);
                        }
                    }
                }
            });
        }

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
    
    fn create_progress_dialog(
        window: &adw::ApplicationWindow,
        mode: ClipboardMode,
    ) -> (ProgressBar, Label, Label) {
        let dialog = adw::Window::builder()
            .transient_for(window)
            .modal(true)
            .title(if mode == ClipboardMode::Copy { "Copying files..." } else { "Moving files..." })
            .default_width(400)
            .resizable(false)
            .build();
        
        let content_box = GtkBox::new(Orientation::Vertical, 12);
        content_box.set_margin_top(20);
        content_box.set_margin_bottom(20);
        content_box.set_margin_start(20);
        content_box.set_margin_end(20);
        
        let file_label = Label::builder()
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .build();
        file_label.set_text("Preparing...");
        content_box.append(&file_label);
        
        let progress_bar = ProgressBar::new();
        progress_bar.set_show_text(false);
        content_box.append(&progress_bar);
        
        let status_label = Label::builder()
            .halign(gtk4::Align::Start)
            .css_classes(vec!["dim-label"])
            .build();
        status_label.set_text("0 / 0");
        content_box.append(&status_label);
        
        dialog.set_content(Some(&content_box));
        dialog.present();
        
        (progress_bar, status_label, file_label)
    }
}
