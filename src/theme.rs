use gtk::gdk;
use vte4::prelude::*;

use crate::settings::{TerminalPadding, Theme};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb(u8, u8, u8);

const BACKGROUND: Rgb = Rgb(0x28, 0x2c, 0x34);
const FOREGROUND: Rgb = Rgb(0xdc, 0xdf, 0xe4);
const HEADER_BACKGROUND: Rgb = Rgb(0x30, 0x36, 0x43);
const TAB_HOVER: Rgb = Rgb(0x35, 0x3b, 0x48);
const HEADER_BUTTON_HOVER: Rgb = Rgb(0x44, 0x4a, 0x55);
const TAB_CLOSE_HOVER: Rgb = Rgb(0x5c, 0x63, 0x70);
const TAB_DROP_TARGET: Rgb = Rgb(0xff, 0xff, 0xff);
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
    let tab_drop_target = TAB_DROP_TARGET.css();
    let mut css = format!(
        "\
        window.zter-window {{
            background-color: transparent;
            background-image: none;
            color: {};
            box-shadow: none;
            border: 1px solid {};
            border-radius: 12px;
        }}
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
        window.zter-settings-window .zter-settings-header windowcontrols button {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            min-height: 28px;
            min-width: 28px;
            margin: 0 2px;
            padding: 0;
        }}
        window.zter-window .zter-header windowcontrols button:hover,
        window.zter-settings-window .zter-settings-header windowcontrols button:hover {{
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
        .zter-drag-space {{
            min-width: 40px;
            outline-width: 0;
        }}
        .zter-drag-space.zter-header-drop-target {{
            outline: 1px solid {};
            outline-offset: -1px;
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
        .zter-header-tab.zter-tab-drop-target {{
            border-width: 0;
            outline: 1px solid {};
            outline-offset: -1px;
            box-shadow: none;
        }}
        .zter-header-tab.zter-tab-drop-target.zter-tab-drop-before {{
            background-image: linear-gradient(to right, transparent 24.5%, {tab_drop_target} 24.5%, {tab_drop_target} 25.5%, transparent 25.5%);
        }}
        .zter-header-tab.zter-tab-drop-target.zter-tab-drop-after {{
            background-image: linear-gradient(to right, transparent 74.5%, {tab_drop_target} 74.5%, {tab_drop_target} 75.5%, transparent 75.5%);
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
            border-radius: 999px;
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
            margin: 0 0 0 4px;
            padding: 0;
        }}
        button.zter-tab-close:hover {{
            background-color: {};
        }}
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
        scrollbar.zter-terminal-scrollbar {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
            margin: 6px 4px 6px 0;
            min-width: 8px;
            opacity: 0.72;
            transition: opacity 140ms ease-out;
        }}
        scrollbar.zter-terminal-scrollbar:hover {{
            opacity: 1;
        }}
        scrollbar.zter-terminal-scrollbar.zter-terminal-scrollbar-hidden {{
            opacity: 0;
        }}
        scrollbar.zter-terminal-scrollbar trough {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
            min-width: 8px;
        }}
        scrollbar.zter-terminal-scrollbar slider {{
            background-color: {};
            background-image: none;
            border-width: 0;
            border-radius: 4px;
            box-shadow: none;
            min-height: 24px;
            min-width: 8px;
        }}
        .zter-content {{
            background-color: transparent;
            border-radius: 0 0 12px 12px;
        }}
        picture.zter-background {{
            background-color: transparent;
            background-image: none;
            border-radius: 0 0 12px 12px;
        }}",
        FOREGROUND.css(),
        SELECTION.css(),
        HEADER_BACKGROUND.css(),
        FOREGROUND.css(),
        TAB_DROP_TARGET.css(),
        HEADER_BACKGROUND.css(),
        TAB_HOVER.css(),
        SELECTION.css(),
        TAB_DROP_TARGET.css(),
        BACKGROUND.css(),
        TAB_CLOSE_HOVER.css(),
        HEADER_BUTTON_HOVER.css(),
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
        Rgb(0x5c, 0x63, 0x70).css()
    );
    css.push_str(&settings_window_css());
    css
}

