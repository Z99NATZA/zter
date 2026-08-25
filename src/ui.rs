use std::cell::{Cell, RefCell};
use std::env;
use std::fmt;
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gtk::gdk::prelude::*;
use gtk::prelude::*;
use vte4::prelude::*;

use crate::{
    config::{AppConfig, WallpaperSource},
    identity::{APPLICATION_NAME, ICON_NAME},
    settings::TerminalPadding,
    theme,
};

const DEFAULT_WIDTH: i32 = 960;
const DEFAULT_HEIGHT: i32 = 600;
const WALLPAPER_BLEND_OPERATOR: gtk::cairo::Operator = gtk::cairo::Operator::Screen;
const BUNDLED_WALLPAPER: &[u8] = include_bytes!("../data/wallpapers/zter-wallpaper.png");
const TAB_ID_PREFIX: &str = "zter-tab-";
const TAB_WIDTH: f64 = 220.0;
const TAB_SCROLL_STEP: f64 = 48.0;
const TERMINAL_RESIZE_SETTLE: Duration = Duration::from_millis(120);
const TERMINAL_TOP_BORDER: i32 = 1;

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TabShortcut {
    New,
    Close,
    Previous,
    Next,
}

#[derive(Debug, Eq, PartialEq)]
struct TabTitleState {
    automatic: String,
    manual: Option<String>,
}

impl TabTitleState {
    fn new(automatic: String) -> Self {
        Self {
            automatic,
            manual: None,
        }
    }

    fn displayed(&self) -> &str {
        self.manual.as_deref().unwrap_or(&self.automatic)
    }

    fn update_automatic(&mut self, title: String) {
        self.automatic = title;
    }

    fn save_manual(&mut self, title: &str) {
        let title = sanitize_title(title);
        self.manual = (!title.is_empty()).then_some(title);
    }
}

struct TabHeader {
    tab: gtk::Box,
    title_label: gtk::Label,
    title_stack: gtk::Stack,
    title_entry: gtk::Entry,
    select_button: gtk::Button,
    close_button: gtk::Button,
}

#[derive(Clone, Default)]
struct WallpaperAsset {
    texture: Option<gtk::gdk::Texture>,
}

#[derive(Debug)]
enum WallpaperPreparationError {
    Cairo(gtk::cairo::Error),
    Downscale,
    PixelConversion,
}

impl fmt::Display for WallpaperPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cairo(error) => write!(formatter, "could not blend the wallpaper: {error}"),
            Self::Downscale => formatter.write_str("could not downscale the wallpaper"),
            Self::PixelConversion => {
                formatter.write_str("could not convert the prepared wallpaper to pixels")
            }
        }
    }
}

