use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::io::Cursor;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gtk::gdk::prelude::*;
use gtk::prelude::*;
use vte4::prelude::*;

use crate::{
    config::{AppConfig, WallpaperSource},
    identity::{APPLICATION_NAME, ICON_NAME, SETTINGS_RELOAD_ACTION},
    settings::{MAX_FONT_SIZE, MIN_FONT_SIZE, TerminalPadding},
    theme,
};

const DEFAULT_WIDTH: i32 = 960;
const DEFAULT_HEIGHT: i32 = 600;
const WALLPAPER_BLEND_OPERATOR: gtk::cairo::Operator = gtk::cairo::Operator::Screen;
const BUNDLED_WALLPAPER: &[u8] = include_bytes!("../data/wallpapers/zter-wallpaper.png");
const TAB_ID_PREFIX: &str = "zter-tab-";
const TAB_DROP_TARGET_CLASS: &str = "zter-tab-drop-target";
const TAB_WIDTH: f64 = 220.0;
const TAB_SCROLL_STEP: f64 = 48.0;
const TERMINAL_RESIZE_SETTLE: Duration = Duration::from_millis(120);
const TERMINAL_TOP_BORDER: i32 = 1;
const TERMINAL_SCROLLBAR_HIDDEN_CLASS: &str = "zter-terminal-scrollbar-hidden";
const TERMINAL_ZOOM_STEP: f64 = 1.0;
// Places the supported point range inside VTE's native 0.25-4.0 scale.
const TERMINAL_FONT_SCALE_BASE_SIZE: f64 = 20.0;
const TERMINAL_INTERRUPT: &[u8] = b"\x03";
const TERMINAL_END_OF_INPUT: &[u8] = b"\x04";
const TERMINAL_SUSPEND: &[u8] = b"\x1a";
const CODEX_ACTION_REQUIRED_STATUS: &str = "[ ! ] Action Required";
const TERMINAL_STATUS_GLYPHS: [char; 16] = [
    '⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏', '◐', '◑', '✦', '✋', '✳', '◇',
];

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TabShortcut {
    New,
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClipboardShortcut {
    Copy,
    Paste,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ClipboardShortcutKeycodes {
    copy: Vec<u32>,
    paste: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClipboardPasteRoute {
    PasteText,
    PassThrough,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClipboardCopyRoute {
    CopySelection,
    ConfirmInterrupt,
    PassThrough,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ForegroundProcessShortcut {
    ConfirmEndOfInput,
    ConfirmSuspend,
    Suppress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalZoom {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalTitleStatus {
    Glyph(char),
    ActionRequired,
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

    fn displayed(&self) -> String {
        let Some(manual) = &self.manual else {
            return self.automatic.clone();
        };
        match recognized_terminal_status(&self.automatic) {
            Some((TerminalTitleStatus::Glyph(glyph), _)) => format!("{glyph} {manual}"),
            Some((TerminalTitleStatus::ActionRequired, _)) => {
                format!("{CODEX_ACTION_REQUIRED_STATUS} | {manual}")
            }
            None => manual.clone(),
        }
    }

    fn editable(&self) -> &str {
        if let Some(manual) = &self.manual {
            return manual;
        }
        recognized_terminal_status(&self.automatic)
            .map(|(_, title)| title)
            .unwrap_or(&self.automatic)
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
struct CloseProtection {
    shell_pids: Rc<RefCell<HashMap<String, Option<libc::pid_t>>>>,
    prompt_open: Rc<Cell<bool>>,
    window_close_confirmed: Rc<Cell<bool>>,
}

#[derive(Clone)]
struct WallpaperAsset {
    inner: Rc<WallpaperAssetInner>,
}

#[derive(Default)]
struct WallpaperAssetInner {
    texture: RefCell<Option<gtk::gdk::Texture>>,
    backgrounds: RefCell<Vec<gtk::glib::WeakRef<gtk::Picture>>>,
    reload_generation: Cell<u64>,
}

impl WallpaperAsset {
    fn new(texture: Option<gtk::gdk::Texture>) -> Self {
        Self {
            inner: Rc::new(WallpaperAssetInner {
                texture: RefCell::new(texture),
                ..WallpaperAssetInner::default()
            }),
        }
    }

    fn create_background(&self) -> gtk::Picture {
        let background = gtk::Picture::new();
        background.add_css_class("zter-background");
        background.set_can_target(false);
        background.set_can_shrink(true);
        background.set_content_fit(gtk::ContentFit::Cover);
        background.set_hexpand(true);
        background.set_vexpand(true);
        background.set_paintable(self.inner.texture.borrow().as_ref());
        self.inner
            .backgrounds
            .borrow_mut()
            .push(background.downgrade());
        background
    }

    fn replace(&self, texture: Option<gtk::gdk::Texture>) {
        *self.inner.texture.borrow_mut() = texture.clone();
        self.inner.backgrounds.borrow_mut().retain(|background| {
            let Some(background) = background.upgrade() else {
                return false;
            };
            background.set_paintable(texture.as_ref());
            true
        });
    }

    fn begin_reload(&self) -> u64 {
        let generation = self.inner.reload_generation.get().wrapping_add(1);
        self.inner.reload_generation.set(generation);
        generation
    }

    fn is_current_reload(&self, generation: u64) -> bool {
        self.inner.reload_generation.get() == generation
    }
}

impl Default for WallpaperAsset {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Clone)]
struct WallpaperPreparation {
    source: Option<WallpaperSource>,
    display_size: (i32, i32),
    background: [f64; 4],
    opacity: f64,
}

struct PreparedWallpaper {
    width: i32,
    height: i32,
    stride: usize,
    pixels: Vec<u8>,
}

#[derive(Debug)]
enum WallpaperPreparationError {
    Load(String),
    Cairo(gtk::cairo::Error),
    Downscale,
    PixelAccess(String),
}

impl fmt::Display for WallpaperPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => write!(formatter, "could not load the wallpaper: {error}"),
            Self::Cairo(error) => write!(formatter, "could not blend the wallpaper: {error}"),
            Self::Downscale => formatter.write_str("could not downscale the wallpaper"),
            Self::PixelAccess(error) => {
                write!(
                    formatter,
                    "could not read the prepared wallpaper pixels: {error}"
                )
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
    ApplyInitial((i32, i32)),
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
            TerminalResizeAction::ApplyInitial(size)
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

struct TerminalZoomState {
    font_size: f64,
}

impl TerminalZoomState {
    fn new(font_size: f64) -> Self {
        Self { font_size }
    }

    fn request(&mut self, zoom: TerminalZoom) -> Option<f64> {
        let next = zoomed_font_size(self.font_size, zoom);
        if next == self.font_size {
            return None;
        }

        self.font_size = next;
        Some(next)
    }
}

#[derive(Clone)]
struct TerminalZoomControl {
    terminal: gtk::glib::WeakRef<vte4::Terminal>,
    state: Rc<RefCell<TerminalZoomState>>,
}

impl TerminalZoomControl {
    fn new(terminal: &vte4::Terminal, initial_font_size: f64) -> Self {
        Self {
            terminal: terminal.downgrade(),
            state: Rc::new(RefCell::new(TerminalZoomState::new(initial_font_size))),
        }
    }

    fn request(&self, zoom: TerminalZoom) {
        let Some(font_size) = self.state.borrow_mut().request(zoom) else {
            return;
        };
        let Some(terminal) = self.terminal.upgrade() else {
            return;
        };

        terminal.set_font_scale(terminal_font_scale(font_size));
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
    install_settings_reload_action(
        application,
        &gtk::prelude::WidgetExt::display(&window),
        &wallpaper,
    );

    let notebook = create_notebook();
    let close_protection = CloseProtection::default();
    let (header, tab_strip, tab_scroller) =
        create_header(&window, &notebook, config, &wallpaper, &close_protection);
    install_tab_shortcuts(
        &window,
        &notebook,
        &tab_strip,
        &tab_scroller,
        config,
        &wallpaper,
        &close_protection,
    );
    install_tab_switch_handler(&window, &notebook, &tab_strip, &tab_scroller, config);
    install_window_close_protection(&window, &notebook, &close_protection);

    window.set_titlebar(Some(&header));
    window.set_child(Some(&notebook));
    add_terminal_tab(
        &window,
        &notebook,
        &tab_strip,
        &tab_scroller,
        config,
        &wallpaper,
        &close_protection,
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
    close_protection: &CloseProtection,
) -> (gtk::Box, gtk::Box, gtk::ScrolledWindow) {
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
        close_protection,
    );
    let pinned_new_tab = create_new_tab_button(
        window,
        notebook,
        &tab_strip,
        &tab_scroller,
        config,
        wallpaper,
        close_protection,
    );
    pinned_new_tab.set_visible(false);

    let drag_space = gtk::WindowHandle::new();
    drag_space.add_css_class("zter-drag-space");
    drag_space.set_hexpand(true);
    let overflow_drag_space = gtk::WindowHandle::new();
    overflow_drag_space.add_css_class("zter-drag-space");
    overflow_drag_space.set_visible(false);
    scroll_content.append(&inline_new_tab);
    scroll_content.append(&drag_space);
    install_tab_overflow(
        &tab_scroller,
        &inline_new_tab,
        &pinned_new_tab,
        &overflow_drag_space,
    );

    let window_controls = gtk::WindowControls::new(gtk::PackType::End);
    window_controls.set_valign(gtk::Align::Center);

    header.append(&tab_scroller);
    header.append(&pinned_new_tab);
    header.append(&overflow_drag_space);
    header.append(&window_controls);

    (header, tab_strip, tab_scroller)
}

fn create_new_tab_button(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
    close_protection: &CloseProtection,
) -> gtk::Button {
    let button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .has_frame(false)
        .tooltip_text("New tab")
        .build();
    button.add_css_class("zter-new-tab");
    button.set_valign(gtk::Align::Center);

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let config = config.clone();
    let wallpaper = wallpaper.clone();
    let close_protection = close_protection.clone();
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
            &close_protection,
        );
    });

    button
}

fn install_tab_overflow(
    scroller: &gtk::ScrolledWindow,
    inline_button: &gtk::Button,
    pinned_button: &gtk::Button,
    drag_space: &gtk::WindowHandle,
) {
    let inline_weak = inline_button.downgrade();
    let pinned_weak = pinned_button.downgrade();
    let drag_space_weak = drag_space.downgrade();
    scroller.hadjustment().connect_changed(move |adjustment| {
        let (Some(inline_button), Some(pinned_button), Some(drag_space)) = (
            inline_weak.upgrade(),
            pinned_weak.upgrade(),
            drag_space_weak.upgrade(),
        ) else {
            return;
        };
        let overflow = adjustment.upper() > adjustment.page_size() + 0.5;
        inline_button.set_visible(!overflow);
        pinned_button.set_visible(overflow);
        drag_space.set_visible(overflow);
    });

    let adjustment = scroller.hadjustment();
    let overflow = adjustment.upper() > adjustment.page_size() + 0.5;
    inline_button.set_visible(!overflow);
    pinned_button.set_visible(overflow);
    drag_space.set_visible(overflow);
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
    close_protection: &CloseProtection,
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let config = config.clone();
    let wallpaper = wallpaper.clone();
    let close_protection = close_protection.clone();
    controller.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(shortcut) = tab_shortcut(key, modifiers) else {
            return gtk::glib::Propagation::Proceed;
        };
        let Some(notebook) = notebook_weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };

        match shortcut {
            TabShortcut::New => {
                let (Some(window), Some(tab_strip), Some(tab_scroller)) = (
                    window_weak.upgrade(),
                    tab_strip_weak.upgrade(),
                    tab_scroller_weak.upgrade(),
                ) else {
                    return gtk::glib::Propagation::Proceed;
                };
                add_terminal_tab(
                    &window,
                    &notebook,
                    &tab_strip,
                    &tab_scroller,
                    &config,
                    &wallpaper,
                    &close_protection,
                );
            }
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
    close_protection: &CloseProtection,
) {
    let working_directory = new_tab_working_directory(notebook, config, close_protection);
    let fallback_title = default_tab_title(config.shell());
    let tab_id = next_tab_id();
    close_protection
        .shell_pids
        .borrow_mut()
        .insert(tab_id.clone(), None);
    let terminal = create_terminal(config, window, &tab_id, close_protection);
    install_foreground_process_key_protection(&terminal, window, &tab_id, close_protection);
    let terminal_for_spawn = terminal.clone();
    let config_for_spawn = config.clone();
    let tab_id_for_spawn = tab_id.clone();
    let close_protection_for_spawn = close_protection.clone();
    let content = create_content(&terminal, config, wallpaper, move || {
        spawn_shell(
            &terminal_for_spawn,
            &config_for_spawn,
            &working_directory,
            &tab_id_for_spawn,
            &close_protection_for_spawn,
        );
    });
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
    let close_protection_for_button = close_protection.clone();
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
        request_close_tab(
            &window,
            &notebook,
            &tab_strip,
            &tab_scroller,
            &content,
            &close_protection_for_button,
        );
    });

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let content_weak = content.downgrade();
    let close_protection_for_exit = close_protection.clone();
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
        close_tab(
            &window,
            &notebook,
            &tab_strip,
            &tab_scroller,
            &content,
            &close_protection_for_exit,
        );
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
            state.displayed()
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
}

fn new_tab_working_directory(
    notebook: &gtk::Notebook,
    config: &AppConfig,
    close_protection: &CloseProtection,
) -> String {
    let active_page = notebook
        .current_page()
        .and_then(|page_number| notebook.nth_page(Some(page_number)));
    let active_directory = active_page.and_then(|page| {
        let terminal_directory = find_terminal(&page)
            .and_then(|terminal| terminal.current_directory_uri())
            .and_then(|uri| local_path_from_uri(&uri));
        terminal_directory.or_else(|| shell_working_directory(&page, close_protection))
    });

    working_directory_or_fallback(active_directory, config.working_directory())
}

fn local_path_from_uri(uri: &str) -> Option<PathBuf> {
    gtk::gio::File::for_uri(uri).path()
}

fn shell_working_directory(
    content: &impl IsA<gtk::Widget>,
    close_protection: &CloseProtection,
) -> Option<PathBuf> {
    let shell_pid = close_protection
        .shell_pids
        .borrow()
        .get(content.widget_name().as_str())
        .copied()
        .flatten()?;

    process_working_directory(shell_pid)
}

fn process_working_directory(pid: libc::pid_t) -> Option<PathBuf> {
    fs::read_link(Path::new("/proc").join(pid.to_string()).join("cwd")).ok()
}

fn working_directory_or_fallback(directory: Option<PathBuf>, fallback: &str) -> String {
    directory
        .filter(|directory| directory.is_dir())
        .and_then(|directory| directory.into_os_string().into_string().ok())
        .unwrap_or_else(|| fallback.to_owned())
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
        .tooltip_text("Close tab")
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
        // Keep the title edit gesture separate from tab activation and drag-and-drop.
        gesture.set_state(gtk::EventSequenceState::Claimed);
        entry.set_text(state.borrow().editable());
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
            state.displayed()
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
        let state = state.borrow();
        let title = state.displayed();
        entry.set_text(state.editable());
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
    drop_target.set_preload(true);
    let hovering = Rc::new(Cell::new(false));

    let tab_weak = tab.downgrade();
    let target_id = tab_id.to_owned();
    let hovering_on_enter = hovering.clone();
    drop_target.connect_enter(move |drop_target, _, _| {
        hovering_on_enter.set(true);
        if let Some(tab) = tab_weak.upgrade() {
            sync_tab_drop_highlight(drop_target, &tab, &target_id, true);
        }
        gtk::gdk::DragAction::MOVE
    });

    let tab_weak = tab.downgrade();
    let target_id = tab_id.to_owned();
    let hovering_on_value = hovering.clone();
    drop_target.connect_value_notify(move |drop_target| {
        if let Some(tab) = tab_weak.upgrade() {
            sync_tab_drop_highlight(drop_target, &tab, &target_id, hovering_on_value.get());
        }
    });

    let tab_weak = tab.downgrade();
    let hovering_on_leave = hovering.clone();
    drop_target.connect_leave(move |_| {
        hovering_on_leave.set(false);
        if let Some(tab) = tab_weak.upgrade() {
            tab.remove_css_class(TAB_DROP_TARGET_CLASS);
        }
    });

    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let tab_weak = tab.downgrade();
    let hovering_on_drop = hovering;
    let target_id = tab_id.to_owned();
    drop_target.connect_drop(move |_, value, _, _| {
        hovering_on_drop.set(false);
        if let Some(tab) = tab_weak.upgrade() {
            tab.remove_css_class(TAB_DROP_TARGET_CLASS);
        }
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

fn sync_tab_drop_highlight(
    drop_target: &gtk::DropTarget,
    tab: &gtk::Box,
    target_id: &str,
    hovering: bool,
) {
    let source_id = drop_target
        .value()
        .and_then(|value| value.get::<String>().ok());
    if should_highlight_tab_drop_target(source_id.as_deref(), target_id, hovering) {
        tab.add_css_class(TAB_DROP_TARGET_CLASS);
    } else {
        tab.remove_css_class(TAB_DROP_TARGET_CLASS);
    }
}

fn should_highlight_tab_drop_target(
    source_id: Option<&str>,
    target_id: &str,
    hovering: bool,
) -> bool {
    hovering
        && source_id
            .is_some_and(|source_id| source_id.starts_with(TAB_ID_PREFIX) && source_id != target_id)
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

fn install_window_close_protection(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    close_protection: &CloseProtection,
) {
    let notebook_weak = notebook.downgrade();
    let close_protection = close_protection.clone();
    window.connect_close_request(move |window| {
        if close_protection.window_close_confirmed.replace(false) {
            return gtk::glib::Propagation::Proceed;
        }

        let Some(notebook) = notebook_weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };
        if !notebook_has_running_foreground_process(&notebook, &close_protection) {
            return gtk::glib::Propagation::Proceed;
        }
        if close_protection.prompt_open.get() {
            return gtk::glib::Propagation::Stop;
        }

        let window_weak = window.downgrade();
        let close_protection_for_confirm = close_protection.clone();
        show_close_confirmation(
            window,
            &close_protection,
            "Processes are still running. Close zter?",
            move || {
                let Some(window) = window_weak.upgrade() else {
                    return;
                };
                close_protection_for_confirm
                    .window_close_confirmed
                    .set(true);
                window.close();
            },
        );
        gtk::glib::Propagation::Stop
    });
}

fn request_close_tab(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    content: &impl IsA<gtk::Widget>,
    close_protection: &CloseProtection,
) {
    if !tab_has_running_foreground_process(content, close_protection) {
        close_tab(
            window,
            notebook,
            tab_strip,
            tab_scroller,
            content,
            close_protection,
        );
        return;
    }
    if close_protection.prompt_open.get() {
        return;
    }

    let window_weak = window.downgrade();
    let notebook_weak = notebook.downgrade();
    let tab_strip_weak = tab_strip.downgrade();
    let tab_scroller_weak = tab_scroller.downgrade();
    let content_weak = content.as_ref().downgrade();
    let close_protection_for_confirm = close_protection.clone();
    show_close_confirmation(
        window,
        close_protection,
        "A process is still running. Close this tab?",
        move || {
            let (Some(window), Some(notebook), Some(tab_strip), Some(tab_scroller), Some(content)) = (
                window_weak.upgrade(),
                notebook_weak.upgrade(),
                tab_strip_weak.upgrade(),
                tab_scroller_weak.upgrade(),
                content_weak.upgrade(),
            ) else {
                return;
            };
            close_tab(
                &window,
                &notebook,
                &tab_strip,
                &tab_scroller,
                &content,
                &close_protection_for_confirm,
            );
        },
    );
}

fn show_close_confirmation<F>(
    window: &gtk::ApplicationWindow,
    close_protection: &CloseProtection,
    message: &str,
    on_confirm: F,
) where
    F: FnOnce() + 'static,
{
    if close_protection.prompt_open.replace(true) {
        return;
    }

    let dialog = gtk::Window::builder()
        .transient_for(window)
        .modal(true)
        .decorated(false)
        .resizable(false)
        .destroy_with_parent(true)
        .build();
    dialog.add_css_class("zter-close-dialog");

    let surface = gtk::Box::new(gtk::Orientation::Vertical, 0);
    surface.add_css_class("zter-close-dialog-surface");
    surface.set_overflow(gtk::Overflow::Hidden);

    let message_label = gtk::Label::builder()
        .label(message)
        .justify(gtk::Justification::Center)
        .wrap(true)
        .build();
    message_label.add_css_class("zter-close-dialog-message");
    surface.append(&message_label);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions.add_css_class("zter-close-dialog-actions");
    actions.set_homogeneous(true);

    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .has_frame(false)
        .receives_default(true)
        .build();
    cancel_button.add_css_class("zter-close-dialog-cancel");
    let close_button = gtk::Button::builder()
        .label("Close")
        .has_frame(false)
        .build();
    close_button.add_css_class("zter-close-dialog-confirm");
    actions.append(&cancel_button);
    actions.append(&close_button);
    surface.append(&actions);

    dialog.set_child(Some(&surface));
    dialog.set_default_widget(Some(&cancel_button));

    let prompt_open = close_protection.prompt_open.clone();
    dialog.connect_destroy(move |_| prompt_open.set(false));

    let dialog_weak = dialog.downgrade();
    cancel_button.connect_clicked(move |_| {
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
    });

    let on_confirm = Rc::new(RefCell::new(Some(on_confirm)));
    let dialog_weak = dialog.downgrade();
    let prompt_open = close_protection.prompt_open.clone();
    close_button.connect_clicked(move |_| {
        prompt_open.set(false);
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
        if let Some(on_confirm) = on_confirm.borrow_mut().take() {
            on_confirm();
        }
    });

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let dialog_weak = dialog.downgrade();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key != gtk::gdk::Key::Escape {
            return gtk::glib::Propagation::Proceed;
        }
        if let Some(dialog) = dialog_weak.upgrade() {
            dialog.close();
        }
        gtk::glib::Propagation::Stop
    });
    dialog.add_controller(key_controller);

    dialog.present();
    cancel_button.grab_focus();
}

fn notebook_has_running_foreground_process(
    notebook: &gtk::Notebook,
    close_protection: &CloseProtection,
) -> bool {
    (0..notebook.n_pages()).any(|page_number| {
        notebook
            .nth_page(Some(page_number))
            .is_some_and(|page| tab_has_running_foreground_process(&page, close_protection))
    })
}

fn tab_has_running_foreground_process(
    content: &impl IsA<gtk::Widget>,
    close_protection: &CloseProtection,
) -> bool {
    let shell_pid = close_protection
        .shell_pids
        .borrow()
        .get(content.widget_name().as_str())
        .copied()
        .flatten();
    let Some((terminal, shell_pid)) = find_terminal(content.as_ref()).zip(shell_pid) else {
        return false;
    };

    terminal_has_running_foreground_process(&terminal, shell_pid)
}

fn terminal_has_running_foreground_process(
    terminal: &vte4::Terminal,
    shell_pid: libc::pid_t,
) -> bool {
    let Some(pty) = terminal.pty() else {
        return false;
    };

    // SAFETY: VTE owns the PTY and keeps its borrowed file descriptor valid for
    // the duration of this call. Both functions report failure with `-1`.
    let foreground_process_group = unsafe { libc::tcgetpgrp(pty.fd().as_raw_fd()) };
    // SAFETY: `shell_pid` came from VTE's successful spawn result. A process
    // that has already exited is handled by `getpgid` returning `-1`.
    let shell_process_group = unsafe { libc::getpgid(shell_pid) };
    process_groups_have_running_foreground_process(shell_process_group, foreground_process_group)
}

fn process_groups_have_running_foreground_process(
    shell_process_group: libc::pid_t,
    foreground_process_group: libc::pid_t,
) -> bool {
    shell_process_group > 0
        && foreground_process_group > 0
        && shell_process_group != foreground_process_group
}

fn close_tab(
    window: &gtk::ApplicationWindow,
    notebook: &gtk::Notebook,
    tab_strip: &gtk::Box,
    tab_scroller: &gtk::ScrolledWindow,
    content: &impl IsA<gtk::Widget>,
    close_protection: &CloseProtection,
) {
    let Some(page_number) = notebook.page_num(content) else {
        return;
    };
    if let Some(tab) = tab_by_id(tab_strip, &content.widget_name()) {
        tab_strip.remove(&tab);
    }
    close_protection
        .shell_pids
        .borrow_mut()
        .remove(content.widget_name().as_str());
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
    if !control || shift {
        return None;
    }

    let system_modifiers = gtk::gdk::ModifierType::ALT_MASK
        | gtk::gdk::ModifierType::SUPER_MASK
        | gtk::gdk::ModifierType::HYPER_MASK
        | gtk::gdk::ModifierType::META_MASK;
    match key.to_lower() {
        gtk::gdk::Key::t if !modifiers.intersects(system_modifiers) => Some(TabShortcut::New),
        gtk::gdk::Key::Page_Up => Some(TabShortcut::Previous),
        gtk::gdk::Key::Page_Down => Some(TabShortcut::Next),
        _ => None,
    }
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

fn recognized_terminal_status(title: &str) -> Option<(TerminalTitleStatus, &str)> {
    if let Some(remainder) = title.strip_prefix(CODEX_ACTION_REQUIRED_STATUS) {
        if remainder.is_empty() {
            return Some((TerminalTitleStatus::ActionRequired, remainder));
        }
        let title = remainder.trim_start().strip_prefix('|')?.trim_start();
        return Some((TerminalTitleStatus::ActionRequired, title));
    }

    let glyph = title.chars().next()?;
    if !TERMINAL_STATUS_GLYPHS.contains(&glyph) {
        return None;
    }
    let remainder = title.strip_prefix(glyph)?;
    if remainder.is_empty() {
        return Some((TerminalTitleStatus::Glyph(glyph), remainder));
    }
    if !remainder.chars().next()?.is_whitespace() {
        return None;
    }
    Some((TerminalTitleStatus::Glyph(glyph), remainder.trim_start()))
}

fn set_window_title(window: &gtk::ApplicationWindow, title: &str) {
    window.set_title(Some(&format!("{title} — {APPLICATION_NAME}")));
}

fn create_terminal(
    config: &AppConfig,
    window: &gtk::ApplicationWindow,
    tab_id: &str,
    close_protection: &CloseProtection,
) -> vte4::Terminal {
    let terminal = vte4::Terminal::new();
    terminal.add_css_class("zter-terminal");
    terminal.set_hexpand(true);
    terminal.set_vexpand(true);
    terminal.set_scrollback_lines(config.scrollback_lines());
    terminal.set_scroll_on_keystroke(true);
    terminal.set_mouse_autohide(true);
    terminal.set_allow_hyperlink(true);
    terminal.set_font(Some(&terminal_font(
        config.font_family(),
        TERMINAL_FONT_SCALE_BASE_SIZE,
    )));
    terminal.set_font_scale(terminal_font_scale(config.font_size()));
    install_hyperlink_activation(&terminal, window);
    install_clipboard_shortcuts(&terminal, window, tab_id, close_protection);
    install_clipboard_context_menu(&terminal);
    theme::apply_to(&terminal, config.theme());

    terminal
}

fn install_foreground_process_key_protection(
    terminal: &vte4::Terminal,
    window: &gtk::ApplicationWindow,
    tab_id: &str,
    close_protection: &CloseProtection,
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    let window_weak = window.downgrade();
    let terminal_weak = terminal.downgrade();
    let tab_id = tab_id.to_owned();
    let close_protection = close_protection.clone();
    controller.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(shortcut) = foreground_process_shortcut(key, modifiers) else {
            return gtk::glib::Propagation::Proceed;
        };
        let Some(terminal) = terminal_weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };
        if !tab_terminal_has_running_foreground_process(
            &terminal,
            tab_id.as_str(),
            &close_protection,
        ) {
            return gtk::glib::Propagation::Proceed;
        }

        let control_sequence = match shortcut {
            ForegroundProcessShortcut::ConfirmEndOfInput => TERMINAL_END_OF_INPUT,
            ForegroundProcessShortcut::ConfirmSuspend => TERMINAL_SUSPEND,
            ForegroundProcessShortcut::Suppress => return gtk::glib::Propagation::Stop,
        };
        let Some(window) = window_weak.upgrade() else {
            return gtk::glib::Propagation::Stop;
        };
        let terminal_weak = terminal.downgrade();
        let tab_id = tab_id.clone();
        let close_protection_for_confirm = close_protection.clone();
        show_close_confirmation(
            &window,
            &close_protection,
            "A process is still running. Close?",
            move || {
                let Some(terminal) = terminal_weak.upgrade() else {
                    return;
                };
                if tab_terminal_has_running_foreground_process(
                    &terminal,
                    tab_id.as_str(),
                    &close_protection_for_confirm,
                ) {
                    terminal.feed_child(control_sequence);
                }
            },
        );

        gtk::glib::Propagation::Stop
    });

    terminal.add_controller(controller);
}

fn foreground_process_shortcut(
    key: gtk::gdk::Key,
    modifiers: gtk::gdk::ModifierType,
) -> Option<ForegroundProcessShortcut> {
    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let shift = modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK);
    let system_modifiers = gtk::gdk::ModifierType::ALT_MASK
        | gtk::gdk::ModifierType::SUPER_MASK
        | gtk::gdk::ModifierType::HYPER_MASK
        | gtk::gdk::ModifierType::META_MASK;

    if !control || modifiers.intersects(system_modifiers) {
        return None;
    }

    match key.to_lower() {
        gtk::gdk::Key::d if !shift => Some(ForegroundProcessShortcut::ConfirmEndOfInput),
        gtk::gdk::Key::z if shift => Some(ForegroundProcessShortcut::Suppress),
        gtk::gdk::Key::z => Some(ForegroundProcessShortcut::ConfirmSuspend),
        _ => None,
    }
}

fn terminal_font(family: &str, size: f64) -> gtk::pango::FontDescription {
    let mut font = gtk::pango::FontDescription::new();
    font.set_family(family);
    font.set_size((size * f64::from(gtk::pango::SCALE)).round() as i32);
    font
}

fn terminal_font_scale(font_size: f64) -> f64 {
    font_size / TERMINAL_FONT_SCALE_BASE_SIZE
}

fn install_clipboard_shortcuts(
    terminal: &vte4::Terminal,
    window: &gtk::ApplicationWindow,
    tab_id: &str,
    close_protection: &CloseProtection,
) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let keycodes = ClipboardShortcutKeycodes::from_display(&terminal.display());

    let window_weak = window.downgrade();
    let terminal_weak = terminal.downgrade();
    let tab_id = tab_id.to_owned();
    let close_protection = close_protection.clone();
    controller.connect_key_pressed(move |_, key, keycode, modifiers| {
        let Some(terminal) = terminal_weak.upgrade() else {
            return gtk::glib::Propagation::Proceed;
        };

        match clipboard_shortcut(key, keycode, modifiers, &keycodes) {
            Some(ClipboardShortcut::Copy) => match clipboard_copy_route(
                terminal.has_selection(),
                tab_terminal_has_running_foreground_process(
                    &terminal,
                    tab_id.as_str(),
                    &close_protection,
                ),
            ) {
                ClipboardCopyRoute::CopySelection => {
                    terminal.copy_clipboard_format(vte4::Format::Text)
                }
                ClipboardCopyRoute::ConfirmInterrupt => {
                    let Some(window) = window_weak.upgrade() else {
                        return gtk::glib::Propagation::Stop;
                    };
                    let terminal_weak = terminal.downgrade();
                    let tab_id = tab_id.clone();
                    let close_protection_for_confirm = close_protection.clone();
                    show_close_confirmation(
                        &window,
                        &close_protection,
                        "A process is still running. Close?",
                        move || {
                            let Some(terminal) = terminal_weak.upgrade() else {
                                return;
                            };
                            if tab_terminal_has_running_foreground_process(
                                &terminal,
                                tab_id.as_str(),
                                &close_protection_for_confirm,
                            ) {
                                terminal.feed_child(TERMINAL_INTERRUPT);
                            }
                        },
                    );
                }
                ClipboardCopyRoute::PassThrough => {
                    return gtk::glib::Propagation::Proceed;
                }
            },
            None => return gtk::glib::Propagation::Proceed,
            Some(ClipboardShortcut::Paste) => {
                let clipboard = terminal.clipboard();
                match clipboard_paste_route(
                    clipboard.formats().contains_type(gtk::glib::Type::STRING),
                ) {
                    ClipboardPasteRoute::PasteText => terminal.paste_clipboard(),
                    ClipboardPasteRoute::PassThrough => {
                        return gtk::glib::Propagation::Proceed;
                    }
                }
            }
        }

        gtk::glib::Propagation::Stop
    });

    terminal.add_controller(controller);
}

fn tab_terminal_has_running_foreground_process(
    terminal: &vte4::Terminal,
    tab_id: &str,
    close_protection: &CloseProtection,
) -> bool {
    let shell_pid = close_protection
        .shell_pids
        .borrow()
        .get(tab_id)
        .copied()
        .flatten();
    let Some(shell_pid) = shell_pid else {
        return false;
    };

    terminal_has_running_foreground_process(terminal, shell_pid)
}

impl ClipboardShortcutKeycodes {
    fn from_display(display: &gtk::gdk::Display) -> Self {
        Self {
            copy: keycodes_for_keyval(display, gtk::gdk::Key::c),
            paste: keycodes_for_keyval(display, gtk::gdk::Key::v),
        }
    }

    fn shortcut_for_keycode(&self, keycode: u32) -> Option<ClipboardShortcut> {
        if self.copy.contains(&keycode) {
            Some(ClipboardShortcut::Copy)
        } else if self.paste.contains(&keycode) {
            Some(ClipboardShortcut::Paste)
        } else {
            None
        }
    }
}

fn keycodes_for_keyval(display: &gtk::gdk::Display, key: gtk::gdk::Key) -> Vec<u32> {
    let Some(keys) = display.map_keyval(key) else {
        return Vec::new();
    };
    let mut keycodes: Vec<u32> = keys.into_iter().map(|key| key.keycode()).collect();
    keycodes.sort_unstable();
    keycodes.dedup();
    keycodes
}

fn clipboard_shortcut(
    key: gtk::gdk::Key,
    keycode: u32,
    modifiers: gtk::gdk::ModifierType,
    keycodes: &ClipboardShortcutKeycodes,
) -> Option<ClipboardShortcut> {
    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let shift = modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK);
    if !control || shift {
        return None;
    }

    match key.to_lower() {
        gtk::gdk::Key::c => Some(ClipboardShortcut::Copy),
        gtk::gdk::Key::v => Some(ClipboardShortcut::Paste),
        _ => keycodes.shortcut_for_keycode(keycode),
    }
}

fn clipboard_copy_route(has_selection: bool, has_foreground_process: bool) -> ClipboardCopyRoute {
    if has_selection {
        ClipboardCopyRoute::CopySelection
    } else if has_foreground_process {
        ClipboardCopyRoute::ConfirmInterrupt
    } else {
        ClipboardCopyRoute::PassThrough
    }
}

fn clipboard_paste_route(clipboard_contains_text: bool) -> ClipboardPasteRoute {
    if clipboard_contains_text {
        ClipboardPasteRoute::PasteText
    } else {
        ClipboardPasteRoute::PassThrough
    }
}

fn install_hyperlink_activation(terminal: &vte4::Terminal, window: &gtk::ApplicationWindow) {
    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_PRIMARY);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);

    let terminal_weak = terminal.downgrade();
    let window_weak = window.downgrade();
    click.connect_pressed(move |gesture, _, _, _| {
        if !is_control_hyperlink_click(gesture.current_event_state()) {
            return;
        }
        let Some(terminal) = terminal_weak.upgrade() else {
            return;
        };
        let Some(uri) = terminal.hyperlink_hover_uri() else {
            return;
        };

        let launcher = gtk::UriLauncher::new(&uri);
        let window = window_weak.upgrade();
        launcher.launch(window.as_ref(), None::<&gtk::gio::Cancellable>, |result| {
            if let Err(error) = result {
                eprintln!("zter: could not open hyperlink: {error}");
            }
        });
        gesture.set_state(gtk::EventSequenceState::Claimed);
    });
    terminal.add_controller(click);
}