fn settings_window_css() -> String {
    format!(
        "\
        window.zter-settings-window {{
            background-color: transparent;
            background-image: none;
            border-radius: 12px;
            box-shadow: none;
        }}
        window.zter-settings-window .zter-settings-surface {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 12px;
            box-shadow: none;
        }}
        window.zter-settings-window .zter-settings-header {{
            background-color: {};
            background-image: none;
            border-bottom: 1px solid {};
            box-shadow: none;
            min-height: 36px;
        }}
        window.zter-settings-window .zter-settings-title {{
            color: {};
            font-weight: 600;
            padding-left: 16px;
        }}
        window.zter-settings-window .zter-settings-form {{
            padding: 16px;
        }}
        window.zter-settings-window frame.zter-settings-group {{
            background-color: transparent;
            background-image: none;
            border: 1px solid {};
            border-radius: 7px;
            box-shadow: none;
            padding: 2px 12px 12px;
        }}
        window.zter-settings-window frame.zter-settings-group > label {{
            background-color: {};
            color: {};
            margin-left: 0;
            padding: 0 4px 0 0;
        }}
        window.zter-settings-window .zter-settings-padding {{
            margin-top: 6px;
        }}
        window.zter-settings-window .zter-settings-field {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
            padding: 0;
        }}
        window.zter-settings-window .zter-settings-field-title {{
            color: {};
            font-size: 12px;
            min-height: 18px;
            padding-left: 2px;
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox {{
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            color: {};
            margin: 0;
            min-height: 18px;
            padding: 0;
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox label {{
            font-size: 12px;
            min-height: 18px;
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox check {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 4px;
            box-shadow: none;
            color: {};
            min-height: 14px;
            min-width: 14px;
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox:hover check {{
            border-color: #9DA5B4;
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox:checked check {{
            background-color: {};
            border-color: #9DA5B4;
            color: #282C34;
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox:focus check {{
            border-color: #DCDFE4;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio {{
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            color: #DCDFE4;
            margin: 0;
            padding: 0;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio radio {{
            background-color: #282C34;
            background-image: none;
            border: 1px solid #3E4451;
            box-shadow: none;
            color: #DCDFE4;
            min-height: 14px;
            min-width: 14px;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio:hover radio {{
            border-color: #9DA5B4;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio:checked radio {{
            background-color: #9DA5B4;
            background-image: none;
            border-color: #9DA5B4;
            color: #282C34;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio:focus radio {{
            border-color: #DCDFE4;
        }}
        window.zter-settings-window .zter-settings-field > entry,
        window.zter-settings-window .zter-settings-field > spinbutton,
        window.zter-settings-window .zter-settings-field > .zter-settings-value {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 7px;
            box-shadow: none;
            color: {};
            min-height: 36px;
            outline-width: 0;
            padding: 0 10px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale trough {{
            background-color: #303643;
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            min-height: 6px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale highlight {{
            background-color: #9DA5B4;
            background-image: none;
            border-radius: 999px;
            box-shadow: none;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled trough,
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled highlight {{
            background-color: #5C6370;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale slider {{
            background-color: #9DA5B4;
            background-image: none;
            border: 1px solid #5C6370;
            border-radius: 999px;
            box-shadow: none;
            min-height: 14px;
            min-width: 14px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:hover slider,
        window.zter-settings-window scale.zter-settings-opacity-scale:focus slider {{
            background-color: #DCDFE4;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:focus slider {{
            border-color: #DCDFE4;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled slider {{
            background-color: #5C6370;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale value {{
            color: #DCDFE4;
            min-width: 34px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled value {{
            color: #9DA5B4;
        }}
        window.zter-settings-window .zter-settings-field > entry:disabled,
        window.zter-settings-window .zter-settings-field > spinbutton:disabled,
        window.zter-settings-window .zter-settings-field > .zter-settings-value:disabled,
        window.zter-settings-window checkbutton.zter-settings-checkbox:disabled,
        window.zter-settings-window .zter-settings-actions button:disabled {{
            opacity: 0.3;
        }}
        window.zter-settings-window .zter-settings-field spinbutton entry {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 0;
            box-shadow: none;
            color: {};
            min-height: 32px;
            outline-width: 0;
            padding: 0;
        }}
        window.zter-settings-window .zter-settings-field > entry:focus,
        window.zter-settings-window .zter-settings-field > spinbutton:focus,
        window.zter-settings-window .zter-settings-field spinbutton entry:focus {{
            border-color: {};
            box-shadow: none;
            outline-width: 0;
        }}
        window.zter-settings-window .zter-settings-field spinbutton button {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            color: {};
            min-height: 28px;
            min-width: 28px;
            margin: 4px 2px;
            padding: 0;
        }}
        window.zter-settings-window .zter-settings-field spinbutton button:hover {{
            background-color: {};
        }}
        window.zter-settings-window .zter-settings-actions {{
            border-top: 1px solid {};
            padding: 12px 16px;
        }}
        window.zter-settings-window .zter-settings-status {{
            color: {};
            margin-right: 8px;
        }}
        window.zter-settings-window .zter-settings-actions button {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 7px;
            box-shadow: none;
            color: {};
            min-height: 32px;
            min-width: 76px;
            outline-width: 0;
            padding: 0 14px;
        }}
        window.zter-settings-window .zter-settings-actions button:hover {{
            background-color: {};
        }}
        window.zter-settings-window .zter-settings-actions button:focus {{
            box-shadow: none;
            outline-width: 0;
        }}
        window.zter-window .zter-header button.zter-settings-button {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            min-height: 28px;
            min-width: 28px;
            margin: 0 2px;
            padding: 0;
            transition: background-color 180ms ease-out;
        }}
        window.zter-window .zter-header button.zter-settings-button:hover {{
            background-color: {};
        }}",
        BACKGROUND.css(),
        SELECTION.css(),
        HEADER_BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        SELECTION.css(),
        BACKGROUND.css(),
        Rgb(0x9d, 0xa5, 0xb4).css(),
        Rgb(0x9d, 0xa5, 0xb4).css(),
        Rgb(0x9d, 0xa5, 0xb4).css(),
        BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        Rgb(0x9d, 0xa5, 0xb4).css(),
        BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        FOREGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        HEADER_BUTTON_HOVER.css(),
        SELECTION.css(),
        Rgb(0xe0, 0x6c, 0x75).css(),
        HEADER_BACKGROUND.css(),
        SELECTION.css(),
        FOREGROUND.css(),
        TAB_HOVER.css(),
        HEADER_BUTTON_HOVER.css()
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

        assert_eq!(css.matches("border:").count(), 10);
        assert_eq!(css.matches("border-top:").count(), 3);
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
        let (_, header_rule) = css.split_once("window.zter-window .zter-header {").unwrap();
        let (header_rule, _) = header_rule.split_once('}').unwrap();

        assert!(header_rule.contains("background-color: #303643"));
        assert!(header_rule.contains("border-bottom-width: 0"));
        assert!(!header_rule.contains("#E06C75"));
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
    fn background_surface_css_is_transparent_for_prepared_alpha_pixels() {
        let css = application_css(TerminalPadding::default());
        let (_, background_rule) = css.split_once("picture.zter-background").unwrap();
        let (background_rule, _) = background_rule.split_once('}').unwrap();

        assert!(background_rule.contains("background-color: transparent"));
        assert!(background_rule.contains("background-image: none"));
    }

    #[test]
    fn app_window_css_allows_terminal_background_alpha() {
        let css = application_css(TerminalPadding::default());
        let (_, window_rule) = css.split_once("window.zter-window").unwrap();
        let (window_rule, _) = window_rule.split_once('}').unwrap();

        assert!(window_rule.contains("background-color: transparent"));
        assert!(window_rule.contains("border: 1px solid #3E4451"));
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
    fn terminal_scrollbar_is_overlay_styled_without_a_shadow() {
        let css = application_css(TerminalPadding::default());
        let (_, scrollbar_rule) = css
            .split_once("scrollbar.zter-terminal-scrollbar {")
            .unwrap();
        let (scrollbar_rule, _) = scrollbar_rule.split_once('}').unwrap();
        let (_, hidden_rule) = css
            .split_once("scrollbar.zter-terminal-scrollbar.zter-terminal-scrollbar-hidden {")
            .unwrap();
        let (hidden_rule, _) = hidden_rule.split_once('}').unwrap();

        assert!(scrollbar_rule.contains("background-color: transparent"));
        assert!(scrollbar_rule.contains("box-shadow: none"));
        assert!(scrollbar_rule.contains("min-width: 8px"));
        assert!(hidden_rule.contains("opacity: 0"));
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
    fn tab_drop_target_uses_an_inset_white_outline() {
        let css = application_css(TerminalPadding::default());
        let (_, target_rule) = css
            .split_once(".zter-header-tab.zter-tab-drop-target")
            .unwrap();
        let (target_rule, _) = target_rule.split_once('}').unwrap();

        assert!(!target_rule.contains("background-color:"));
        assert!(target_rule.contains("border-width: 0"));
        assert!(target_rule.contains("outline: 1px solid #FFFFFF"));
        assert!(target_rule.contains("outline-offset: -1px"));
        assert!(target_rule.contains("box-shadow: none"));
    }

    #[test]
    fn tab_drop_target_marks_space_evenly_inset_positions() {
        let css = application_css(TerminalPadding::default());
        let (_, before_rule) = css
            .split_once(".zter-tab-drop-target.zter-tab-drop-before")
            .unwrap();
        let (before_rule, _) = before_rule.split_once('}').unwrap();
        let (_, after_rule) = css
            .split_once(".zter-tab-drop-target.zter-tab-drop-after")
            .unwrap();
        let (after_rule, _) = after_rule.split_once('}').unwrap();

        assert!(before_rule.contains("transparent 24.5%, #FFFFFF 24.5%"));
        assert!(before_rule.contains("#FFFFFF 25.5%, transparent 25.5%"));
        assert!(after_rule.contains("transparent 74.5%, #FFFFFF 74.5%"));
        assert!(after_rule.contains("#FFFFFF 75.5%, transparent 75.5%"));
    }

    #[test]
    fn blank_header_drop_target_uses_an_inset_white_outline() {
        let css = application_css(TerminalPadding::default());
        let (_, target_rule) = css
            .split_once(".zter-drag-space.zter-header-drop-target")
            .unwrap();
        let (target_rule, _) = target_rule.split_once('}').unwrap();

        assert!(target_rule.contains("outline: 1px solid #FFFFFF"));
        assert!(target_rule.contains("outline-offset: -1px"));
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
    fn titlebar_drag_spaces_keep_a_40px_minimum() {
        let css = application_css(TerminalPadding::default());
        let (_, drag_space_rule) = css.split_once(".zter-drag-space").unwrap();
        let (drag_space_rule, _) = drag_space_rule.split_once('}').unwrap();

        assert!(drag_space_rule.contains("min-width: 40px"));
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
    fn new_tab_buttons_touch_their_following_drag_spaces() {
        let css = application_css(TerminalPadding::default());
        let (_, new_tab_rule) = css
            .split_once("button.zter-new-tab {\n            background-color")
            .unwrap();
        let (new_tab_rule, _) = new_tab_rule.split_once('}').unwrap();

        assert!(new_tab_rule.contains("margin: 0 0 0 4px"));
    }

    #[test]
    fn settings_fields_use_a_border_without_input_focus_rings() {
        let css = application_css(TerminalPadding::default());
        let (_, input_rule) = css
            .split_once("window.zter-settings-window .zter-settings-field > entry,")
            .unwrap();
        let (input_rule, _) = input_rule.split_once('}').unwrap();
        let (_, focus_rule) = css
            .split_once("window.zter-settings-window .zter-settings-field > entry:focus")
            .unwrap();
        let (focus_rule, _) = focus_rule.split_once('}').unwrap();

        assert!(input_rule.contains("border: 1px solid #3E4451"));
        assert!(input_rule.contains("box-shadow: none"));
        assert!(input_rule.contains("min-height: 36px"));
        assert!(focus_rule.contains("outline-width: 0"));
        assert!(focus_rule.contains("box-shadow: none"));
    }

    #[test]
    fn settings_form_uses_comfortable_spacing() {
        let css = application_css(TerminalPadding::default());
        let (_, form_rule) = css
            .split_once("window.zter-settings-window .zter-settings-form {")
            .unwrap();
        let (form_rule, _) = form_rule.split_once('}').unwrap();
        let (_, label_rule) = css
            .split_once("window.zter-settings-window .zter-settings-field-title {")
            .unwrap();
        let (label_rule, _) = label_rule.split_once('}').unwrap();

        assert!(form_rule.contains("padding: 16px"));
        assert!(label_rule.contains("font-size: 12px"));
        assert!(label_rule.contains("min-height: 18px"));
    }

    #[test]
    fn settings_opacity_checkbox_is_compact_and_shadow_free() {
        let css = application_css(TerminalPadding::default());
        let (_, checkbox_rule) = css
            .split_once("checkbutton.zter-settings-checkbox {")
            .unwrap();
        let (checkbox_rule, _) = checkbox_rule.split_once('}').unwrap();
        let (_, label_rule) = css
            .split_once("checkbutton.zter-settings-checkbox label {")
            .unwrap();
        let (label_rule, _) = label_rule.split_once('}').unwrap();
        let (_, check_rule) = css
            .split_once("checkbutton.zter-settings-checkbox check {")
            .unwrap();
        let (check_rule, _) = check_rule.split_once('}').unwrap();
        let (_, checked_rule) = css
            .split_once("checkbutton.zter-settings-checkbox:checked check {")
            .unwrap();
        let (checked_rule, _) = checked_rule.split_once('}').unwrap();

        assert!(checkbox_rule.contains("background-color: transparent"));
        assert!(checkbox_rule.contains("box-shadow: none"));
        assert!(checkbox_rule.contains("min-height: 18px"));
        assert!(label_rule.contains("font-size: 12px"));
        assert!(check_rule.contains("background-color: #282C34"));
        assert!(check_rule.contains("border: 1px solid #3E4451"));
        assert!(check_rule.contains("box-shadow: none"));
        assert!(check_rule.contains("min-height: 14px"));
        assert!(check_rule.contains("min-width: 14px"));
        assert!(checked_rule.contains("background-color: #9DA5B4"));
        assert!(checked_rule.contains("border-color: #9DA5B4"));
        assert!(checked_rule.contains("color: #282C34"));
    }

    #[test]
    fn settings_background_image_modes_use_neutral_radio_controls() {
        let css = application_css(TerminalPadding::default());
        let (_, radio_rule) = css
            .split_once("checkbutton.zter-settings-radio radio {")
            .unwrap();
        let (radio_rule, _) = radio_rule.split_once('}').unwrap();
        let (_, checked_rule) = css
            .split_once("checkbutton.zter-settings-radio:checked radio {")
            .unwrap();
        let (checked_rule, _) = checked_rule.split_once('}').unwrap();

        assert!(radio_rule.contains("background-color: #282C34"));
        assert!(radio_rule.contains("background-image: none"));
        assert!(radio_rule.contains("border: 1px solid #3E4451"));
        assert!(radio_rule.contains("box-shadow: none"));
        assert!(radio_rule.contains("min-height: 14px"));
        assert!(radio_rule.contains("min-width: 14px"));
        assert!(checked_rule.contains("background-color: #9DA5B4"));
        assert!(checked_rule.contains("background-image: none"));
        assert!(checked_rule.contains("border-color: #9DA5B4"));
        assert!(checked_rule.contains("color: #282C34"));
    }

    #[test]
    fn settings_opacity_scale_uses_the_neutral_input_surface() {
        let css = application_css(TerminalPadding::default());
        let (_, trough_rule) = css
            .split_once("scale.zter-settings-opacity-scale trough {")
            .unwrap();
        let (trough_rule, _) = trough_rule.split_once('}').unwrap();
        let (_, highlight_rule) = css
            .split_once("scale.zter-settings-opacity-scale highlight {")
            .unwrap();
        let (highlight_rule, _) = highlight_rule.split_once('}').unwrap();
        let (_, slider_rule) = css
            .split_once("scale.zter-settings-opacity-scale slider {")
            .unwrap();
        let (slider_rule, _) = slider_rule.split_once('}').unwrap();
        let (_, disabled_track_rule) = css
            .split_once("scale.zter-settings-opacity-scale:disabled trough,")
            .unwrap();
        let (disabled_track_rule, _) = disabled_track_rule.split_once('}').unwrap();
        let (_, disabled_control_rule) = css
            .split_once(".zter-settings-field > entry:disabled,")
            .unwrap();
        let (disabled_control_rule, _) = disabled_control_rule.split_once('}').unwrap();

        assert!(trough_rule.contains("background-color: #303643"));
        assert!(trough_rule.contains("box-shadow: none"));
        assert!(highlight_rule.contains("background-color: #9DA5B4"));
        assert!(disabled_track_rule.contains("background-color: #5C6370"));
        assert!(disabled_control_rule.contains("opacity: 0.3"));
        assert!(slider_rule.contains("background-color: #9DA5B4"));
        assert!(slider_rule.contains("border: 1px solid #5C6370"));
        assert!(slider_rule.contains("box-shadow: none"));
    }

    #[test]
    fn settings_selection_controls_have_distinct_hover_and_focus_states() {
        let css = application_css(TerminalPadding::default());

        assert!(css.contains(
            "checkbutton.zter-settings-checkbox:hover check {\n            border-color: #9DA5B4"
        ));
        assert!(css.contains(
            "checkbutton.zter-settings-checkbox:focus check {\n            border-color: #DCDFE4"
        ));
        assert!(css.contains(
            "checkbutton.zter-settings-radio:hover radio {\n            border-color: #9DA5B4"
        ));
        assert!(css.contains(
            "checkbutton.zter-settings-radio:focus radio {\n            border-color: #DCDFE4"
        ));
        assert!(css.contains(
            "scale.zter-settings-opacity-scale:hover slider,\n        window.zter-settings-window scale.zter-settings-opacity-scale:focus slider {\n            background-color: #DCDFE4"
        ));
        assert!(css.contains(
            "scale.zter-settings-opacity-scale:focus slider {\n            border-color: #DCDFE4"
        ));
    }

    #[test]
    fn settings_selection_controls_do_not_use_the_blue_terminal_accent() {
        let css = settings_window_css();

        assert!(!css.contains(&CURSOR.css()));
    }

    #[test]
    fn padding_group_uses_a_neutral_border_and_integrated_title() {
        let css = application_css(TerminalPadding::default());
        let (_, group_rule) = css
            .split_once("window.zter-settings-window frame.zter-settings-group {")
            .unwrap();
        let (group_rule, _) = group_rule.split_once('}').unwrap();
        let (_, title_rule) = css
            .split_once("window.zter-settings-window frame.zter-settings-group > label {")
            .unwrap();
        let (title_rule, _) = title_rule.split_once('}').unwrap();

        assert!(group_rule.contains("border: 1px solid #3E4451"));
        assert!(group_rule.contains("box-shadow: none"));
        assert!(title_rule.contains("background-color: #282C34"));
        assert!(title_rule.contains("margin-left: 0"));
        assert!(title_rule.contains("padding: 0 4px 0 0"));
    }

    #[test]
    fn settings_close_button_matches_the_terminal_window_controls() {
        let css = application_css(TerminalPadding::default());
        let selector = "window.zter-settings-window .zter-settings-header windowcontrols button";
        let (_, close_rule) = css.split_once(selector).unwrap();
        let (close_rule, remainder) = close_rule.split_once('}').unwrap();
        let (_, close_hover) = remainder.split_once(&format!("{selector}:hover")).unwrap();
        let (close_hover, _) = close_hover.split_once('}').unwrap();

        assert!(close_rule.contains("background-color: transparent"));
        assert!(close_rule.contains("border-radius: 999px"));
        assert!(close_rule.contains("min-height: 28px"));
        assert!(close_rule.contains("min-width: 28px"));
        assert!(close_rule.contains("box-shadow: none"));
        assert!(!close_rule.contains("transition:"));
        assert!(close_hover.contains("background-color: transparent"));
    }

    #[test]
    fn settings_spin_buttons_match_the_terminal_window_controls() {
        let css = application_css(TerminalPadding::default());
        let selector = "window.zter-settings-window .zter-settings-field spinbutton button";
        let (_, button_rule) = css.split_once(selector).unwrap();
        let (button_rule, remainder) = button_rule.split_once('}').unwrap();
        let (_, hover_rule) = remainder.split_once(&format!("{selector}:hover")).unwrap();
        let (hover_rule, _) = hover_rule.split_once('}').unwrap();

        assert!(button_rule.contains("background-color: transparent"));
        assert!(button_rule.contains("border-radius: 999px"));
        assert!(button_rule.contains("min-height: 28px"));
        assert!(button_rule.contains("min-width: 28px"));
        assert!(button_rule.contains("margin: 4px 2px"));
        assert!(!button_rule.contains("transition:"));
        assert!(hover_rule.contains("background-color: #444A55"));
    }

    #[test]
    fn settings_header_matches_the_terminal_header_height() {
        let css = application_css(TerminalPadding::default());
        let (_, settings_header_rule) = css
            .split_once("window.zter-settings-window .zter-settings-header {")
            .unwrap();
        let (settings_header_rule, _) = settings_header_rule.split_once('}').unwrap();
        let (_, terminal_header_rule) =
            css.split_once("window.zter-window .zter-header {").unwrap();
        let (terminal_header_rule, _) = terminal_header_rule.split_once('}').unwrap();

        assert!(settings_header_rule.contains("min-height: 36px"));
        assert!(terminal_header_rule.contains("min-height: 36px"));
    }

    #[test]
    fn settings_actions_are_separated_and_shadow_free() {
        let css = application_css(TerminalPadding::default());
        let (_, actions_rule) = css
            .split_once("window.zter-settings-window .zter-settings-actions {")
            .unwrap();
        let (actions_rule, _) = actions_rule.split_once('}').unwrap();
        let (_, button_rule) = css
            .split_once("window.zter-settings-window .zter-settings-actions button {")
            .unwrap();
        let (button_rule, _) = button_rule.split_once('}').unwrap();

        assert!(actions_rule.contains("border-top: 1px solid #3E4451"));
        assert!(actions_rule.contains("padding: 12px 16px"));
        assert!(button_rule.contains("background-color: #303643"));
        assert!(button_rule.contains("border: 1px solid #3E4451"));
        assert!(button_rule.contains("color: #DCDFE4"));
        assert!(button_rule.contains("min-height: 32px"));
        assert!(button_rule.contains("box-shadow: none"));
        assert!(!button_rule.contains("transition:"));
        assert!(!css.contains("button.zter-settings-ok {"));
    }

    #[test]
    fn settings_button_matches_the_compact_header_controls() {
        let css = application_css(TerminalPadding::default());
        let (_, button_rule) = css
            .split_once("window.zter-window .zter-header button.zter-settings-button {")
            .unwrap();
        let (button_rule, _) = button_rule.split_once('}').unwrap();

        assert!(button_rule.contains("min-height: 28px"));
        assert!(button_rule.contains("min-width: 28px"));
        assert!(button_rule.contains("border-radius: 999px"));
        assert!(button_rule.contains("box-shadow: none"));
    }

    #[test]
    fn app_owned_header_buttons_use_visible_hover_fills() {
        let css = application_css(TerminalPadding::default());
        let (_, tab_close_hover) = css.split_once("button.zter-tab-close:hover {").unwrap();
        let (tab_close_hover, _) = tab_close_hover.split_once('}').unwrap();
        let (_, new_tab_hover) = css.split_once("button.zter-new-tab:hover {").unwrap();
        let (new_tab_hover, _) = new_tab_hover.split_once('}').unwrap();
        let (_, settings_button_hover) = css
            .split_once("window.zter-window .zter-header button.zter-settings-button:hover {")
            .unwrap();
        let (settings_button_hover, _) = settings_button_hover.split_once('}').unwrap();

        assert!(tab_close_hover.contains("background-color: #5C6370"));
        assert!(new_tab_hover.contains("background-color: #444A55"));
        assert!(settings_button_hover.contains("background-color: #444A55"));
    }

    #[test]
    fn tab_close_button_is_compact_and_circular() {
        let css = application_css(TerminalPadding::default());
        let (_, close_button) = css.split_once("button.zter-tab-close {").unwrap();
        let (close_button, _) = close_button.split_once('}').unwrap();

        assert!(close_button.contains("border-radius: 999px"));
        assert!(close_button.contains("min-height: 20px"));
        assert!(close_button.contains("min-width: 20px"));
    }
}