impl From<gtk::cairo::Error> for WallpaperPreparationError {
    fn from(error: gtk::cairo::Error) -> Self {
        Self::Cairo(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalResizeAction {
    Ignore,
    ApplyNow((i32, i32)),
    Defer,
}

#[derive(Default)]
struct DeferredTerminalResize {
    observed: Option<(i32, i32)>,
    applied: Option<(i32, i32)>,
}

impl DeferredTerminalResize {
    fn observe(&mut self, size: (i32, i32)) -> TerminalResizeAction {
        if size.0 <= 0 || size.1 <= 0 || self.observed == Some(size) {
            return TerminalResizeAction::Ignore;
        }

        self.observed = Some(size);
        if self.applied.is_none() {
            self.applied = Some(size);
            TerminalResizeAction::ApplyNow(size)
        } else {
            TerminalResizeAction::Defer
        }
    }

    fn settle(&mut self) -> Option<(i32, i32)> {
        if self.observed == self.applied {
            return None;
        }

        self.applied = self.observed;
        self.applied
    }
}

pub fn build(application: &gtk::Application, config: &AppConfig) {
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title(APPLICATION_NAME)
        .icon_name(ICON_NAME)
        .default_width(DEFAULT_WIDTH)
        .default_height(DEFAULT_HEIGHT)
        .build();
    window.add_css_class("zter-window");
    theme::install_display_styles(
        &gtk::prelude::WidgetExt::display(&window),
        config.terminal_padding(),
    );
    let wallpaper = prepare_wallpaper_asset(config, &gtk::prelude::WidgetExt::display(&window));

    let notebook = create_notebook();
    let (header, tab_strip, tab_scroller) = create_header(&window, &notebook, config, &wallpaper);
    install_tab_shortcuts(
        &window,
        &notebook,
        &tab_strip,
        &tab_scroller,
        config,
        &wallpaper,
    );
    install_tab_switch_handler(&window, &notebook, &tab_strip, &tab_scroller, config);

    window.set_titlebar(Some(&header));
    window.set_child(Some(&notebook));
    add_terminal_tab(
        &window,
        &notebook,
        &tab_strip,
        &tab_scroller,
        config,
        &wallpaper,
    );
    window.present();
    focus_current_terminal(&notebook);
}

fn create_notebook() -> gtk::Notebook {
    let notebook = gtk::Notebook::new();
    notebook.add_css_class("zter-tabs");
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);
    notebook.set_show_border(false);
    notebook.set_show_tabs(false);
    notebook
}

fn create_header(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
) -> (gtk::WindowHandle, gtk::Box, gtk::ScrolledWindow) {
    let window_handle = gtk::WindowHandle::new();
    window_handle.add_css_class("zter-window-handle");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    header.add_css_class("zter-header");

    let tab_strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_strip.add_css_class("zter-tab-strip");
    tab_strip.set_hexpand(false);

    let scroll_content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    scroll_content.append(&tab_strip);

    let tab_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .has_frame(false)
        .hexpand(true)
        .child(&scroll_content)
        .build();
    tab_scroller.add_css_class("zter-tab-scroller");
    install_tab_strip_scrolling(&tab_scroller);

    let inline_new_tab = create_new_tab_button(
        window,
        notebook,
        &tab_strip,
        &tab_scroller,
        config,
        wallpaper,
    );
    let pinned_new_tab = create_new_tab_button(
        window,
        notebook,
        &tab_strip,
        &tab_scroller,
        config,
        wallpaper,
    );
    pinned_new_tab.set_visible(false);

    let drag_space = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    drag_space.set_hexpand(true);
    scroll_content.append(&inline_new_tab);
    scroll_content.append(&drag_space);
    install_tab_overflow(&tab_scroller, &inline_new_tab, &pinned_new_tab);

    let window_controls = gtk::WindowControls::new(gtk::PackType::End);
    window_controls.set_valign(gtk::Align::Center);

    header.append(&tab_scroller);
    header.append(&pinned_new_tab);
    header.append(&window_controls);
    window_handle.set_child(Some(&header));

    (window_handle, tab_strip, tab_scroller)
}

fn create_new_tab_button(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
) -> gtk::Button {
    let button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .has_frame(false)
        .tooltip_text("New tab (Ctrl+Shift+T)")
        .build();
    button.add_css_class("zter-new-tab");
    button.set_valign(gtk::Align::Center);

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let config = config.clone();
    let wallpaper = wallpaper.clone();
    button.connect_clicked(move |_| {
        let (Some(window), Some(notebook), Some(tab_strip), Some(tab_scroller)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            tab_strip_weak.upgrade(),
            tab_scroller_weak.upgrade(),
        ) else {
            return;
        };
        add_terminal_tab(
            &window,
            &notebook,
            &tab_strip,
            &tab_scroller,
            &config,
            &wallpaper,
        );
    });

    button
}

fn install_tab_overflow(
    scroller: &gtk::ScrolledWindow,
    inline_button: &gtk::Button,
    pinned_button: &gtk::Button,
) {
    let inline_weak = inline_button.downgrade();
    let pinned_weak = pinned_button.downgrade();
    scroller.hadjustment().connect_changed(move |adjustment| {
        let (Some(inline_button), Some(pinned_button)) =
            (inline_weak.upgrade(), pinned_weak.upgrade())
        else {
            return;
        };
        let overflow = adjustment.upper() > adjustment.page_size() + 0.5;
        inline_button.set_visible(!overflow);
        pinned_button.set_visible(overflow);
    });

    let adjustment = scroller.hadjustment();
    let overflow = adjustment.upper() > adjustment.page_size() + 0.5;
    inline_button.set_visible(!overflow);
    pinned_button.set_visible(overflow);
}

fn install_tab_strip_scrolling(scroller: &gtk::ScrolledWindow) {
    let controller = gtk::EventControllerScroll::new(
        gtk::EventControllerScrollFlags::BOTH_AXES | gtk::EventControllerScrollFlags::KINETIC,
    );
    let scroller_weak = scroller.downgrade();
    controller.connect_scroll(move |_, dx, dy| {
        let Some(scroller) = scroller_weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };
        let adjustment = scroller.hadjustment();
        let delta = if dx.abs() > dy.abs() { dx } else { dy };
        let maximum = (adjustment.upper() - adjustment.page_size()).max(adjustment.lower());

        if delta == 0.0 || maximum <= adjustment.lower() {
            return gtk::glib::Propagation::Proceed;
        }

        adjustment.set_value(
            (adjustment.value() + delta * TAB_SCROLL_STEP).clamp(adjustment.lower(), maximum),
        );
        gtk::glib::Propagation::Stop
    });
    scroller.add_controller(controller);
}

fn install_tab_shortcuts(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let config = config.clone();
    let wallpaper = wallpaper.clone();
    controller.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(shortcut) = tab_shortcut(key, modifiers) else {
            return gtk::glib::Propagation::Proceed;
        };
        let (Some(window), Some(notebook), Some(tab_strip), Some(tab_scroller)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            tab_strip_weak.upgrade(),
            tab_scroller_weak.upgrade(),
        ) else {
            return gtk::glib::Propagation::Proceed;
        };

        match shortcut {
            TabShortcut::New => add_terminal_tab(
                &window,
                &notebook,
                &tab_strip,
                &tab_scroller,
                &config,
                &wallpaper,
            ),
            TabShortcut::Close => close_current_tab(&window, &notebook, &tab_strip, &tab_scroller),
            TabShortcut::Previous => notebook.prev_page(),
            TabShortcut::Next => notebook.next_page(),
        }

        gtk::glib::Propagation::Stop
    });

    window.add_controller(controller);
}

fn install_tab_switch_handler(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    config: &AppConfig,
) {
    let window_weak = window.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let fallback_title = default_tab_title(config.shell());
    notebook.connect_switch_page(move |_, page, _| {
        let Some(terminal) = find_terminal(page) else {
            return;
        };
        if let Some(window) = window_weak.upgrade() {
            let title = tab_strip_weak
                .upgrade()
                .and_then(|tab_strip| displayed_tab_title(&tab_strip, &page.widget_name()))
                .unwrap_or_else(|| terminal_display_title(&terminal, &fallback_title));
            set_window_title(&window, &title);
        }
        if let (Some(tab_strip), Some(tab_scroller)) =
            (tab_strip_weak.upgrade(), tab_scroller_weak.upgrade())
        {
            sync_header_tabs_for_id(&tab_strip, &tab_scroller, Some(page.widget_name().as_str()));
        }
        terminal.grab_focus();
    });
}