fn is_control_hyperlink_click(modifiers: gtk::gdk::ModifierType) -> bool {
    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let other_modifiers = gtk::gdk::ModifierType::SHIFT_MASK
        | gtk::gdk::ModifierType::ALT_MASK
        | gtk::gdk::ModifierType::SUPER_MASK
        | gtk::gdk::ModifierType::HYPER_MASK
        | gtk::gdk::ModifierType::META_MASK;
    control && !modifiers.intersects(other_modifiers)
}

fn install_clipboard_context_menu(terminal: &vte4::Terminal) {
    let popover = gtk::Popover::new();
    popover.add_css_class("zter-clipboard-menu");
    popover.set_autohide(true);
    popover.set_has_arrow(false);
    popover.set_parent(terminal);
    let popover_for_destroy = popover.clone();
    terminal.connect_destroy(move |_| {
        if popover_for_destroy.parent().is_some() {
            popover_for_destroy.unparent();
        }
    });

    let menu = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let copy_button = clipboard_menu_button("Copy", "Ctrl+C");
    let paste_button = clipboard_menu_button("Paste", "Ctrl+V");
    menu.append(&copy_button);
    menu.append(&paste_button);
    popover.set_child(Some(&menu));

    let terminal_weak = terminal.downgrade();
    let popover_weak = popover.downgrade();
    copy_button.connect_clicked(move |_| {
        if let Some(terminal) = terminal_weak.upgrade() {
            terminal.copy_clipboard_format(vte4::Format::Text);
        }
        if let Some(popover) = popover_weak.upgrade() {
            popover.popdown();
        }
    });

    let terminal_weak = terminal.downgrade();
    let popover_weak = popover.downgrade();
    paste_button.connect_clicked(move |_| {
        if let Some(terminal) = terminal_weak.upgrade() {
            terminal.paste_clipboard();
        }
        if let Some(popover) = popover_weak.upgrade() {
            popover.popdown();
        }
    });

    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_SECONDARY);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let terminal_weak = terminal.downgrade();
    let popover_weak = popover.downgrade();
    click.connect_pressed(move |gesture, _, x, y| {
        let (Some(terminal), Some(popover)) = (terminal_weak.upgrade(), popover_weak.upgrade())
        else {
            return;
        };
        copy_button.set_sensitive(terminal.has_selection());
        popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
        gesture.set_state(gtk::EventSequenceState::Claimed);
    });
    terminal.add_controller(click);
}

