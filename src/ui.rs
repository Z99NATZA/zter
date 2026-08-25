use std::cell::Cell;
use std::env;
use std::f64::consts::{FRAC_PI_2, PI};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use gtk::gdk::prelude::GdkCairoContextExt;
use gtk::prelude::*;
use vte4::prelude::*;

use crate::{
    config::AppConfig,
    identity::{APPLICATION_NAME, ICON_NAME},
    theme,
};

const DEFAULT_WIDTH: i32 = 960;
const DEFAULT_HEIGHT: i32 = 600;
const WINDOW_CORNER_RADIUS: f64 = 12.0;
const WALLPAPER_BLEND_OPERATOR: gtk::cairo::Operator = gtk::cairo::Operator::Screen;
const TAB_ID_PREFIX: &str = "zter-tab-";
const TAB_WIDTH: f64 = 220.0;
const TAB_SCROLL_STEP: f64 = 48.0;

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TabShortcut {
    New,
    Close,
    Previous,
    Next,
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

    let notebook = create_notebook();
    let (header, tab_strip, tab_scroller) = create_header(&window, &notebook, config);
    install_tab_shortcuts(&window, &notebook, &tab_strip, &tab_scroller, config);
    install_tab_switch_handler(&window, &notebook, &tab_strip, &tab_scroller, config);

    window.set_titlebar(Some(&header));
    window.set_child(Some(&notebook));
    add_terminal_tab(&window, &notebook, &tab_strip, &tab_scroller, config);
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

    let inline_new_tab = create_new_tab_button(window, notebook, &tab_strip, &tab_scroller, config);
    let pinned_new_tab = create_new_tab_button(window, notebook, &tab_strip, &tab_scroller, config);
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
    button.connect_clicked(move |_| {
        let (Some(window), Some(notebook), Some(tab_strip), Some(tab_scroller)) = (
            window_weak.upgrade(),
            notebook_weak.upgrade(),
            tab_strip_weak.upgrade(),
            tab_scroller_weak.upgrade(),
        ) else {
            return;
        };
        add_terminal_tab(&window, &notebook, &tab_strip, &tab_scroller, &config);
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
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let config = config.clone();
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
            TabShortcut::New => {
                add_terminal_tab(&window, &notebook, &tab_strip, &tab_scroller, &config)
            }
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
            update_window_title(&window, &terminal, &fallback_title);
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
) {
    let terminal = create_terminal(config);
    let content = create_content(&terminal, config);
    let fallback_title = default_tab_title(config.shell());
    let tab_id = next_tab_id();
    content.set_widget_name(&tab_id);
    let (tab, title_label, select_button, close_button) =
        create_header_tab(&fallback_title, &tab_id);

    let page_number = notebook.append_page(&content, None::<&gtk::Widget>);
    tab_strip.append(&tab);

    let notebook_weak = notebook.downgrade();
    let content_weak = content.downgrade();
    select_button.connect_clicked(move |_| {
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
        &tab,
        &select_button,
        &tab_id,
    );

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let content_weak = content.downgrade();
    close_button.connect_clicked(move |_| {
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
    terminal.connect_window_title_changed(move |terminal| {
        let title = terminal_display_title(terminal, &fallback_for_title);
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

fn create_header_tab(
    title: &str,
    tab_id: &str,
) -> (gtk::Box, gtk::Label, gtk::Button, gtk::Button) {
    let label = gtk::Label::new(Some(title));
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
    tab.append(&select_button);
    tab.append(&close_button);

    (tab, label, select_button, close_button)
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
    let title: String = title
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    let title = title.trim();
    if title.is_empty() {
        fallback.to_owned()
    } else {
        title.to_owned()
    }
}

fn update_window_title(window: &gtk::ApplicationWindow, terminal: &vte4::Terminal, fallback: &str) {
    set_window_title(window, &terminal_display_title(terminal, fallback));
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

fn create_content(terminal: &vte4::Terminal, config: &AppConfig) -> gtk::Overlay {
    let overlay = gtk::Overlay::new();
    overlay.add_css_class("zter-content");

    let background = create_background(config);
    overlay.set_child(Some(&background));
    overlay.add_overlay(terminal);
    overlay.set_measure_overlay(terminal, true);

    overlay
}

fn create_background(config: &AppConfig) -> gtk::DrawingArea {
    let background = gtk::DrawingArea::new();
    background.set_can_target(false);
    background.set_hexpand(true);
    background.set_vexpand(true);

    let color = theme::background_color(config.theme());
    let wallpaper = config.wallpaper().and_then(|path| {
        gtk::gdk_pixbuf::Pixbuf::from_file(path)
            .map_err(|error| {
                eprintln!(
                    "zter: could not load wallpaper {}: {error}; using the theme background",
                    path.display()
                );
            })
            .ok()
    });
    let wallpaper_opacity = config.wallpaper_opacity();

    background.set_draw_func(move |_, context, width, height| {
        let width = f64::from(width);
        let height = f64::from(height);
        clip_rounded_bottom(context, width, height, WINDOW_CORNER_RADIUS);

        context.set_source_color(&color);
        let _ = context.paint();

        let Some(wallpaper) = wallpaper.as_ref() else {
            return;
        };

        let placement = cover_placement(
            width,
            height,
            f64::from(wallpaper.width()),
            f64::from(wallpaper.height()),
        );
        context.set_operator(WALLPAPER_BLEND_OPERATOR);
        context.translate(placement.x, placement.y);
        context.scale(placement.scale, placement.scale);
        context.set_source_pixbuf(wallpaper, 0.0, 0.0);
        let _ = context.paint_with_alpha(wallpaper_opacity);
    });

    background
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CoverPlacement {
    x: f64,
    y: f64,
    scale: f64,
}

fn cover_placement(
    area_width: f64,
    area_height: f64,
    image_width: f64,
    image_height: f64,
) -> CoverPlacement {
    let scale = (area_width / image_width).max(area_height / image_height);
    CoverPlacement {
        x: (area_width - image_width * scale) / 2.0,
        y: (area_height - image_height * scale) / 2.0,
        scale,
    }
}

fn clip_rounded_bottom(context: &gtk::cairo::Context, width: f64, height: f64, radius: f64) {
    let radius = radius.min(width / 2.0).min(height);
    context.new_path();
    context.move_to(0.0, 0.0);
    context.line_to(width, 0.0);
    context.line_to(width, height - radius);
    context.arc(width - radius, height - radius, radius, 0.0, FRAC_PI_2);
    context.line_to(radius, height);
    context.arc(radius, height - radius, radius, FRAC_PI_2, PI);
    context.close_path();
    context.clip();
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
    fn cover_placement_centers_and_crops_a_wide_image() {
        let placement = cover_placement(960.0, 600.0, 1600.0, 900.0);

        assert!((placement.scale - (2.0 / 3.0)).abs() < f64::EPSILON);
        assert!((placement.x + 53.333_333_333_333_37).abs() < 1e-10);
        assert_eq!(placement.y, 0.0);
    }

    #[test]
    fn cover_placement_centers_and_crops_a_tall_image() {
        let placement = cover_placement(960.0, 600.0, 900.0, 1600.0);

        assert!((placement.scale - (16.0 / 15.0)).abs() < f64::EPSILON);
        assert_eq!(placement.x, 0.0);
        assert!((placement.y + 553.333_333_333_333_4).abs() < 1e-10);
    }

    #[test]
    fn wallpaper_uses_screen_blending() {
        assert_eq!(WALLPAPER_BLEND_OPERATOR, gtk::cairo::Operator::Screen);
    }
}