fn add_terminal_tab(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
) {
    let terminal = create_terminal(config);
    let content = create_content(&terminal, config, wallpaper);
    let fallback_title = default_tab_title(config.shell());
    let tab_id = next_tab_id();
    content.set_widget_name(&tab_id);
    let header = create_header_tab(&fallback_title, &tab_id);
    let title_state = Rc::new(RefCell::new(TabTitleState::new(fallback_title.clone())));

    let page_number = notebook.append_page(&content, None::<&gtk::Widget>);
    tab_strip.append(&header.tab);

    install_tab_title_editing(window, notebook, &content, &header, title_state.clone());

    let notebook_weak = notebook.downgrade();
    let content_weak = content.downgrade();
    header.select_button.connect_clicked(move |_| {
        let (Some(notebook), Some(content)) = (notebook_weak.upgrade(), content_weak.upgrade())
        else {
            return;
        };
        if let Some(page_number) = notebook.page_num(&content) {
            notebook.set_current_page(Some(page_number));
        }
    });

    install_tab_drag_and_drop(
        notebook,
        tab_strip,
        tab_scroller,
        &header.tab,
        &header.select_button,
        &tab_id,
    );

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let content_weak = content.downgrade();
    header.close_button.connect_clicked(move |_| {
        let (Some(window), Some(notebook), Some(tab_strip), Some(tab_scroller), Some(content)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            tab_strip_weak.upgrade(),
            tab_scroller_weak.upgrade(),
            content_weak.upgrade(),
        ) else {
            return;
        };
        close_tab(&window, &notebook, &tab_strip, &tab_scroller, &content);
    });

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let content_weak = content.downgrade();
    terminal.connect_child_exited(move |_, _| {
        let (Some(window), Some(notebook), Some(tab_strip), Some(tab_scroller), Some(content)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            tab_strip_weak.upgrade(),
            tab_scroller_weak.upgrade(),
            content_weak.upgrade(),
        ) else {
            return;
        };
        close_tab(&window, &notebook, &tab_strip, &tab_scroller, &content);
    });

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let content_weak = content.downgrade();
    let fallback_for_title = fallback_title.clone();
    let title_label = header.title_label.clone();
    terminal.connect_window_title_changed(move |terminal| {
        let automatic = terminal_display_title(terminal, &fallback_for_title);
        let title = {
            let mut state = title_state.borrow_mut();
            state.update_automatic(automatic);
            state.manual.is_none().then(|| state.displayed().to_owned())
        };
        let Some(title) = title else {
            return;
        };
        title_label.set_text(&title);

        let (Some(window), Some(notebook), Some(content)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            content_weak.upgrade(),
        ) else {
            return;
        };
        if notebook.page_num(&content) == notebook.current_page() {
            set_window_title(&window, &title);
        }
    });

    notebook.set_current_page(Some(page_number));
    set_window_title(window, &fallback_title);
    sync_header_tabs(notebook, tab_strip, tab_scroller);
    terminal.grab_focus();

    spawn_shell(&terminal, config);
}

fn create_header_tab(title: &str, tab_id: &str) -> TabHeader {
    let label = gtk::Label::new(Some(title));
    label.add_css_class("zter-tab-title");
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_hexpand(true);
    label.set_max_width_chars(28);
    label.set_xalign(0.5);

    let select_button = gtk::Button::builder()
        .has_frame(false)
        .hexpand(true)
        .child(&label)
        .build();
    select_button.add_css_class("zter-tab-select");

    let title_entry = gtk::Entry::new();
    title_entry.add_css_class("zter-tab-title-entry");
    title_entry.set_hexpand(true);
    title_entry.set_max_length(256);
    title_entry.set_width_chars(1);
    title_entry.set_max_width_chars(28);

    let title_stack = gtk::Stack::new();
    title_stack.set_hexpand(true);
    title_stack.set_hhomogeneous(true);
    title_stack.set_vhomogeneous(true);
    title_stack.add_named(&select_button, Some("display"));
    title_stack.add_named(&title_entry, Some("editor"));
    title_stack.set_visible_child_name("display");

    let close_button = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .has_frame(false)
        .tooltip_text("Close tab (Ctrl+Shift+W)")
        .build();
    close_button.add_css_class("zter-tab-close");
    close_button.set_valign(gtk::Align::Center);

    let tab = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab.add_css_class("zter-header-tab");
    tab.set_hexpand(false);
    tab.set_widget_name(tab_id);
    tab.append(&title_stack);
    tab.append(&close_button);

    TabHeader {
        tab,
        title_label: label,
        title_stack,
        title_entry,
        select_button,
        close_button,
    }
}

