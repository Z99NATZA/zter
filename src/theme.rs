use gtk::gdk;
use vte4::prelude::*;

use crate::settings::{TerminalPadding, Theme};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb(u8, u8, u8);

const BACKGROUND: Rgb = Rgb(0x28, 0x2c, 0x34);
const FOREGROUND: Rgb = Rgb(0xdc, 0xdf, 0xe4);
const HEADER_BACKGROUND: Rgb = Rgb(0x30, 0x36, 0x43);
const TAB_HOVER: Rgb = Rgb(0x35, 0x3b, 0x48);
const CURSOR: Rgb = Rgb(0x61, 0xaf, 0xef);
const SELECTION: Rgb = Rgb(0x3e, 0x44, 0x51);
const TRANSPARENT_BACKGROUND_CLASS: &str = "zter-transparent-background";

const ANSI_PALETTE: [Rgb; 16] = [
    Rgb(0x28, 0x2c, 0x34), // black
    Rgb(0xe0, 0x6c, 0x75), // red: error semantics only
    Rgb(0x98, 0xc3, 0x79), // green
    Rgb(0xe5, 0xc0, 0x7b), // yellow
    Rgb(0x61, 0xaf, 0xef), // blue
    Rgb(0xc6, 0x78, 0xdd), // magenta
    Rgb(0x56, 0xb6, 0xc2), // cyan
    Rgb(0xdc, 0xdf, 0xe4), // white
    Rgb(0x5c, 0x63, 0x70), // bright black
    Rgb(0xe0, 0x6c, 0x75), // bright red: error semantics only
    Rgb(0x98, 0xc3, 0x79), // bright green
    Rgb(0xe5, 0xc0, 0x7b), // bright yellow
    Rgb(0x61, 0xaf, 0xef), // bright blue
    Rgb(0xc6, 0x78, 0xdd), // bright magenta
    Rgb(0x56, 0xb6, 0xc2), // bright cyan
    Rgb(0xff, 0xff, 0xff), // bright white
];

pub fn apply_to(terminal: &vte4::Terminal, theme: Theme) {
    match theme {
        Theme::OneHalfDark => apply_one_half_dark(terminal),
    }
}

fn apply_one_half_dark(terminal: &vte4::Terminal) {
    let foreground = rgba(FOREGROUND);
    let background = rgba(BACKGROUND);
    let cursor = rgba(CURSOR);
    let cursor_foreground = rgba(BACKGROUND);
    let palette: Vec<gdk::RGBA> = ANSI_PALETTE.into_iter().map(rgba).collect();
    let palette: Vec<&gdk::RGBA> = palette.iter().collect();

    terminal.set_colors(Some(&foreground), Some(&background), &palette);
    terminal.set_color_cursor(Some(&cursor));
    terminal.set_color_cursor_foreground(Some(&cursor_foreground));
    terminal.set_color_highlight(None);
    terminal.set_color_highlight_foreground(None);
    terminal.add_css_class(TRANSPARENT_BACKGROUND_CLASS);
    terminal.set_clear_background(false);
}

pub fn background_color(theme: Theme) -> gdk::RGBA {
    match theme {
        Theme::OneHalfDark => rgba(BACKGROUND),
    }
}