fn clipboard_menu_button(label: &str, shortcut: &str) -> gtk::Button {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);

    let label = gtk::Label::new(Some(label));
    label.set_halign(gtk::Align::Start);
    label.set_hexpand(true);
    row.append(&label);

    let shortcut = gtk::Label::new(Some(shortcut));
    shortcut.add_css_class("zter-clipboard-shortcut");
    shortcut.set_halign(gtk::Align::End);
    row.append(&shortcut);

    let button = gtk::Button::new();
    button.add_css_class("zter-clipboard-menu-item");
    button.set_child(Some(&row));
    button
}

fn create_content<F>(
    terminal: &vte4::Terminal,
    config: &AppConfig,
    wallpaper: &WallpaperAsset,
    on_initial_size: F,
) -> gtk::Overlay
where
    F: FnOnce() + 'static,
{
    let overlay = gtk::Overlay::new();
    overlay.add_css_class("zter-content");
    overlay.set_overflow(gtk::Overflow::Hidden);

    let background = create_background(wallpaper);
    let terminal_viewport = create_terminal_viewport(terminal, config, on_initial_size);
    let terminal_scrollbar = create_terminal_scrollbar(terminal);
    overlay.set_child(Some(&background));
    overlay.add_overlay(&terminal_viewport);
    overlay.add_overlay(&terminal_scrollbar);
    overlay.set_measure_overlay(&terminal_scrollbar, false);

    overlay
}