fn install_tab_title_editing(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    content: &gtk::Overlay,
    header: &TabHeader,
    title_state: Rc<RefCell<TabTitleState>>,
) {
    let editor_had_focus = Rc::new(Cell::new(false));
    let double_click = gtk::GestureClick::new();
    double_click.set_button(gtk::gdk::BUTTON_PRIMARY);
    double_click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let stack_weak = header.title_stack.downgrade();
    let entry_weak = header.title_entry.downgrade();
    let state = title_state.clone();
    let focus_state = editor_had_focus.clone();
    double_click.connect_pressed(move |gesture, press_count, _, _| {
        if press_count != 2 {
            return;
        }
        let (Some(stack), Some(entry)) = (stack_weak.upgrade(), entry_weak.upgrade()) else {
            return;
        };
        if stack.visible_child_name().as_deref() != Some("display") {
            return;
        }
        // Stop GtkWindowHandle from treating the same press as a titlebar double-click.
        gesture.set_state(gtk::EventSequenceState::Claimed);
        entry.set_text(state.borrow().displayed());
        focus_state.set(false);
        stack.set_visible_child_name("editor");

        let entry_weak = entry.downgrade();
        let focus_state = focus_state.clone();
        stack.add_tick_callback(move |stack, _| {
            if stack.visible_child_name().as_deref() != Some("editor") {
                return gtk::glib::ControlFlow::Break;
            }
            let Some(entry) = entry_weak.upgrade() else {
                return gtk::glib::ControlFlow::Break;
            };
            if !entry.is_mapped() {
                return gtk::glib::ControlFlow::Continue;
            }
            if entry.grab_focus() {
                focus_state.set(true);
                entry.select_region(0, -1);
            }
            gtk::glib::ControlFlow::Break
        });
    });
    header.title_stack.add_controller(double_click);

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let content_weak = content.downgrade();
    let stack_weak = header.title_stack.downgrade();
    let entry_weak = header.title_entry.downgrade();
    let label_weak = header.title_label.downgrade();
    let state = title_state.clone();
    let save = Rc::new(move |focus_terminal: bool| {
        let (Some(window), Some(notebook), Some(content), Some(stack), Some(entry), Some(label)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            content_weak.upgrade(),
            stack_weak.upgrade(),
            entry_weak.upgrade(),
            label_weak.upgrade(),
        ) else {
            return;
        };

        let title = {
            let mut state = state.borrow_mut();
            state.save_manual(&entry.text());
            state.displayed().to_owned()
        };
        label.set_text(&title);
        stack.set_visible_child_name("display");
        if notebook.page_num(&content) == notebook.current_page() {
            set_window_title(&window, &title);
            if focus_terminal {
                focus_current_terminal(&notebook);
            }
        }
    });

    let save_on_activate = save.clone();
    header
        .title_entry
        .connect_activate(move |_| save_on_activate(true));

    let stack_weak = header.title_stack.downgrade();
    let save_on_focus_loss = save.clone();
    let focus_state = editor_had_focus;
    header.title_entry.connect_has_focus_notify(move |entry| {
        let Some(stack) = stack_weak.upgrade() else {
            return;
        };
        if entry.has_focus() {
            focus_state.set(true);
        } else if focus_state.replace(false)
            && stack.visible_child_name().as_deref() == Some("editor")
        {
            save_on_focus_loss(false);
        }
    });

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let notebook_weak = notebook.downgrade();
    let stack_weak = header.title_stack.downgrade();
    let entry_weak = header.title_entry.downgrade();
    let label_weak = header.title_label.downgrade();
    let state = title_state;
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key != gtk::gdk::Key::Escape {
            return gtk::glib::Propagation::Proceed;
        }
        let (Some(notebook), Some(stack), Some(entry), Some(label)) = (
            notebook_weak.upgrade(),
            stack_weak.upgrade(),
            entry_weak.upgrade(),
            label_weak.upgrade(),
        ) else {
            return gtk::glib::Propagation::Stop;
        };
        let title = state.borrow().displayed().to_owned();
        entry.set_text(&title);
        label.set_text(&title);
        stack.set_visible_child_name("display");
        focus_current_terminal(&notebook);
        gtk::glib::Propagation::Stop
    });
    header.title_entry.add_controller(key_controller);
}