pub fn install_display_styles(display: &gdk::Display, terminal_padding: TerminalPadding) {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&application_css(terminal_padding));
    gtk::style_context_add_provider_for_display(
        display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn application_css(terminal_padding: TerminalPadding) -> String {
    format!(
        "\
        window.zter-window {{
            background-color: {};
            background-image: none;
            color: {};
            box-shadow: none;
            border: 1px solid {};
            border-radius: 12px;
        }}
        window.zter-window .zter-window-handle,
        window.zter-window .zter-header {{
            background-color: {};
            background-image: none;
            border-bottom-width: 0;
            color: {};
            min-height: 36px;
            padding: 0;
            box-shadow: none;
        }}
        window.zter-window .zter-header windowcontrols button {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            min-height: 28px;
            min-width: 28px;
            margin: 0 2px;
            padding: 0;
            transition: all 180ms ease-out;
        }}
        window.zter-window .zter-header windowcontrols button:hover {{
            background-color: transparent;
        }}
        notebook.zter-tabs,
        notebook.zter-tabs > stack {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
        }}
        .zter-tab-scroller,
        .zter-tab-strip {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
        }}
        .zter-header-tab,
        button.zter-tab-close,
        button.zter-new-tab {{
            transition: background-color 180ms ease-out;
        }}
        .zter-header-tab {{
            background-color: {};
            background-image: none;
            border-width: 0;
            min-height: 36px;
            min-width: 220px;
            outline-width: 0;
            box-shadow: none;
        }}
        .zter-header-tab:hover {{
            background-color: {};
        }}
        .zter-header-tab.zter-tab-active {{
            background-color: {};
            border-width: 0;
            box-shadow: none;
        }}
        button.zter-tab-select {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 0;
            min-height: 36px;
            padding: 0 8px;
            box-shadow: none;
        }}
        button.zter-tab-select:hover {{
            background-color: transparent;
        }}
        entry.zter-tab-title-entry {{
            background-color: {};
            background-image: none;
            border-width: 0;
            border-radius: 0;
            min-height: 28px;
            margin: 4px 8px;
            padding: 0 6px;
            outline-width: 0;
            box-shadow: none;
        }}
        button.zter-tab-close {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
            min-height: 20px;
            min-width: 20px;
            margin-right: 8px;
            padding: 2px;
        }}
        button.zter-new-tab {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            min-height: 28px;
            min-width: 28px;
            margin: 0 30px 0 4px;
            padding: 0;
        }}
        button.zter-tab-close:hover,
        button.zter-new-tab:hover {{
            background-color: {};
        }}
        popover.zter-clipboard-menu {{
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            padding: 0;
        }}
        popover.zter-clipboard-menu > contents {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 8px;
            box-shadow: none;
            padding: 4px;
        }}
        popover.zter-clipboard-menu button.zter-clipboard-menu-item {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 4px;
            box-shadow: none;
            color: {};
            min-height: 30px;
            min-width: 168px;
            padding: 0 10px;
        }}
        popover.zter-clipboard-menu button.zter-clipboard-menu-item:hover {{
            background-color: {};
        }}
        popover.zter-clipboard-menu button.zter-clipboard-menu-item:disabled {{
            opacity: 0.45;
        }}
        popover.zter-clipboard-menu .zter-clipboard-shortcut {{
            color: {};
        }}
        window.zter-close-dialog {{
            background-color: transparent;
            background-image: none;
            border-radius: 12px;
            box-shadow: none;
        }}
        window.zter-close-dialog .zter-close-dialog-surface {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 12px;
            box-shadow: none;
            min-width: 320px;
        }}
        window.zter-close-dialog .zter-close-dialog-message {{
            color: {};
            min-height: 58px;
            padding: 0 20px;
        }}
        window.zter-close-dialog .zter-close-dialog-actions {{
            border-top: 1px solid {};
        }}
        window.zter-close-dialog button {{
            background-color: {};
            background-image: none;
            border-width: 0;
            border-radius: 0;
            box-shadow: none;
            color: {};
            min-height: 44px;
            padding: 0 18px;
            transition: background-color 140ms ease-out;
        }}
        window.zter-close-dialog button:hover {{
            background-color: {};
        }}
        window.zter-close-dialog button.zter-close-dialog-confirm {{
            background-color: rgba(224, 108, 117, 0.14);
            border-left: 1px solid {};
            color: {};
        }}
        window.zter-close-dialog button.zter-close-dialog-confirm:hover {{
            background-color: rgba(224, 108, 117, 0.23);
        }}
        .zter-terminal {{
            border-top: 1px solid {};
            box-shadow: none;
            padding: {}px {}px {}px {}px;
        }}
        .zter-terminal.zter-transparent-background {{
            background-color: transparent;
            background-image: none;
        }}
        .zter-content {{
            background-color: transparent;
            border-radius: 0 0 12px 12px;
        }}
        picture.zter-background {{
            background-color: {};
            background-image: none;
            border-radius: 0 0 12px 12px;
        }}",
        BACKGROUND.css(),
        FOREGROUND.css(),
        SELECTION.css(),
        HEADER_BACKGROUND.css(),
        FOREGROUND.css(),
        HEADER_BACKGROUND.css(),
        TAB_HOVER.css(),
        SELECTION.css(),
        BACKGROUND.css(),
        SELECTION.css(),
        HEADER_BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        SELECTION.css(),
        Rgb(0x9d, 0xa5, 0xb4).css(),
        HEADER_BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        TAB_HOVER.css(),
        BACKGROUND.css(),
        Rgb(0xe0, 0x6c, 0x75).css(),
        SELECTION.css(),
        terminal_padding.top(),
        terminal_padding.right(),
        terminal_padding.bottom(),
        terminal_padding.left(),
        BACKGROUND.css()
    )
}

