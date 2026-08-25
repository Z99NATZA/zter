use gtk::gdk;
use vte4::prelude::*;

use crate::settings::{TerminalPadding, Theme};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb(u8, u8, u8);

const BACKGROUND: Rgb = Rgb(0x28, 0x2c, 0x34);
const FOREGROUND: Rgb = Rgb(0xdc, 0xdf, 0xe4);
const HEADER_BACKGROUND: Rgb = Rgb(0x30, 0x36, 0x43);
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
    let selection = rgba(SELECTION);
    let selection_foreground = rgba(FOREGROUND);
    let palette: Vec<gdk::RGBA> = ANSI_PALETTE.into_iter().map(rgba).collect();
    let palette: Vec<&gdk::RGBA> = palette.iter().collect();

    terminal.set_colors(Some(&foreground), Some(&background), &palette);
    terminal.set_color_cursor(Some(&cursor));
    terminal.set_color_cursor_foreground(Some(&cursor_foreground));
    terminal.set_color_highlight(Some(&selection));
    terminal.set_color_highlight_foreground(Some(&selection_foreground));
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
        window.zter-window headerbar,
        window.zter-window .titlebar {{
            background-color: {};
            background-image: none;
            border-bottom-width: 0;
            color: {};
            box-shadow: none;
        }}
        window.zter-window headerbar windowcontrols button {{
            background-color: transparent;
            background-image: none;
            box-shadow: none;
        }}
        window.zter-window headerbar windowcontrols button:hover {{
            background-color: {};
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
        }}",
        BACKGROUND.css(),
        FOREGROUND.css(),
        SELECTION.css(),
        HEADER_BACKGROUND.css(),
        FOREGROUND.css(),
        SELECTION.css(),
        SELECTION.css(),
        terminal_padding.top(),
        terminal_padding.right(),
        terminal_padding.bottom(),
        terminal_padding.left()
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
    fn app_css_uses_only_outer_and_divider_borders_and_disables_owned_shadows() {
        let css = application_css(TerminalPadding::default());
        let shadow_rules: Vec<&str> = css
            .lines()
            .filter(|line| line.contains("box-shadow:"))
            .collect();

        assert_eq!(css.matches("border:").count(), 1);
        assert_eq!(css.matches("border-top:").count(), 1);
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
    fn header_uses_reference_tone_without_red_or_a_second_divider() {
        let css = application_css(TerminalPadding::default());

        assert!(css.contains("window.zter-window headerbar"));
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
}
