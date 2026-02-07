use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Orientation, Popover, SearchEntry, ToggleButton};
use libadwaita as adw;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone)]
pub struct NautilusHeaderBar {
    container: adw::HeaderBar,
    breadcrumbs_box: GtkBox,
    path_entry: Entry,
    path_entry_box: GtkBox,
    search_entry: SearchEntry,
    search_popover: Popover,
    view_toggle_btn: Button,
    is_editing_path: Rc<RefCell<bool>>,

    on_path_clicked: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    on_path_entered: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>>,
    on_search: Rc<RefCell<Option<Box<dyn Fn(String)>>>>,
    on_view_toggle: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    on_new_folder: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl NautilusHeaderBar {
    pub fn new() -> Self {
        let container = adw::HeaderBar::new();
        container.add_css_class("flat");
        container.set_show_back_button(false);

        // ===== CENTER: Breadcrumb path bar with editable entry =====
        let path_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .halign(gtk4::Align::Center)
            .css_classes(["nautilus-path-container"])
            .build();

        // Breadcrumbs view
        let breadcrumbs_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .css_classes(["linked", "nautilus-path-bar"])
            .build();

        // Path entry (hidden by default, shown when clicking breadcrumbs)
        let path_entry = Entry::builder()
            .placeholder_text("Enter path...")
            .build();
        path_entry.add_css_class("nautilus-path-entry");
        path_entry.set_visible(false);

        let path_entry_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .build();
        path_entry_box.append(&path_entry);

        path_box.append(&breadcrumbs_box);
        path_box.append(&path_entry_box);

        container.set_title_widget(Some(&path_box));

        // ===== RIGHT SIDE: Actions =====
        
        // Search button with popover
        let search_btn = ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text("Search (Ctrl+F)")
            .build();

        let search_popover = Popover::builder()
            .has_arrow(true)
            .build();
        search_popover.set_parent(&search_btn);

        let search_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search files...")
            .width_chars(30)
            .build();
        search_entry.add_css_class("nautilus-search");

        search_box.append(&search_entry);
        search_popover.set_child(Some(&search_box));

        {
            let search_popover_clone = search_popover.clone();
            search_btn.connect_toggled(move |btn| {
                if btn.is_active() {
                    search_popover_clone.popup();
                } else {
                    search_popover_clone.popdown();
                }
            });
        }

        {
            let search_btn_clone = search_btn.clone();
            search_popover.connect_closed(move |_| {
                search_btn_clone.set_active(false);
            });
        }

        container.pack_end(&search_btn);

        // View toggle (grid/list)
        let view_toggle_btn = Button::builder()
            .icon_name("view-grid-symbolic")
            .tooltip_text("Toggle View")
            .build();
        container.pack_end(&view_toggle_btn);

        // New folder button
        let new_folder_btn = Button::builder()
            .icon_name("folder-new-symbolic")
            .tooltip_text("New Folder")
            .build();
        container.pack_end(&new_folder_btn);

        // Callbacks
        let on_path_clicked: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        let on_path_entered: Rc<RefCell<Option<Box<dyn Fn(PathBuf)>>>> = Rc::new(RefCell::new(None));
        let on_search: Rc<RefCell<Option<Box<dyn Fn(String)>>>> = Rc::new(RefCell::new(None));
        let on_view_toggle: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let on_new_folder: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let is_editing_path = Rc::new(RefCell::new(false));

        // Make breadcrumbs clickable to show entry (double-click)
        {
            let breadcrumbs_box_clone = breadcrumbs_box.clone();
            let path_entry_clone = path_entry.clone();
            let is_editing_path_clone = is_editing_path.clone();

            // Track clicks for double-click detection
            let last_click_time = Rc::new(RefCell::new(std::time::Instant::now()));
            let click_count = Rc::new(RefCell::new(0u32));

            let gesture = gtk4::GestureClick::new();
            gesture.set_button(1);
            gesture.connect_pressed(move |_, _, _, _| {
                let now = std::time::Instant::now();
                let mut last_time = last_click_time.borrow_mut();
                let mut count = click_count.borrow_mut();

                // Check if this is a double click (within 500ms of last click)
                if now.duration_since(*last_time).as_millis() < 500 {
                    *count += 1;
                } else {
                    *count = 1;
                }

                *last_time = now;

                // If double click (count == 2), show entry
                if *count == 2 {
                    *is_editing_path_clone.borrow_mut() = true;
                    breadcrumbs_box_clone.set_visible(false);
                    path_entry_clone.set_visible(true);
                    path_entry_clone.grab_focus();
                    path_entry_clone.select_region(0, -1);
                    *count = 0; // Reset counter
                }
            });
            breadcrumbs_box.add_controller(gesture);
        }

        // Handle path entry
        {
            let breadcrumbs_box_clone = breadcrumbs_box.clone();
            let path_entry_clone = path_entry.clone();
            let is_editing_path_clone = is_editing_path.clone();
            let on_path_entered_clone = on_path_entered.clone();

            path_entry.connect_activate(move |entry| {
                let text = entry.text();
                if !text.is_empty() {
                    let path = PathBuf::from(text.as_str());
                    if path.exists() {
                        if let Some(ref callback) = *on_path_entered_clone.borrow() {
                            callback(path);
                        }
                    }
                }
                *is_editing_path_clone.borrow_mut() = false;
                breadcrumbs_box_clone.set_visible(true);
                path_entry_clone.set_visible(false);
            });
        }

        // Cancel editing on escape key
        {
            let breadcrumbs_box_clone = breadcrumbs_box.clone();
            let path_entry_clone = path_entry.clone();
            let is_editing_path_clone = is_editing_path.clone();

            // Handle Escape key to cancel editing
            let controller = gtk4::EventControllerKey::new();
            controller.connect_key_pressed(move |_, keyval, _, _| {
                // Check if Escape key is pressed
                if keyval == gtk4::gdk::Key::Escape {
                    *is_editing_path_clone.borrow_mut() = false;
                    breadcrumbs_box_clone.set_visible(true);
                    path_entry_clone.set_visible(false);
                    gtk4::glib::Propagation::Stop
                } else {
                    gtk4::glib::Propagation::Proceed
                }
            });
            path_entry.add_controller(controller);
        }

        // Cancel editing when clicking outside - use a timeout to check focus
        {
            let breadcrumbs_box_clone = breadcrumbs_box.clone();
            let path_entry_clone = path_entry.clone();
            let is_editing_path_clone = is_editing_path.clone();
            let path_entry_weak = path_entry.downgrade();

            // Use a timeout to periodically check if entry lost focus
            gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                if let Some(entry) = path_entry_weak.upgrade() {
                    if *is_editing_path_clone.borrow() && !entry.has_focus() {
                        *is_editing_path_clone.borrow_mut() = false;
                        breadcrumbs_box_clone.set_visible(true);
                        path_entry_clone.set_visible(false);
                        gtk4::glib::ControlFlow::Break
                    } else {
                        gtk4::glib::ControlFlow::Continue
                    }
                } else {
                    gtk4::glib::ControlFlow::Break
                }
            });
        }