fn next_tab_id() -> String {
    format!(
        "{TAB_ID_PREFIX}{}",
        NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn install_tab_drag_and_drop(
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    tab: &gtk::Box,
    drag_handle: &gtk::Button,
    tab_id: &str,
) {
    let drag_source = gtk::DragSource::new();
    drag_source.set_actions(gtk::gdk::DragAction::MOVE);
    let source_id = tab_id.to_owned();
    drag_source.connect_prepare(move |_, _, _| {
        Some(gtk::gdk::ContentProvider::for_value(&source_id.to_value()))
    });
    drag_handle.add_controller(drag_source);

    let drop_target = gtk::DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let target_id = tab_id.to_owned();
    drop_target.connect_drop(move |_, value, _, _| {
        let Ok(source_id) = value.get::<String>() else {
            return false;
        };
        let (Some(notebook), Some(tab_strip), Some(tab_scroller)) = (
            notebook_weak.upgrade(),
            tab_strip_weak.upgrade(),
            tab_scroller_weak.upgrade(),
        ) else {
            return false;
        };

        reorder_tab(&notebook, &tab_strip, &tab_scroller, &source_id, &target_id)
    });
    tab.add_controller(drop_target);
}

fn reorder_tab(
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    source_id: &str,
    target_id: &str,
) -> bool {
    if source_id == target_id {
        return false;
    }

    let Some(source_content) = notebook_page_by_id(notebook, source_id) else {
        return false;
    };
    let Some(target_content) = notebook_page_by_id(notebook, target_id) else {
        return false;
    };
    let Some(source_position) = notebook.page_num(&source_content) else {
        return false;
    };
    let Some(target_position) = notebook.page_num(&target_content) else {
        return false;
    };
    let Some(source_tab) = tab_by_id(tab_strip, source_id) else {
        return false;
    };
    let Some(target_tab) = tab_by_id(tab_strip, target_id) else {
        return false;
    };

    notebook
        .page(&source_content)
        .set_position(target_position as i32);
    if source_position < target_position {
        tab_strip.reorder_child_after(&source_tab, Some(&target_tab));
    } else {
        let previous = target_tab.prev_sibling();
        tab_strip.reorder_child_after(&source_tab, previous.as_ref());
    }
    sync_header_tabs(notebook, tab_strip, tab_scroller);
    true
}

fn notebook_page_by_id(notebook: &gtk::Notebook, tab_id: &str) -> Option<gtk::Widget> {
    (0..notebook.n_pages()).find_map(|position| {
        let page = notebook.nth_page(Some(position))?;
        (page.widget_name() == tab_id).then_some(page)
    })
}

fn tab_by_id(tab_strip: &gtk::Box, tab_id: &str) -> Option<gtk::Widget> {
    let mut child = tab_strip.first_child();
    while let Some(tab) = child {
        if tab.widget_name() == tab_id {
            return Some(tab);
        }
        child = tab.next_sibling();
    }
    None
}

fn displayed_tab_title(tab_strip: &gtk::Box, tab_id: &str) -> Option<String> {
    let tab = tab_by_id(tab_strip, tab_id)?;
    find_tab_title_label(&tab).map(|label| label.text().to_string())
}

fn find_tab_title_label(widget: &gtk::Widget) -> Option<gtk::Label> {
    if let Ok(label) = widget.clone().downcast::<gtk::Label>() {
        if label.has_css_class("zter-tab-title") {
            return Some(label);
        }
    }

    let mut child = widget.first_child();
    while let Some(current) = child {
        if let Some(label) = find_tab_title_label(&current) {
            return Some(label);
        }
        child = current.next_sibling();
    }
    None
}

fn sync_header_tabs(
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
) {
    let active_id = notebook
        .current_page()
        .and_then(|position| notebook.nth_page(Some(position)))
        .map(|page| page.widget_name());
    sync_header_tabs_for_id(tab_strip, tab_scroller, active_id.as_deref());
}

fn sync_header_tabs_for_id(
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    active_id: Option<&str>,
) {
    let mut active_position = None;
    let mut active_tab = None;
    let mut position = 0;
    let mut child = tab_strip.first_child();
    while let Some(tab) = child {
        let is_active = active_id.is_some_and(|id| id == tab.widget_name());
        if is_active {
            tab.add_css_class("zter-tab-active");
            active_position = Some(position);
            active_tab = Some(tab.clone());
        } else {
            tab.remove_css_class("zter-tab-active");
        }
        position += 1;
        child = tab.next_sibling();
    }

    if let (Some(active_tab), Some(active_position)) = (active_tab, active_position) {
        reveal_tab(tab_scroller, &active_tab, active_position);
    }
}

fn reveal_tab(tab_scroller: &gtk::ScrolledWindow, tab: &gtk::Widget, tab_position: u32) {
    let frames_before_reveal = Cell::new(2);
    let tab_weak = tab.downgrade();
    tab_scroller.add_tick_callback(move |tab_scroller, _| {
        let remaining = frames_before_reveal.get();
        if remaining > 0 {
            frames_before_reveal.set(remaining - 1);
            return gtk::glib::ControlFlow::Continue;
        }

        let Some(tab) = tab_weak.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        if !tab.has_css_class("zter-tab-active") {
            return gtk::glib::ControlFlow::Break;
        }

        let adjustment = tab_scroller.hadjustment();
        let visible_start = adjustment.value();
        let visible_end = visible_start + adjustment.page_size();
        let tab_start = f64::from(tab_position) * TAB_WIDTH;
        let tab_end = tab_start + TAB_WIDTH;
        let requested = if tab_start < visible_start {
            tab_start
        } else if tab_end > visible_end {
            tab_end - adjustment.page_size()
        } else {
            return gtk::glib::ControlFlow::Break;
        };
        let maximum = (adjustment.upper() - adjustment.page_size()).max(adjustment.lower());
        adjustment.set_value(requested.clamp(adjustment.lower(), maximum));
        gtk::glib::ControlFlow::Break
    });
}

fn close_current_tab(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
) {
    let Some(page_number) = notebook.current_page() else {
        return;
    };
    let Some(content) = notebook.nth_page(Some(page_number)) else {
        return;
    };
    close_tab(window, notebook, tab_strip, tab_scroller, &content);
}

fn close_tab(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    content: &impl IsA<gtk::Widget>,
) {
    let Some(page_number) = notebook.page_num(content) else {
        return;
    };
    if let Some(tab) = tab_by_id(tab_strip, &content.widget_name()) {
        tab_strip.remove(&tab);
    }
    notebook.remove_page(Some(page_number));

    if notebook.n_pages() == 0 {
        window.close();
    } else {
        sync_header_tabs(notebook, tab_strip, tab_scroller);
        focus_current_terminal(notebook);
    }
}

fn focus_current_terminal(notebook: &gtk::Notebook) {
    let Some(page_number) = notebook.current_page() else {
        return;
    };
    let Some(page) = notebook.nth_page(Some(page_number)) else {
        return;
    };
    if let Some(terminal) = find_terminal(&page) {
        terminal.grab_focus();
    }
}

fn find_terminal(widget: &gtk::Widget) -> Option<vte4::Terminal> {
    if let Ok(terminal) = widget.clone().downcast::<vte4::Terminal>() {
        return Some(terminal);
    }

    let mut child = widget.first_child();
    while let Some(current) = child {
        if let Some(terminal) = find_terminal(&current) {
            return Some(terminal);
        }
        child = current.next_sibling();
    }
    None
}

fn tab_shortcut(key: gtk::gdk::Key, modifiers: gtk::gdk::ModifierType) -> Option<TabShortcut> {
    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let shift = modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK);

    if control && shift {
        return match key.to_lower() {
            gtk::gdk::Key::t => Some(TabShortcut::New),
            gtk::gdk::Key::w => Some(TabShortcut::Close),
            _ => None,
        };
    }

    if control {
        return match key {
            gtk::gdk::Key::Page_Up => Some(TabShortcut::Previous),
            gtk::gdk::Key::Page_Down => Some(TabShortcut::Next),
            _ => None,
        };
    }

    None
}

fn default_tab_title(shell: &str) -> String {
    let shell_name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(shell);
    format!("{shell_name} in zter")
}

fn terminal_display_title(terminal: &vte4::Terminal, fallback: &str) -> String {
    display_title(terminal.window_title().as_deref(), fallback)
}

fn display_title(title: Option<&str>, fallback: &str) -> String {
    let Some(title) = title else {
        return fallback.to_owned();
    };
    let title = sanitize_title(title);
    if title.is_empty() {
        fallback.to_owned()
    } else {
        title
    }
}

fn sanitize_title(title: &str) -> String {
    title
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn set_window_title(window: &gtk::ApplicationWindow, title: &str) {
    window.set_title(Some(&format!("{title} — {APPLICATION_NAME}")));
}

fn create_terminal(config: &AppConfig) -> vte4::Terminal {
    let terminal = vte4::Terminal::new();
    terminal.add_css_class("zter-terminal");
    terminal.set_hexpand(true);
    terminal.set_vexpand(true);
    terminal.set_scrollback_lines(config.scrollback_lines());
    terminal.set_scroll_on_keystroke(true);
    terminal.set_mouse_autohide(true);
    terminal.set_allow_hyperlink(true);
    terminal.set_font(Some(&terminal_font(config)));
    install_clipboard_shortcuts(&terminal);
    theme::apply_to(&terminal, config.theme());

    terminal
}

fn terminal_font(config: &AppConfig) -> gtk::pango::FontDescription {
    let mut font = gtk::pango::FontDescription::new();
    font.set_family(config.font_family());
    font.set_size((config.font_size() * f64::from(gtk::pango::SCALE)).round() as i32);
    font
}

fn install_clipboard_shortcuts(terminal: &vte4::Terminal) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let terminal_weak = terminal.downgrade();
    controller.connect_key_pressed(move |_, key, _, modifiers| {
        let is_terminal_shortcut = modifiers
            .contains(gtk::gdk::ModifierType::CONTROL_MASK | gtk::gdk::ModifierType::SHIFT_MASK);

        if !is_terminal_shortcut {
            return gtk::glib::Propagation::Proceed;
        }

        let Some(terminal) = terminal_weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };

        match key.to_lower() {
            gtk::gdk::Key::c => terminal.copy_clipboard_format(vte4::Format::Text),
            gtk::gdk::Key::v => terminal.paste_clipboard(),
            _ => return gtk::glib::Propagation::Proceed,
        }

        gtk::glib::Propagation::Stop
    });

    terminal.add_controller(controller);
}