fn create_terminal_scrollbar(terminal: &vte4::Terminal) -> gtk::Scrollbar {
    let adjustment = terminal.vadjustment().unwrap_or_else(|| {
        let adjustment = gtk::Adjustment::new(0.0, 0.0, 0.0, 1.0, 10.0, 10.0);
        terminal.set_vadjustment(Some(&adjustment));
        adjustment
    });
    let scrollbar = gtk::Scrollbar::new(gtk::Orientation::Vertical, Some(&adjustment));
    scrollbar.add_css_class("zter-terminal-scrollbar");
    scrollbar.set_halign(gtk::Align::End);
    scrollbar.set_valign(gtk::Align::Fill);
    sync_terminal_scrollbar(&scrollbar, &adjustment);

    let scrollbar_weak = scrollbar.downgrade();
    adjustment.connect_changed(move |adjustment| {
        if let Some(scrollbar) = scrollbar_weak.upgrade() {
            sync_terminal_scrollbar(&scrollbar, adjustment);
        }
    });

    scrollbar
}

fn sync_terminal_scrollbar(scrollbar: &gtk::Scrollbar, adjustment: &gtk::Adjustment) {
    let has_scrollback = terminal_has_scrollback(
        adjustment.lower(),
        adjustment.upper(),
        adjustment.page_size(),
    );
    scrollbar.set_can_target(has_scrollback);
    if has_scrollback {
        scrollbar.remove_css_class(TERMINAL_SCROLLBAR_HIDDEN_CLASS);
    } else {
        scrollbar.add_css_class(TERMINAL_SCROLLBAR_HIDDEN_CLASS);
    }
}

