use std::env;
use std::f64::consts::{FRAC_PI_2, PI};

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

    let terminal = create_terminal(config);
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