fn create_content(
    terminal: &vte4::Terminal,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
) -> gtk::Overlay {
    let overlay = gtk::Overlay::new();
    overlay.add_css_class("zter-content");
    overlay.set_overflow(gtk::Overflow::Hidden);

    let background = create_background(wallpaper);
    let terminal_viewport = create_terminal_viewport(terminal, config.terminal_padding());
    overlay.set_child(Some(&background));
    overlay.add_overlay(&terminal_viewport);

    overlay
}

fn create_terminal_viewport(terminal: &vte4::Terminal, padding: TerminalPadding) -> gtk::Fixed {
    let viewport = gtk::Fixed::new();
    viewport.set_hexpand(true);
    viewport.set_vexpand(true);
    viewport.set_overflow(gtk::Overflow::Hidden);
    viewport.put(terminal, 0.0, 0.0);
    install_deferred_terminal_resize(&viewport, terminal, padding);
    viewport
}

fn install_deferred_terminal_resize(
    viewport: &gtk::Fixed,
    terminal: &vte4::Terminal,
    padding: TerminalPadding,
) {
    let resize = Rc::new(RefCell::new(DeferredTerminalResize::default()));
    let pending = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));
    let terminal_weak = terminal.downgrade();

    viewport.add_tick_callback(move |viewport, _| {
        let size = (viewport.width(), viewport.height());
        match resize.borrow_mut().observe(size) {
            TerminalResizeAction::Ignore => {}
            TerminalResizeAction::ApplyNow(size) => {
                if let Some(terminal) = terminal_weak.upgrade() {
                    apply_terminal_viewport_size(&terminal, size, padding);
                }
            }
            TerminalResizeAction::Defer => {
                if let Some(source) = pending.borrow_mut().take() {
                    source.remove();
                }

                let resize = resize.clone();
                let pending_for_timeout = pending.clone();
                let terminal_weak = terminal_weak.clone();
                let source = gtk::glib::timeout_add_local_once(TERMINAL_RESIZE_SETTLE, move || {
                    pending_for_timeout.borrow_mut().take();
                    let Some(size) = resize.borrow_mut().settle() else {
                        return;
                    };
                    if let Some(terminal) = terminal_weak.upgrade() {
                        apply_terminal_viewport_size(&terminal, size, padding);
                    }
                });
                *pending.borrow_mut() = Some(source);
            }
        }

        gtk::glib::ControlFlow::Continue
    });
}

fn apply_terminal_viewport_size(
    terminal: &vte4::Terminal,
    (width, height): (i32, i32),
    padding: TerminalPadding,
) {
    let (columns, rows) = terminal_grid_size(
        (width, height),
        padding,
        (terminal.char_width(), terminal.char_height()),
    );
    terminal.set_size(columns, rows);
    terminal.set_size_request(width, height);
}

fn terminal_grid_size(
    (width, height): (i32, i32),
    padding: TerminalPadding,
    (cell_width, cell_height): (i64, i64),
) -> (i64, i64) {
    let horizontal_padding = i32::from(padding.left()) + i32::from(padding.right());
    let vertical_padding =
        i32::from(padding.top()) + i32::from(padding.bottom()) + TERMINAL_TOP_BORDER;
    let content_width = i64::from((width - horizontal_padding).max(1));
    let content_height = i64::from((height - vertical_padding).max(1));

    (
        (content_width / cell_width.max(1)).max(1),
        (content_height / cell_height.max(1)).max(1),
    )
}