fn terminal_has_scrollback(lower: f64, upper: f64, page_size: f64) -> bool {
    upper - lower > page_size + f64::EPSILON
}

fn create_terminal_viewport<F>(
    terminal: &vte4::Terminal,
    config: &AppConfig,
    on_initial_size: F,
) -> gtk::ScrolledWindow
where
    F: FnOnce() + 'static,
{
    let padding = config.terminal_padding();
    let terminal_surface = gtk::Fixed::new();
    terminal_surface.put(terminal, 0.0, 0.0);
    let scroll_content = gtk::Viewport::builder()
        .scroll_to_focus(false)
        .child(&terminal_surface)
        .build();

    let viewport = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .vscrollbar_policy(gtk::PolicyType::External)
        .has_frame(false)
        .kinetic_scrolling(false)
        .hexpand(true)
        .vexpand(true)
        .child(&scroll_content)
        .build();
    install_deferred_terminal_resize(&viewport, terminal, padding, on_initial_size);
    install_terminal_zoom(terminal, config.font_size());
    viewport
}

fn install_terminal_zoom(terminal: &vte4::Terminal, initial_font_size: f64) {
    let control = TerminalZoomControl::new(terminal, initial_font_size);

    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let control_for_key = control.clone();
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(zoom) = terminal_zoom_shortcut(key, modifiers) else {
            return gtk::glib::Propagation::Proceed;
        };
        control_for_key.request(zoom);
        gtk::glib::Propagation::Stop
    });
    terminal.add_controller(key_controller);

    let scroll_controller = gtk::EventControllerScroll::new(
        gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::DISCRETE,
    );
    scroll_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    scroll_controller.connect_scroll(move |controller, _, dy| {
        let Some(zoom) = terminal_zoom_scroll(controller.current_event_state(), dy) else {
            return gtk::glib::Propagation::Proceed;
        };
        control.request(zoom);
        gtk::glib::Propagation::Stop
    });
    terminal.add_controller(scroll_controller);
}

fn terminal_zoom_shortcut(
    key: gtk::gdk::Key,
    modifiers: gtk::gdk::ModifierType,
) -> Option<TerminalZoom> {
    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let system_modifiers = gtk::gdk::ModifierType::ALT_MASK
        | gtk::gdk::ModifierType::SHIFT_MASK
        | gtk::gdk::ModifierType::SUPER_MASK
        | gtk::gdk::ModifierType::HYPER_MASK
        | gtk::gdk::ModifierType::META_MASK;
    if !control || modifiers.intersects(system_modifiers) {
        return None;
    }

    match key {
        gtk::gdk::Key::equal => Some(TerminalZoom::In),
        gtk::gdk::Key::minus => Some(TerminalZoom::Out),
        _ => None,
    }
}