fn rgba(Rgb(red, green, blue): Rgb) -> gdk::RGBA {
    gdk::RGBA::new(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    )
}

impl Rgb {
    fn css(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_half_dark_palette_has_sixteen_ansi_colors() {
        assert_eq!(ANSI_PALETTE.len(), 16);
    }

    #[test]
    fn red_is_limited_to_normal_and_bright_ansi_red() {
        let red = Rgb(0xe0, 0x6c, 0x75);
        let red_indexes: Vec<usize> = ANSI_PALETTE
            .iter()
            .enumerate()
            .filter_map(|(index, color)| (*color == red).then_some(index))
            .collect();

        assert_eq!(red_indexes, [1, 9]);
    }

    #[test]
    fn app_css_uses_only_meaningful_borders_and_disables_owned_shadows() {
        let css = application_css(TerminalPadding::default());
        let shadow_rules: Vec<&str> = css
            .lines()
            .filter(|line| line.contains("box-shadow:"))
            .collect();

        assert_eq!(css.matches("border:").count(), 3);
        assert_eq!(css.matches("border-top:").count(), 2);
        assert!(!shadow_rules.is_empty());
        assert!(
            shadow_rules
                .iter()
                .all(|rule| rule.trim() == "box-shadow: none;")
        );
        assert!(css.contains("border: 1px solid #3E4451"));
        assert!(css.contains("border-top: 1px solid #3E4451"));
    }

    #[test]
    fn close_dialog_has_uniform_corners_and_a_soft_red_close_action() {
        let css = application_css(TerminalPadding::default());
        let (_, window_rule) = css.split_once("window.zter-close-dialog {").unwrap();
        let (window_rule, _) = window_rule.split_once('}').unwrap();
        let (_, surface_rule) = css
            .split_once("window.zter-close-dialog .zter-close-dialog-surface {")
            .unwrap();
        let (surface_rule, _) = surface_rule.split_once('}').unwrap();
        let (_, close_rule) = css
            .split_once("button.zter-close-dialog-confirm {")
            .unwrap();
        let (close_rule, _) = close_rule.split_once('}').unwrap();

        assert!(window_rule.contains("border-radius: 12px"));
        assert!(surface_rule.contains("border-radius: 12px"));
        assert!(close_rule.contains("background-color: rgba(224, 108, 117, 0.14)"));
        assert!(close_rule.contains("color: #E06C75"));
    }

    #[test]
    fn clipboard_menu_is_compact_and_uses_the_neutral_palette() {
        let css = application_css(TerminalPadding::default());
        let (_, surface_rule) = css
            .split_once("popover.zter-clipboard-menu > contents")
            .unwrap();
        let (surface_rule, _) = surface_rule.split_once('}').unwrap();
        let (_, item_rule) = css.split_once("button.zter-clipboard-menu-item {").unwrap();
        let (item_rule, _) = item_rule.split_once('}').unwrap();

        assert!(surface_rule.contains("background-color: #303643"));
        assert!(surface_rule.contains("border: 1px solid #3E4451"));
        assert!(surface_rule.contains("box-shadow: none"));
        assert!(surface_rule.contains("padding: 4px"));
        assert!(item_rule.contains("min-height: 30px"));
        assert!(item_rule.contains("min-width: 168px"));
        assert!(item_rule.contains("padding: 0 10px"));
    }

    #[test]
    fn header_uses_reference_tone_without_red_or_a_second_divider() {
        let css = application_css(TerminalPadding::default());

        assert!(css.contains("window.zter-window .zter-header"));
        assert!(css.contains("background-color: #303643"));
        assert!(css.contains("border-bottom-width: 0"));
        assert!(!css.contains("#E06C75"));
    }

    #[test]
    fn composed_terminal_css_is_explicitly_transparent() {
        let css = application_css(TerminalPadding::default());
        let (_, wallpaper_rule) = css
            .split_once(".zter-terminal.zter-transparent-background")
            .unwrap();
        let (wallpaper_rule, _) = wallpaper_rule.split_once('}').unwrap();

        assert!(wallpaper_rule.contains("background-color: transparent"));
        assert!(wallpaper_rule.contains("background-image: none"));
    }

    #[test]
    fn prepared_wallpaper_surface_keeps_the_opaque_theme_fallback() {
        let css = application_css(TerminalPadding::default());
        let (_, background_rule) = css.split_once("picture.zter-background").unwrap();
        let (background_rule, _) = background_rule.split_once('}').unwrap();

        assert!(background_rule.contains("background-color: #282C34"));
        assert!(background_rule.contains("background-image: none"));
    }

    #[test]
    fn app_css_rounds_the_window_and_lower_content() {
        let css = application_css(TerminalPadding::default());

        assert!(css.contains("border-radius: 12px"));
        assert!(css.contains("border-radius: 0 0 12px 12px"));
    }

    #[test]
    fn terminal_padding_uses_css_edge_order() {
        let css = application_css(TerminalPadding::new(1, 2, 3, 4));

        assert!(css.contains("padding: 1px 2px 3px 4px"));
    }

    #[test]
    fn active_tab_uses_color_without_a_border_or_shadow() {
        let css = application_css(TerminalPadding::default());
        let (_, active_tab_rule) = css.split_once(".zter-header-tab.zter-tab-active").unwrap();
        let (active_tab_rule, _) = active_tab_rule.split_once('}').unwrap();

        assert!(active_tab_rule.contains("background-color: #3E4451"));
        assert!(active_tab_rule.contains("border-width: 0"));
        assert!(active_tab_rule.contains("box-shadow: none"));
    }

    #[test]
    fn tab_title_editor_stays_compact_without_a_border_or_shadow() {
        let css = application_css(TerminalPadding::default());
        let (_, editor_rule) = css.split_once("entry.zter-tab-title-entry").unwrap();
        let (editor_rule, _) = editor_rule.split_once('}').unwrap();

        assert!(editor_rule.contains("background-color: #282C34"));
        assert!(editor_rule.contains("min-height: 28px"));
        assert!(editor_rule.contains("border-width: 0"));
        assert!(editor_rule.contains("box-shadow: none"));
    }

    #[test]
    fn unified_header_and_tabs_share_one_height() {
        let css = application_css(TerminalPadding::default());
        let (_, header_rule) = css.split_once("window.zter-window .zter-header").unwrap();
        let (header_rule, _) = header_rule.split_once('}').unwrap();
        let (_, tab_rule) = css.split_once(".zter-header-tab {").unwrap();
        let (tab_rule, _) = tab_rule.split_once('}').unwrap();

        assert!(header_rule.contains("min-height: 36px"));
        assert!(tab_rule.contains("min-height: 36px"));
    }

    #[test]
    fn header_controls_use_compact_spacing_without_a_rectangular_hover_fill() {
        let css = application_css(TerminalPadding::default());
        let selector = "window.zter-window .zter-header windowcontrols button";
        let (_, control_rule) = css.split_once(selector).unwrap();
        let (control_rule, remainder) = control_rule.split_once('}').unwrap();
        let (_, hover_rule) = remainder.split_once(&format!("{selector}:hover")).unwrap();
        let (hover_rule, _) = hover_rule.split_once('}').unwrap();

        assert!(control_rule.contains("min-height: 28px"));
        assert!(control_rule.contains("min-width: 28px"));
        assert!(control_rule.contains("margin: 0 2px"));
        assert!(control_rule.contains("border-radius: 999px"));
        assert!(control_rule.contains("transition: all 180ms ease-out"));
        assert!(hover_rule.contains("background-color: transparent"));
    }

    #[test]
    fn app_owned_header_hover_transitions_last_180ms() {
        let css = application_css(TerminalPadding::default());

        assert!(css.contains("transition: background-color 180ms ease-out"));
    }

    #[test]
    fn new_tab_and_window_controls_keep_a_32px_minimum_gap() {
        let css = application_css(TerminalPadding::default());
        let (_, new_tab_rule) = css.rsplit_once("button.zter-new-tab {").unwrap();
        let (new_tab_rule, _) = new_tab_rule.split_once('}').unwrap();
        let (_, control_rule) = css
            .split_once("window.zter-window .zter-header windowcontrols button {")
            .unwrap();
        let (control_rule, _) = control_rule.split_once('}').unwrap();

        assert!(new_tab_rule.contains("margin: 0 30px 0 4px"));
        assert!(control_rule.contains("margin: 0 2px"));
    }
}