fn prepare_wallpaper_asset(config: &AppConfig, display: &gtk::gdk::Display) -> WallpaperAsset {
    let Some(source) = config.wallpaper() else {
        return WallpaperAsset::default();
    };
    let wallpaper = match load_wallpaper(source) {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            eprintln!(
                "zter: could not load the bundled wallpaper: {error}; using the theme background"
            );
            return WallpaperAsset::default();
        }
    };
    let display_size = display_pixel_size(display);
    let wallpaper = match prepare_wallpaper_pixels(
        &wallpaper,
        display_size,
        &theme::background_color(config.theme()),
        config.wallpaper_opacity(),
    ) {
        Ok(wallpaper) => wallpaper,
        Err(error) => {
            eprintln!("zter: {error}; using the theme background");
            return WallpaperAsset::default();
        }
    };

    WallpaperAsset {
        texture: Some(gtk::gdk::Texture::for_pixbuf(&wallpaper)),
    }
}

fn display_pixel_size(display: &gtk::gdk::Display) -> (i32, i32) {
    let monitors = display.monitors();
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;

    for index in 0..monitors.n_items() {
        let Some(monitor) = monitors
            .item(index)
            .and_then(|item| item.downcast::<gtk::gdk::Monitor>().ok())
        else {
            continue;
        };
        let geometry = monitor.geometry();
        let scale = monitor.scale_factor().max(1);
        width = width.max(geometry.width().saturating_mul(scale));
        height = height.max(geometry.height().saturating_mul(scale));
    }

    (width, height)
}

fn prepare_wallpaper_pixels(
    wallpaper: &gtk::gdk_pixbuf::Pixbuf,
    display_size: (i32, i32),
    background_color: &gtk::gdk::RGBA,
    opacity: f64,
) -> Result<gtk::gdk_pixbuf::Pixbuf, WallpaperPreparationError> {
    let size = downscaled_wallpaper_size((wallpaper.width(), wallpaper.height()), display_size);
    let wallpaper = if size == (wallpaper.width(), wallpaper.height()) {
        wallpaper.clone()
    } else {
        wallpaper
            .scale_simple(size.0, size.1, gtk::gdk_pixbuf::InterpType::Hyper)
            .ok_or(WallpaperPreparationError::Downscale)?
    };
    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, size.0, size.1)?;
    let context = gtk::cairo::Context::new(&surface)?;
    context.set_source_color(background_color);
    context.paint()?;
    context.set_operator(WALLPAPER_BLEND_OPERATOR);
    context.set_source_pixbuf(&wallpaper, 0.0, 0.0);
    context.paint_with_alpha(opacity)?;
    surface.flush();

    gtk::gdk::pixbuf_get_from_surface(surface.as_ref(), 0, 0, size.0, size.1)
        .ok_or(WallpaperPreparationError::PixelConversion)
}

fn downscaled_wallpaper_size(
    (source_width, source_height): (i32, i32),
    (display_width, display_height): (i32, i32),
) -> (i32, i32) {
    let scale = (f64::from(display_width.max(1)) / f64::from(source_width.max(1)))
        .max(f64::from(display_height.max(1)) / f64::from(source_height.max(1)))
        .min(1.0);

    (
        (f64::from(source_width) * scale).ceil().max(1.0) as i32,
        (f64::from(source_height) * scale).ceil().max(1.0) as i32,
    )
}

fn create_background(wallpaper: &WallpaperAsset) -> gtk::Picture {
    let background = gtk::Picture::new();
    background.add_css_class("zter-background");
    background.set_can_target(false);
    background.set_can_shrink(true);
    background.set_content_fit(gtk::ContentFit::Cover);
    background.set_hexpand(true);
    background.set_vexpand(true);
    background.set_paintable(wallpaper.texture.as_ref());

    background
}

fn load_wallpaper(source: &WallpaperSource) -> Result<gtk::gdk_pixbuf::Pixbuf, gtk::glib::Error> {
    match source {
        WallpaperSource::Bundled => load_bundled_wallpaper(),
        WallpaperSource::File(path) => match gtk::gdk_pixbuf::Pixbuf::from_file(path) {
            Ok(wallpaper) => Ok(wallpaper),
            Err(error) => {
                eprintln!(
                    "zter: warning: could not load wallpaper {}: {error}; using the bundled wallpaper",
                    path.display()
                );
                load_bundled_wallpaper()
            }
        },
    }
}

fn load_bundled_wallpaper() -> Result<gtk::gdk_pixbuf::Pixbuf, gtk::glib::Error> {
    gtk::gdk_pixbuf::Pixbuf::from_read(Cursor::new(BUNDLED_WALLPAPER))
}