fn terminal_zoom_scroll(modifiers: gtk::gdk::ModifierType, dy: f64) -> Option<TerminalZoom> {
    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let other_modifiers = gtk::gdk::ModifierType::SHIFT_MASK
        | gtk::gdk::ModifierType::ALT_MASK
        | gtk::gdk::ModifierType::SUPER_MASK
        | gtk::gdk::ModifierType::HYPER_MASK
        | gtk::gdk::ModifierType::META_MASK;
    if !control || modifiers.intersects(other_modifiers) {
        return None;
    }

    if dy < 0.0 {
        Some(TerminalZoom::In)
    } else if dy > 0.0 {
        Some(TerminalZoom::Out)
    } else {
        None
    }
}

fn zoomed_font_size(current: f64, zoom: TerminalZoom) -> f64 {
    let delta = match zoom {
        TerminalZoom::In => TERMINAL_ZOOM_STEP,
        TerminalZoom::Out => -TERMINAL_ZOOM_STEP,
    };
    (current + delta).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
}

fn install_deferred_terminal_resize<F>(
    viewport: &gtk::ScrolledWindow,
    terminal: &vte4::Terminal,
    padding: TerminalPadding,
    on_initial_size: F,
) where
    F: FnOnce() + 'static,
{
    let resize = Rc::new(RefCell::new(DeferredTerminalResize::default()));
    let pending = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));
    let on_initial_size = RefCell::new(Some(on_initial_size));
    let terminal_weak = terminal.downgrade();

    viewport.add_tick_callback(move |viewport, _| {
        let size = (viewport.width(), viewport.height());
        match resize.borrow_mut().observe(size) {
            TerminalResizeAction::Ignore => {}
            TerminalResizeAction::ApplyInitial(size) => {
                if let Some(terminal) = terminal_weak.upgrade() {
                    apply_terminal_viewport_size(&terminal, size, padding);
                    let on_initial_size = on_initial_size.borrow_mut().take();
                    if let Some(on_initial_size) = on_initial_size {
                        on_initial_size();
                    }
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
    size: (i32, i32),
    padding: TerminalPadding,
) {
    let (width, height) = size;
    apply_terminal_grid_size(terminal, size, padding);
    terminal.set_size_request(width, height);
}

fn apply_terminal_grid_size(terminal: &vte4::Terminal, size: (i32, i32), padding: TerminalPadding) {
    let grid_size = terminal_grid_size(
        size,
        padding,
        (terminal.char_width(), terminal.char_height()),
    );
    terminal.set_size(grid_size.0, grid_size.1);
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
    let preparation = wallpaper_preparation(config, display);
    match prepare_wallpaper(preparation).map(wallpaper_texture) {
        Ok(texture) => WallpaperAsset::new(texture),
        Err(error) => {
            eprintln!("zter: {error}; using the theme background");
            WallpaperAsset::default()
        }
    }
}

fn install_settings_reload_action(
    application: &gtk::Application,
    display: &gtk::gdk::Display,
    wallpaper: &WallpaperAsset,
) {
    if application.lookup_action(SETTINGS_RELOAD_ACTION).is_some() {
        return;
    }

    let action = gtk::gio::SimpleAction::new(SETTINGS_RELOAD_ACTION, None);
    let display = display.clone();
    let wallpaper = wallpaper.clone();
    action.connect_activate(move |_, _| {
        let config = match AppConfig::from_environment() {
            Ok(config) => config,
            Err(error) => {
                eprintln!("zter: could not reload settings: {error}");
                return;
            }
        };
        reload_wallpaper(&wallpaper, wallpaper_preparation(&config, &display));
    });
    application.add_action(&action);
}

fn reload_wallpaper(wallpaper: &WallpaperAsset, preparation: WallpaperPreparation) {
    let generation = wallpaper.begin_reload();
    let worker = std::thread::spawn(move || {
        prepare_wallpaper(preparation).map_err(|error| error.to_string())
    });
    let worker = Rc::new(RefCell::new(Some(worker)));
    let wallpaper = wallpaper.clone();

    gtk::glib::timeout_add_local(Duration::from_millis(16), move || {
        let is_finished = worker
            .borrow()
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished);
        if !is_finished {
            return gtk::glib::ControlFlow::Continue;
        }

        let result = worker
            .borrow_mut()
            .take()
            .expect("finished wallpaper worker must be present")
            .join();
        if !wallpaper.is_current_reload(generation) {
            return gtk::glib::ControlFlow::Break;
        }
        let result = match result {
            Ok(result) => result,
            Err(_) => {
                eprintln!(
                    "zter: wallpaper reload worker stopped unexpectedly; keeping the current wallpaper"
                );
                return gtk::glib::ControlFlow::Break;
            }
        };
        let texture = match result {
            Ok(prepared) => wallpaper_texture(prepared),
            Err(error) => {
                eprintln!("zter: {error}; keeping the current wallpaper");
                return gtk::glib::ControlFlow::Break;
            }
        };

        wallpaper.replace(texture);
        eprintln!("zter: reloaded wallpaper settings");
        gtk::glib::ControlFlow::Break
    });
}

fn wallpaper_preparation(config: &AppConfig, display: &gtk::gdk::Display) -> WallpaperPreparation {
    let background = theme::background_color(config.theme());
    WallpaperPreparation {
        source: config.wallpaper().cloned(),
        display_size: display_pixel_size(display),
        background: [
            f64::from(background.red()),
            f64::from(background.green()),
            f64::from(background.blue()),
            f64::from(background.alpha()),
        ],
        opacity: config.wallpaper_opacity(),
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

fn prepare_wallpaper(
    preparation: WallpaperPreparation,
) -> Result<Option<PreparedWallpaper>, WallpaperPreparationError> {
    let Some(source) = preparation.source.as_ref() else {
        return Ok(None);
    };
    let wallpaper = load_wallpaper(source)
        .map_err(|error| WallpaperPreparationError::Load(error.to_string()))?;
    let size = downscaled_wallpaper_size(
        (wallpaper.width(), wallpaper.height()),
        preparation.display_size,
    );
    let wallpaper = if size == (wallpaper.width(), wallpaper.height()) {
        wallpaper
    } else {
        wallpaper
            .scale_simple(size.0, size.1, gtk::gdk_pixbuf::InterpType::Hyper)
            .ok_or(WallpaperPreparationError::Downscale)?
    };
    let mut surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, size.0, size.1)?;
    let context = gtk::cairo::Context::new(&surface)?;
    context.set_source_rgba(
        preparation.background[0],
        preparation.background[1],
        preparation.background[2],
        preparation.background[3],
    );
    context.paint()?;
    context.set_operator(WALLPAPER_BLEND_OPERATOR);
    context.set_source_pixbuf(&wallpaper, 0.0, 0.0);
    context.paint_with_alpha(preparation.opacity)?;
    drop(context);
    surface.flush();

    let stride = usize::try_from(surface.stride())
        .map_err(|error| WallpaperPreparationError::PixelAccess(error.to_string()))?;
    let pixels = surface
        .data()
        .map_err(|error| WallpaperPreparationError::PixelAccess(error.to_string()))?
        .to_vec();
    Ok(Some(PreparedWallpaper {
        width: size.0,
        height: size.1,
        stride,
        pixels,
    }))
}

fn wallpaper_texture(prepared: Option<PreparedWallpaper>) -> Option<gtk::gdk::Texture> {
    let Some(prepared) = prepared else {
        return None;
    };
    let bytes = gtk::glib::Bytes::from_owned(prepared.pixels);
    #[cfg(target_endian = "little")]
    let format = gtk::gdk::MemoryFormat::B8g8r8a8Premultiplied;
    #[cfg(target_endian = "big")]
    let format = gtk::gdk::MemoryFormat::A8r8g8b8Premultiplied;
    Some(
        gtk::gdk::MemoryTexture::new(
            prepared.width,
            prepared.height,
            format,
            &bytes,
            prepared.stride,
        )
        .upcast(),
    )
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
    wallpaper.create_background()
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

fn spawn_shell(
    terminal: &vte4::Terminal,
    config: &AppConfig,
    working_directory: &str,
    tab_id: &str,
    close_protection: &CloseProtection,
) {
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
    let tab_id = tab_id.to_owned();
    let close_protection = close_protection.clone();

    terminal.spawn_async(
        vte4::PtyFlags::DEFAULT,
        Some(working_directory),
        &argv,
        &environment,
        gtk::glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(pid) => {
                if let Some(shell_pid) = close_protection
                    .shell_pids
                    .borrow_mut()
                    .get_mut(tab_id.as_str())
                {
                    *shell_pid = Some(pid.0);
                }
            }
            Err(error) => {
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
        let initial = (960, 600);
        let latest = (840, 560);

        assert_eq!(
            resize.observe(initial),
            TerminalResizeAction::ApplyInitial(initial)
        );
        assert_eq!(resize.observe(initial), TerminalResizeAction::Ignore);
        assert_eq!(resize.observe((900, 580)), TerminalResizeAction::Defer);
        assert_eq!(resize.observe(latest), TerminalResizeAction::Defer);
        assert_eq!(resize.settle(), Some(latest));
        assert_eq!(resize.observe((1_020, 640)), TerminalResizeAction::Defer);
        assert_eq!(resize.settle(), Some((1_020, 640)));
        assert_eq!(resize.settle(), None);
    }

    #[test]
    fn terminal_resize_ignores_an_unchanged_viewport_after_font_zoom() {
        let mut resize = DeferredTerminalResize::default();
        let viewport_size = (960, 600);

        assert_eq!(
            resize.observe(viewport_size),
            TerminalResizeAction::ApplyInitial(viewport_size)
        );
        assert_eq!(resize.observe(viewport_size), TerminalResizeAction::Ignore);
        assert_eq!(resize.settle(), None);
    }

    #[test]
    fn terminal_resize_waits_for_a_positive_initial_allocation() {
        let mut resize = DeferredTerminalResize::default();
        let initial = (960, 600);

        assert_eq!(resize.observe((0, 0)), TerminalResizeAction::Ignore);
        assert_eq!(resize.observe((960, 0)), TerminalResizeAction::Ignore);
        assert_eq!(
            resize.observe(initial),
            TerminalResizeAction::ApplyInitial(initial)
        );
    }

    #[test]
    fn terminal_grid_excludes_padding_and_the_content_divider() {
        let padding = TerminalPadding::new(10, 20, 30, 40);

        assert_eq!(terminal_grid_size((860, 541), padding, (10, 20)), (80, 25));
    }

    #[test]
    fn terminal_grid_uses_zoomed_cell_metrics_for_the_same_viewport() {
        let padding = TerminalPadding::new(16, 16, 16, 16);

        assert_eq!(terminal_grid_size((960, 600), padding, (8, 16)), (116, 35));
        assert_eq!(terminal_grid_size((960, 600), padding, (10, 20)), (92, 28));
    }

    #[test]
    fn terminal_scrollbar_appears_only_when_history_exceeds_the_page() {
        assert!(!terminal_has_scrollback(0.0, 24.0, 24.0));
        assert!(!terminal_has_scrollback(10.0, 34.0, 24.0));
        assert!(terminal_has_scrollback(0.0, 25.0, 24.0));
        assert!(terminal_has_scrollback(10.0, 35.0, 24.0));
    }

    #[test]
    fn tab_shortcuts_cover_creation_and_navigation() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;
        let control_alt = control | gtk::gdk::ModifierType::ALT_MASK;

        assert_eq!(
            tab_shortcut(gtk::gdk::Key::t, control),
            Some(TabShortcut::New)
        );
        assert_eq!(
            tab_shortcut(gtk::gdk::Key::T, control),
            Some(TabShortcut::New)
        );
        assert_eq!(tab_shortcut(gtk::gdk::Key::t, control_shift), None);
        assert_eq!(tab_shortcut(gtk::gdk::Key::t, control_alt), None);
        assert_eq!(tab_shortcut(gtk::gdk::Key::w, control_shift), None);
        assert_eq!(
            tab_shortcut(gtk::gdk::Key::Page_Up, control),
            Some(TabShortcut::Previous)
        );
        assert_eq!(
            tab_shortcut(gtk::gdk::Key::Page_Down, control),
            Some(TabShortcut::Next)
        );
        assert_eq!(tab_shortcut(gtk::gdk::Key::Page_Up, control_shift), None);
    }

    #[test]
    fn terminal_zoom_shortcuts_cover_plain_equal_and_minus() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;
        let control_alt = control | gtk::gdk::ModifierType::ALT_MASK;

        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::equal, control),
            Some(TerminalZoom::In)
        );
        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::minus, control),
            Some(TerminalZoom::Out)
        );
        assert_eq!(terminal_zoom_shortcut(gtk::gdk::Key::KP_Add, control), None);
        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::KP_Subtract, control),
            None
        );
        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::equal, control_alt),
            None
        );
        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::equal, gtk::gdk::ModifierType::empty()),
            None
        );
        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::equal, control_shift),
            None
        );
        assert_eq!(
            terminal_zoom_shortcut(gtk::gdk::Key::minus, control_shift),
            None
        );
    }

    #[test]
    fn terminal_zoom_scroll_requires_plain_control_and_vertical_motion() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;

        assert_eq!(terminal_zoom_scroll(control, -1.0), Some(TerminalZoom::In));
        assert_eq!(terminal_zoom_scroll(control, 1.0), Some(TerminalZoom::Out));
        assert_eq!(terminal_zoom_scroll(control, 0.0), None);
        assert_eq!(
            terminal_zoom_scroll(gtk::gdk::ModifierType::empty(), -1.0),
            None
        );
        assert_eq!(terminal_zoom_scroll(control_shift, -1.0), None);
    }

    #[test]
    fn terminal_zoom_steps_and_clamps_to_the_supported_font_range() {
        assert_eq!(zoomed_font_size(12.0, TerminalZoom::In), 13.0);
        assert_eq!(zoomed_font_size(12.0, TerminalZoom::Out), 11.0);
        assert_eq!(
            zoomed_font_size(MAX_FONT_SIZE, TerminalZoom::In),
            MAX_FONT_SIZE
        );
        assert_eq!(
            zoomed_font_size(MIN_FONT_SIZE, TerminalZoom::Out),
            MIN_FONT_SIZE
        );
    }

    #[test]
    fn terminal_zoom_applies_each_request_without_a_settle_delay() {
        let mut zoom = TerminalZoomState::new(12.0);

        assert_eq!(zoom.request(TerminalZoom::In), Some(13.0));
        assert_eq!(zoom.request(TerminalZoom::In), Some(14.0));
        assert_eq!(zoom.request(TerminalZoom::Out), Some(13.0));
        assert_eq!(zoom.font_size, 13.0);
    }

    #[test]
    fn terminal_zoom_state_honors_the_supported_bounds() {
        let mut maximum = TerminalZoomState::new(MAX_FONT_SIZE);
        let mut minimum = TerminalZoomState::new(MIN_FONT_SIZE);

        assert_eq!(maximum.request(TerminalZoom::In), None);
        assert_eq!(minimum.request(TerminalZoom::Out), None);
    }

    #[test]
    fn terminal_font_scale_covers_the_supported_point_range() {
        assert_eq!(terminal_font_scale(TERMINAL_FONT_SCALE_BASE_SIZE), 1.0);
        assert_eq!(terminal_font_scale(MIN_FONT_SIZE), 0.3);
        assert_eq!(terminal_font_scale(MAX_FONT_SIZE), 3.6);
        assert!(terminal_font_scale(MIN_FONT_SIZE) >= 0.25);
        assert!(terminal_font_scale(MAX_FONT_SIZE) <= 4.0);
    }

    #[test]
    fn tab_drop_highlight_requires_another_tab_under_the_pointer() {
        assert!(should_highlight_tab_drop_target(
            Some("zter-tab-1"),
            "zter-tab-2",
            true
        ));
        assert!(!should_highlight_tab_drop_target(
            Some("zter-tab-1"),
            "zter-tab-1",
            true
        ));
        assert!(!should_highlight_tab_drop_target(
            Some("external-item"),
            "zter-tab-2",
            true
        ));
        assert!(!should_highlight_tab_drop_target(
            Some("zter-tab-1"),
            "zter-tab-2",
            false
        ));
        assert!(!should_highlight_tab_drop_target(None, "zter-tab-2", true));
    }

    #[test]
    fn clipboard_shortcuts_use_control_without_shift() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;
        let keycodes = ClipboardShortcutKeycodes {
            copy: vec![54],
            paste: vec![55],
        };

        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::c, 0, control, &keycodes),
            Some(ClipboardShortcut::Copy)
        );
        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::v, 0, control, &keycodes),
            Some(ClipboardShortcut::Paste)
        );
        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::c, 54, control_shift, &keycodes),
            None
        );
        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::v, 55, control_shift, &keycodes),
            None
        );
        assert_eq!(
            clipboard_shortcut(
                gtk::gdk::Key::c,
                54,
                gtk::gdk::ModifierType::empty(),
                &keycodes
            ),
            None
        );
    }

    #[test]
    fn clipboard_shortcuts_match_physical_keys_across_layouts() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let keycodes = ClipboardShortcutKeycodes {
            copy: vec![54],
            paste: vec![55],
        };

        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::Thai_saraae, 54, control, &keycodes),
            Some(ClipboardShortcut::Copy)
        );
        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::Thai_oang, 55, control, &keycodes),
            Some(ClipboardShortcut::Paste)
        );
        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::Thai_saraae, 55, control, &keycodes),
            Some(ClipboardShortcut::Paste)
        );
        assert_eq!(
            clipboard_shortcut(gtk::gdk::Key::Thai_oang, 56, control, &keycodes),
            None
        );
    }

    #[test]
    fn hyperlink_activation_requires_plain_control() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;
        let control_alt = control | gtk::gdk::ModifierType::ALT_MASK;

        assert!(is_control_hyperlink_click(control));
        assert!(!is_control_hyperlink_click(control_shift));
        assert!(!is_control_hyperlink_click(control_alt));
        assert!(!is_control_hyperlink_click(gtk::gdk::ModifierType::empty()));
    }

    #[test]
    fn clipboard_copy_route_confirms_only_foreground_interrupts_without_selection() {
        assert_eq!(
            clipboard_copy_route(true, true),
            ClipboardCopyRoute::CopySelection
        );
        assert_eq!(
            clipboard_copy_route(true, false),
            ClipboardCopyRoute::CopySelection
        );
        assert_eq!(
            clipboard_copy_route(false, true),
            ClipboardCopyRoute::ConfirmInterrupt
        );
        assert_eq!(
            clipboard_copy_route(false, false),
            ClipboardCopyRoute::PassThrough
        );
    }

    #[test]
    fn foreground_process_shortcuts_route_control_d_and_control_z() {
        let control = gtk::gdk::ModifierType::CONTROL_MASK;
        let control_shift = control | gtk::gdk::ModifierType::SHIFT_MASK;
        let control_alt = control | gtk::gdk::ModifierType::ALT_MASK;

        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::d, control),
            Some(ForegroundProcessShortcut::ConfirmEndOfInput)
        );
        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::D, control),
            Some(ForegroundProcessShortcut::ConfirmEndOfInput)
        );
        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::d, control_shift),
            None
        );
        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::z, control),
            Some(ForegroundProcessShortcut::ConfirmSuspend)
        );
        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::Z, control_shift),
            Some(ForegroundProcessShortcut::Suppress)
        );
        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::z, control_alt),
            None
        );
        assert_eq!(
            foreground_process_shortcut(gtk::gdk::Key::d, gtk::gdk::ModifierType::empty()),
            None
        );
        assert_eq!(foreground_process_shortcut(gtk::gdk::Key::c, control), None);
    }

    #[test]
    fn clipboard_paste_routes_text_to_vte_and_passes_non_text_through() {
        assert_eq!(clipboard_paste_route(true), ClipboardPasteRoute::PasteText);
        assert_eq!(
            clipboard_paste_route(false),
            ClipboardPasteRoute::PassThrough
        );
    }

    #[test]
    fn close_protection_allows_an_idle_shell_to_close_immediately() {
        assert!(!process_groups_have_running_foreground_process(42, 42));
    }

    #[test]
    fn close_protection_detects_a_foreground_process_started_by_the_shell() {
        assert!(process_groups_have_running_foreground_process(42, 84));
    }

    #[test]
    fn close_protection_does_not_block_when_process_groups_are_unavailable() {
        assert!(!process_groups_have_running_foreground_process(-1, 84));
        assert!(!process_groups_have_running_foreground_process(42, -1));
    }

    #[test]
    fn default_tab_title_uses_the_shell_executable_name() {
        assert_eq!(default_tab_title("/bin/bash"), "bash in zter");
        assert_eq!(default_tab_title("fish"), "fish in zter");
    }

    #[test]
    fn local_directory_uri_is_decoded_for_a_new_tab() {
        assert_eq!(
            local_path_from_uri("file:///tmp/zter%20working%20directory"),
            Some(PathBuf::from("/tmp/zter working directory"))
        );
        assert_eq!(local_path_from_uri("sftp://example.com/tmp"), None);
    }

    #[test]
    fn shell_working_directory_can_be_read_from_proc() {
        assert_eq!(
            process_working_directory(std::process::id() as libc::pid_t),
            env::current_dir().ok()
        );
    }

    #[test]
    fn new_tab_directory_falls_back_to_the_window_startup_directory() {
        assert_eq!(
            working_directory_or_fallback(None, "/window/startup"),
            "/window/startup"
        );
        assert_eq!(
            working_directory_or_fallback(
                Some(PathBuf::from("/directory/that/does/not/exist")),
                "/window/startup"
            ),
            "/window/startup"
        );
        let current_directory = env::current_dir().unwrap();
        assert_eq!(
            working_directory_or_fallback(Some(current_directory.clone()), "/window/startup"),
            current_directory.to_str().unwrap()
        );
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
    fn recognized_status_glyphs_are_kept_before_a_manual_title() {
        for glyph in TERMINAL_STATUS_GLYPHS {
            let automatic = format!("{glyph} agent is working");
            assert_eq!(
                recognized_terminal_status(&automatic),
                Some((TerminalTitleStatus::Glyph(glyph), "agent is working"))
            );

            let mut state = TabTitleState::new(automatic);
            state.save_manual("งานหลัก");
            assert_eq!(state.displayed(), format!("{glyph} งานหลัก"));
            assert_eq!(state.editable(), "งานหลัก");
        }
    }

    #[test]
    fn status_glyph_requires_a_leading_position_and_whitespace_boundary() {
        assert_eq!(
            recognized_terminal_status("◐"),
            Some((TerminalTitleStatus::Glyph('◐'), ""))
        );
        assert_eq!(recognized_terminal_status("◐working"), None);
        assert_eq!(recognized_terminal_status("agent ◐ working"), None);
        assert_eq!(recognized_terminal_status("⠁ working"), None);
        assert_eq!(recognized_terminal_status("🦀 working"), None);
    }

    #[test]
    fn exact_action_required_status_is_kept_before_a_manual_title() {
        assert_eq!(
            recognized_terminal_status("[ ! ] Action Required"),
            Some((TerminalTitleStatus::ActionRequired, ""))
        );
        assert_eq!(
            recognized_terminal_status("[ ! ] Action Required | Review changes"),
            Some((TerminalTitleStatus::ActionRequired, "Review changes"))
        );

        let mut state = TabTitleState::new("[ ! ] Action Required | Review changes".to_owned());
        state.save_manual("project");
        assert_eq!(state.displayed(), "[ ! ] Action Required | project");
        assert_eq!(state.editable(), "project");
    }

    #[test]
    fn malformed_action_required_status_is_not_recognized() {
        assert_eq!(
            recognized_terminal_status("[!] Action Required | Review changes"),
            None
        );
        assert_eq!(
            recognized_terminal_status("[ ! ] Action Required Review changes"),
            None
        );
        assert_eq!(
            recognized_terminal_status("[ ! ] Action Required later | Review changes"),
            None
        );
    }

    #[test]
    fn manual_title_tracks_status_changes_and_removal() {
        let mut state = TabTitleState::new("⠋ working".to_owned());
        state.save_manual("project");
        assert_eq!(state.displayed(), "⠋ project");

        state.update_automatic("⠙ working".to_owned());
        assert_eq!(state.displayed(), "⠙ project");

        state.update_automatic("unknown automatic title".to_owned());
        assert_eq!(state.displayed(), "project");

        state.update_automatic("bash in zter".to_owned());
        assert_eq!(state.displayed(), "project");
    }

    #[test]
    fn editor_omits_status_when_no_manual_title_exists() {
        let glyph = TabTitleState::new("✦ กำลังทำงาน".to_owned());
        assert_eq!(glyph.displayed(), "✦ กำลังทำงาน");
        assert_eq!(glyph.editable(), "กำลังทำงาน");

        let action = TabTitleState::new("[ ! ] Action Required | ตรวจสอบการเปลี่ยนแปลง".to_owned());
        assert_eq!(action.editable(), "ตรวจสอบการเปลี่ยนแปลง");

        let ordinary = TabTitleState::new("日本語の端末".to_owned());
        assert_eq!(ordinary.editable(), "日本語の端末");
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
        let background = theme::background_color(crate::settings::Theme::OneHalfDark);
        let prepared = prepare_wallpaper(WallpaperPreparation {
            source: Some(WallpaperSource::Bundled),
            display_size: (960, 600),
            background: [
                f64::from(background.red()),
                f64::from(background.green()),
                f64::from(background.blue()),
                f64::from(background.alpha()),
            ],
            opacity: 0.15,
        })
        .unwrap()
        .unwrap();

        assert_eq!(
            (prepared.width, prepared.height),
            downscaled_wallpaper_size((wallpaper.width(), wallpaper.height()), (960, 600))
        );
        #[cfg(target_endian = "little")]
        let alpha_offset = 3;
        #[cfg(target_endian = "big")]
        let alpha_offset = 0;
        for (x, y) in [
            (0, 0),
            (prepared.width / 2, prepared.height / 2),
            (prepared.width - 1, prepared.height - 1),
        ] {
            let alpha = usize::try_from(y).unwrap() * prepared.stride
                + usize::try_from(x).unwrap() * 4
                + alpha_offset;
            assert_eq!(prepared.pixels[alpha], u8::MAX);
        }
    }

    #[test]
    fn disabled_wallpaper_prepares_without_image_work() {
        let prepared = prepare_wallpaper(WallpaperPreparation {
            source: None,
            display_size: (960, 600),
            background: [0.0, 0.0, 0.0, 1.0],
            opacity: 0.15,
        })
        .unwrap();

        assert!(prepared.is_none());
    }

    #[test]
    fn wallpaper_reload_uses_sendable_worker_data_and_latest_generation() {
        fn assert_send<T: Send>() {}
        assert_send::<WallpaperPreparation>();
        assert_send::<PreparedWallpaper>();

        let wallpaper = WallpaperAsset::default();
        let first = wallpaper.begin_reload();
        let second = wallpaper.begin_reload();

        assert!(!wallpaper.is_current_reload(first));
        assert!(wallpaper.is_current_reload(second));
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
