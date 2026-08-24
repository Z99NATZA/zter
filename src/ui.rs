use std::env;

use gtk::prelude::*;
use vte4::prelude::*;

use crate::{config::AppConfig, theme};

const DEFAULT_WIDTH: i32 = 960;
const DEFAULT_HEIGHT: i32 = 600;
const SCROLLBACK_LINES: i64 = 10_000;
const WALLPAPER_SHADE_OPACITY: f64 = 0.42;

pub fn build(application: &gtk::Application, config: &AppConfig) {
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("zter")
        .default_width(DEFAULT_WIDTH)
        .default_height(DEFAULT_HEIGHT)
        .build();
    window.add_css_class("zter-window");
    theme::install_display_styles(&gtk::prelude::WidgetExt::display(&window));

    let terminal = create_terminal(config.wallpaper().is_some());
    let content = create_content(&terminal, config);

    let window_weak = window.downgrade();
    terminal.connect_child_exited(move |_, _| {
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });

    window.set_child(Some(&content));
    window.present();
    terminal.grab_focus();

    spawn_shell(&terminal, config);
}

fn create_terminal(has_wallpaper: bool) -> vte4::Terminal {
    let terminal = vte4::Terminal::new();
    terminal.add_css_class("zter-terminal");
    terminal.set_hexpand(true);
    terminal.set_vexpand(true);
    terminal.set_scrollback_lines(SCROLLBACK_LINES);
    terminal.set_scroll_on_keystroke(true);
    terminal.set_mouse_autohide(true);
    terminal.set_allow_hyperlink(true);
    install_clipboard_shortcuts(&terminal);
    theme::apply_to(&terminal, has_wallpaper);

    terminal
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

    if let Some(wallpaper) = config.wallpaper() {
        let picture = gtk::Picture::for_filename(wallpaper);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        overlay.set_child(Some(&picture));

        let shade = create_wallpaper_shade();
        overlay.add_overlay(&shade);
        overlay.add_overlay(terminal);
    } else {
        overlay.set_child(Some(terminal));
    }

    overlay
}

fn create_wallpaper_shade() -> gtk::DrawingArea {
    let shade = gtk::DrawingArea::new();
    shade.set_can_target(false);
    shade.set_hexpand(true);
    shade.set_vexpand(true);
    shade.set_draw_func(|_, context, width, height| {
        context.set_source_rgba(0.0, 0.0, 0.0, WALLPAPER_SHADE_OPACITY);
        context.rectangle(0.0, 0.0, f64::from(width), f64::from(height));
        let _ = context.fill();
    });
    shade
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