fn spawn_shell(terminal: &vte4::Terminal, config: &AppConfig) {
    let argv = [config.shell()];
    let environment: Vec<String> = env::vars_os()
        .filter_map(|(key, value)| {
            let key = key.into_string().ok()?;
            let value = value.into_string().ok()?;
            Some(format!("{key}={value}"))
        })
        .collect();
    let environment: Vec<&str> = environment.iter().map(String::as_str).collect();
    let terminal_for_error = terminal.clone();

    terminal.spawn_async(
        vte4::PtyFlags::DEFAULT,
        Some(config.working_directory()),
        &argv,
        &environment,
        gtk::glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        None::<&gtk::gio::Cancellable>,
        move |result| {
            if let Err(error) = result {
                eprintln!("zter: could not start the shell: {error}");
                terminal_for_error
                    .feed(format!("zter: could not start the shell: {error}\r\n").as_bytes());
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_resize_applies_initial_size_and_defers_until_latest_size_settles() {
        let mut resize = DeferredTerminalResize::default();

        assert_eq!(
            resize.observe((960, 600)),
            TerminalResizeAction::ApplyNow((960, 600))
        );
        assert_eq!(resize.observe((960, 600)), TerminalResizeAction::Ignore);
        assert_eq!(resize.observe((900, 580)), TerminalResizeAction::Defer);
        assert_eq!(resize.observe((840, 560)), TerminalResizeAction::Defer);
        assert_eq!(resize.settle(), Some((840, 560)));
        assert_eq!(resize.settle(), None);
    }

    #[test]
    fn terminal_grid_excludes_padding_and_the_content_divider() {
        let padding = TerminalPadding::new(10, 20, 30, 40);

        assert_eq!(terminal_grid_size((860, 541), padding, (10, 20)), (80, 25));
    }

    #[test]
    fn tab_shortcuts_cover_creation_closing_and_navigation() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;

        assert_eq!(
            tab_shortcut(gtk::gdk::Key::t, control_shift),
            Some(TabShortcut::New)
        );
        assert_eq!(
            tab_shortcut(gtk::gdk::Key::w, control_shift),
            Some(TabShortcut::Close)
        );
        assert_eq!(
            tab_shortcut(gtk::gdk::Key::Page_Up, control),
            Some(TabShortcut::Previous)
        );
        assert_eq!(
            tab_shortcut(gtk::gdk::Key::Page_Down, control),
            Some(TabShortcut::Next)
        );
        assert_eq!(tab_shortcut(gtk::gdk::Key::c, control_shift), None);
    }

    #[test]
    fn default_tab_title_uses_the_shell_executable_name() {
        assert_eq!(default_tab_title("/bin/bash"), "bash in zter");
        assert_eq!(default_tab_title("fish"), "fish in zter");
    }

    #[test]
    fn terminal_title_is_plain_single_line_text_with_a_fallback() {
        assert_eq!(
            display_title(Some("  codex\ncli  "), "bash in zter"),
            "codex cli"
        );
        assert_eq!(display_title(Some("\n\t"), "bash in zter"), "bash in zter");
        assert_eq!(display_title(None, "bash in zter"), "bash in zter");
    }

    #[test]
    fn manual_tab_title_survives_automatic_title_updates() {
        let mut state = TabTitleState::new("bash in zter".to_owned());

        state.save_manual("  project\nserver  ");
        state.update_automatic("vim main.rs".to_owned());

        assert_eq!(state.displayed(), "project server");
    }

    #[test]
    fn empty_manual_tab_title_returns_to_latest_automatic_title() {
        let mut state = TabTitleState::new("bash in zter".to_owned());
        state.save_manual("project");
        state.update_automatic("codex".to_owned());

        state.save_manual(" \n\t ");

        assert_eq!(state.displayed(), "codex");
        assert_eq!(state.manual, None);
    }

    #[test]
    fn wallpaper_downscales_to_cover_the_display_without_upscaling() {
        assert_eq!(
            downscaled_wallpaper_size((3840, 2160), (1920, 1080)),
            (1920, 1080)
        );
        assert_eq!(
            downscaled_wallpaper_size((1672, 941), (1920, 1080)),
            (1672, 941)
        );
    }

    #[test]
    fn wallpaper_keeps_pixels_needed_to_cover_a_different_aspect_ratio() {
        assert_eq!(
            downscaled_wallpaper_size((4000, 1000), (1920, 1080)),
            (4000, 1000)
        );
        assert_eq!(
            downscaled_wallpaper_size((1000, 4000), (1920, 1080)),
            (1000, 4000)
        );
    }

    #[test]
    fn bundled_wallpaper_decodes_as_a_wide_image() {
        let wallpaper = load_wallpaper(&WallpaperSource::Bundled).unwrap();

        assert!(wallpaper.width() > wallpaper.height());
        assert!(wallpaper.width() > 0);
        assert!(wallpaper.height() > 0);
    }

    #[test]
    fn wallpaper_is_blended_once_into_opaque_display_pixels() {
        let wallpaper = load_wallpaper(&WallpaperSource::Bundled).unwrap();
        let prepared = prepare_wallpaper_pixels(
            &wallpaper,
            (960, 600),
            &theme::background_color(crate::settings::Theme::OneHalfDark),
            0.15,
        )
        .unwrap();

        assert_eq!(
            (prepared.width(), prepared.height()),
            downscaled_wallpaper_size((wallpaper.width(), wallpaper.height()), (960, 600))
        );
        assert_eq!(prepared.n_channels(), 4);
        let pixels = prepared.read_pixel_bytes();
        let pixels = pixels.as_ref();
        for (x, y) in [
            (0, 0),
            (prepared.width() / 2, prepared.height() / 2),
            (prepared.width() - 1, prepared.height() - 1),
        ] {
            let alpha =
                usize::try_from(y * prepared.rowstride() + x * prepared.n_channels() + 3).unwrap();
            assert_eq!(pixels[alpha], u8::MAX);
        }
    }

    #[test]
    fn unreadable_external_wallpaper_falls_back_to_the_bundled_image() {
        let path =
            env::temp_dir().join(format!("zter-invalid-wallpaper-{}.png", std::process::id()));
        std::fs::write(&path, b"not an image").unwrap();

        let wallpaper = load_wallpaper(&WallpaperSource::File(path.clone())).unwrap();

        std::fs::remove_file(path).unwrap();
        assert!(wallpaper.width() > wallpaper.height());
    }

    #[test]
    fn wallpaper_uses_screen_blending() {
        assert_eq!(WALLPAPER_BLEND_OPERATOR, gtk::cairo::Operator::Screen);
    }
}