        // Connect signals
        {
            let on_search_clone = on_search.clone();
            search_entry.connect_search_changed(move |entry| {
                if let Some(ref callback) = *on_search_clone.borrow() {
                    callback(entry.text().to_string());
                }
            });
        }

        {
            let on_view_toggle_clone = on_view_toggle.clone();
            view_toggle_btn.connect_clicked(move |_| {
                if let Some(ref callback) = *on_view_toggle_clone.borrow() {
                    callback();
                }
            });
        }

        {
            let on_new_folder_clone = on_new_folder.clone();
            new_folder_btn.connect_clicked(move |_| {
                if let Some(ref callback) = *on_new_folder_clone.borrow() {
                    callback();
                }
            });
        }

        // Add keyboard shortcut Ctrl+L to show path entry
        {
            let breadcrumbs_box_clone = breadcrumbs_box.clone();
            let path_entry_clone = path_entry.clone();
            let is_editing_path_clone = is_editing_path.clone();

            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, keyval, modifiers, _| {
                // Check for Ctrl+L
                if keyval == gtk4::gdk::Key::l && modifiers & gtk4::gdk::ModifierType::CONTROL_MASK.bits() != 0 {
                    if !*is_editing_path_clone.borrow() {
                        *is_editing_path_clone.borrow_mut() = true;
                        breadcrumbs_box_clone.set_visible(false);
                        path_entry_clone.set_visible(true);
                        path_entry_clone.grab_focus();
                        path_entry_clone.select_region(0, -1);
                        return gtk4::glib::Propagation::Stop;
                    }
                }
                gtk4::glib::Propagation::Proceed
            });
            container.add_controller(key_controller);
        }

        Self {
            container,
            breadcrumbs_box,
            path_entry,
            path_entry_box,
            search_entry,
            search_popover,
            view_toggle_btn,
            is_editing_path,
            on_path_clicked,
            on_path_entered,
            on_search,
            on_view_toggle,
            on_new_folder,
        }
    }

    pub fn container(&self) -> &adw::HeaderBar {
        &self.container
    }

    pub fn set_path(&self, path: &Path) {
        // Update path entry text
        self.path_entry.set_text(&path.to_string_lossy());

        // Clear existing breadcrumbs
        while let Some(child) = self.breadcrumbs_box.first_child() {
            self.breadcrumbs_box.remove(&child);
        }

        let home_dir = dirs::home_dir();
        let path_str = path.to_string_lossy();
        
        // Check if path is under home directory
        let (display_path, is_home_relative) = if let Some(ref home) = home_dir {
            if path.starts_with(home) {
                let relative = path.strip_prefix(home).unwrap_or(path);
                if relative.as_os_str().is_empty() {
                    (String::new(), true)
                } else {
                    (relative.to_string_lossy().to_string(), true)
                }
            } else {
                (path_str.to_string(), false)
            }
        } else {
            (path_str.to_string(), false)
        };

        // Home/Root button
        let home_btn = Button::builder()
            .css_classes(["flat"])
            .build();

        if is_home_relative {
            let home_icon = gtk4::Image::builder()
                .icon_name("user-home-symbolic")
                .build();
            home_btn.set_child(Some(&home_icon));
            home_btn.set_tooltip_text(Some("Home"));
            
            let callback = self.on_path_clicked.clone();
            let home_path = home_dir.clone().unwrap_or_else(|| PathBuf::from("/"));
            home_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *callback.borrow() {
                    cb(home_path.clone());
                }
            });
        } else {
            let root_icon = gtk4::Image::builder()
                .icon_name("drive-harddisk-symbolic")
                .build();
            home_btn.set_child(Some(&root_icon));
            home_btn.set_tooltip_text(Some("Computer"));
            
            let callback = self.on_path_clicked.clone();
            home_btn.connect_clicked(move |_| {
                if let Some(ref cb) = *callback.borrow() {
                    cb(PathBuf::from("/"));
                }
            });
        }
        self.breadcrumbs_box.append(&home_btn);

        // Path segments
        if !display_path.is_empty() {
            let segments: Vec<&str> = display_path.split('/').filter(|s| !s.is_empty()).collect();
            let base_path = if is_home_relative {
                home_dir.unwrap_or_else(|| PathBuf::from("/"))
            } else {
                PathBuf::from("/")
            };
            
            let mut accumulated_path = base_path;

            for (i, segment) in segments.iter().enumerate() {
                accumulated_path = accumulated_path.join(segment);
                let is_last = i == segments.len() - 1;

                // Separator arrow
                let arrow = gtk4::Image::builder()
                    .icon_name("go-next-symbolic")
                    .css_classes(["dim-label"])
                    .pixel_size(12)
                    .build();
                self.breadcrumbs_box.append(&arrow);

                let btn = Button::builder()
                    .label(*segment)
                    .css_classes(["flat"])
                    .build();

                if is_last {
                    btn.add_css_class("current-path");
                }

                let path_for_click = accumulated_path.clone();
                let callback = self.on_path_clicked.clone();
                btn.connect_clicked(move |_| {
                    if let Some(ref cb) = *callback.borrow() {
                        cb(path_for_click.clone());
                    }
                });

                self.breadcrumbs_box.append(&btn);
            }
        }
    }

    pub fn connect_path_clicked<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_path_clicked.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_path_entered<F: Fn(PathBuf) + 'static>(&self, callback: F) {
        *self.on_path_entered.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_search<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_search.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_view_toggle<F: Fn() + 'static>(&self, callback: F) {
        *self.on_view_toggle.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_new_folder<F: Fn() + 'static>(&self, callback: F) {
        *self.on_new_folder.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_view_icon(&self, is_grid: bool) {
        if is_grid {
            self.view_toggle_btn.set_icon_name("view-list-symbolic");
            self.view_toggle_btn.set_tooltip_text(Some("List View"));
        } else {
            self.view_toggle_btn.set_icon_name("view-grid-symbolic");
            self.view_toggle_btn.set_tooltip_text(Some("Grid View"));
        }
    }

    pub fn clear_search(&self) {
        self.search_entry.set_text("");
    }
}
