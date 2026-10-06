use gtk::gdk;
use vte4::prelude::*;

use crate::settings::{TerminalPadding, Theme};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb(u8, u8, u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ThemePalette {
    background: Rgb,
    foreground: Rgb,
    muted_foreground: Rgb,
    surface: Rgb,
    header_background: Rgb,
    tab_hover: Rgb,
    header_button_hover: Rgb,
    tab_close_hover: Rgb,
    tab_drop_target: Rgb,
    cursor: Rgb,
    selection: Rgb,
    selection_foreground: Rgb,
    border: Rgb,
    error: Rgb,
    ansi: [Rgb; 16],
}

const BACKGROUND: Rgb = Rgb(0x28, 0x2c, 0x34);
const FOREGROUND: Rgb = Rgb(0xdc, 0xdf, 0xe4);
const HEADER_BACKGROUND: Rgb = Rgb(0x30, 0x36, 0x43);
const TAB_HOVER: Rgb = Rgb(0x35, 0x3b, 0x48);
const HEADER_BUTTON_HOVER: Rgb = Rgb(0x44, 0x4a, 0x55);
const TAB_CLOSE_HOVER: Rgb = Rgb(0x5c, 0x63, 0x70);
const TAB_DROP_TARGET: Rgb = Rgb(0xff, 0xff, 0xff);
const CURSOR: Rgb = Rgb(0x61, 0xaf, 0xef);
const SELECTION: Rgb = Rgb(0x3e, 0x44, 0x51);
const MUTED_FOREGROUND: Rgb = Rgb(0x9d, 0xa5, 0xb4);
const ERROR: Rgb = Rgb(0xe0, 0x6c, 0x75);
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

impl Theme {
    fn palette(self) -> ThemePalette {
        match self {
            Self::OneHalfDark => ThemePalette {
                background: BACKGROUND,
                foreground: FOREGROUND,
                muted_foreground: MUTED_FOREGROUND,
                surface: BACKGROUND,
                header_background: HEADER_BACKGROUND,
                tab_hover: TAB_HOVER,
                header_button_hover: HEADER_BUTTON_HOVER,
                tab_close_hover: TAB_CLOSE_HOVER,
                tab_drop_target: TAB_DROP_TARGET,
                cursor: CURSOR,
                selection: SELECTION,
                selection_foreground: BACKGROUND,
                border: SELECTION,
                error: ERROR,
                ansi: ANSI_PALETTE,
            },
            Self::Purple => zter_palette(
                Rgb(0x8f, 0x5f, 0x8a),
                Rgb(0x2b, 0x22, 0x2d),
                Rgb(0x39, 0x2b, 0x3a),
                Rgb(0x4a, 0x37, 0x4b),
                Rgb(0x5d, 0x45, 0x5b),
                Rgb(0xe9, 0xdd, 0xe9),
                Rgb(0xbd, 0x9b, 0xba),
            ),
            Self::WhiteMist => zter_palette(
                Rgb(0x65, 0x95, 0xb1),
                Rgb(0x20, 0x2b, 0x33),
                Rgb(0x2b, 0x3c, 0x47),
                Rgb(0x37, 0x4d, 0x5b),
                Rgb(0x45, 0x61, 0x72),
                Rgb(0xe6, 0xf1, 0xf5),
                Rgb(0xa9, 0xc8, 0xd8),
            ),
            Self::WhiteSky => zter_palette(
                Rgb(0x6b, 0xa4, 0xf4),
                Rgb(0x1c, 0x2d, 0x40),
                Rgb(0x27, 0x3c, 0x56),
                Rgb(0x32, 0x4e, 0x70),
                Rgb(0x40, 0x62, 0x8c),
                Rgb(0xe7, 0xf2, 0xff),
                Rgb(0xa8, 0xcb, 0xf8),
            ),
            Self::ForestCalm => zter_palette(
                Rgb(0x67, 0xa1, 0x8d),
                Rgb(0x1e, 0x30, 0x2a),
                Rgb(0x29, 0x40, 0x37),
                Rgb(0x35, 0x53, 0x48),
                Rgb(0x43, 0x68, 0x5a),
                Rgb(0xe4, 0xf1, 0xeb),
                Rgb(0xa7, 0xca, 0xbb),
            ),
            Self::OneHalfGray => zter_palette(
                Rgb(0x91, 0x9a, 0xa6),
                Rgb(0x24, 0x29, 0x30),
                Rgb(0x31, 0x38, 0x41),
                Rgb(0x3f, 0x47, 0x52),
                Rgb(0x51, 0x5b, 0x67),
                Rgb(0xe8, 0xeb, 0xee),
                Rgb(0xb7, 0xc0, 0xca),
            ),
            Self::Red => zter_palette(
                Rgb(0xc9, 0x5e, 0x78),
                Rgb(0x35, 0x23, 0x2b),
                Rgb(0x47, 0x2d, 0x37),
                Rgb(0x5b, 0x38, 0x46),
                Rgb(0x70, 0x47, 0x55),
                Rgb(0xf4, 0xe3, 0xe8),
                Rgb(0xd5, 0xa0, 0xae),
            ),
            Self::Mauve => zter_palette(
                Rgb(0xc7, 0x6a, 0x8c),
                Rgb(0x35, 0x25, 0x2e),
                Rgb(0x47, 0x30, 0x3b),
                Rgb(0x5b, 0x3d, 0x4b),
                Rgb(0x70, 0x4d, 0x5d),
                Rgb(0xf3, 0xe5, 0xea),
                Rgb(0xd8, 0xa8, 0xb8),
            ),
        }
    }
}

fn zter_palette(
    accent: Rgb,
    background: Rgb,
    header_background: Rgb,
    tab_hover: Rgb,
    tab_close_hover: Rgb,
    foreground: Rgb,
    muted_foreground: Rgb,
) -> ThemePalette {
    ThemePalette {
        background,
        foreground,
        muted_foreground,
        surface: background,
        header_background,
        tab_hover,
        header_button_hover: tab_hover,
        tab_close_hover,
        tab_drop_target: accent,
        cursor: accent,
        selection: tab_hover,
        selection_foreground: foreground,
        border: tab_hover,
        error: ERROR,
        ansi: themed_ansi(background, foreground, muted_foreground, accent),
    }
}

fn themed_ansi(background: Rgb, foreground: Rgb, muted_foreground: Rgb, accent: Rgb) -> [Rgb; 16] {
    [
        background,
        ERROR,
        Rgb(0x8f, 0xc0, 0x7a),
        Rgb(0xd7, 0xb4, 0x69),
        accent,
        Rgb(0xc5, 0x86, 0xc0),
        Rgb(0x6f, 0xc8, 0xd8),
        foreground,
        muted_foreground,
        ERROR,
        Rgb(0x9d, 0xc9, 0x80),
        Rgb(0xe4, 0xc0, 0x78),
        accent,
        Rgb(0xd0, 0x99, 0xcb),
        Rgb(0x7f, 0xd7, 0xe5),
        Rgb(0xff, 0xff, 0xff),
    ]
}

pub fn apply_to(terminal: &vte4::Terminal, theme: Theme) {
    apply_palette(terminal, theme.palette());
}

fn apply_palette(terminal: &vte4::Terminal, theme: ThemePalette) {
    let foreground = rgba(theme.foreground);
    let background = rgba(theme.background);
    let cursor = rgba(theme.cursor);
    let cursor_foreground = rgba(theme.background);
    let palette: Vec<gdk::RGBA> = theme.ansi.into_iter().map(rgba).collect();
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
    rgba(theme.palette().background)
}

pub fn install_display_styles(
    display: &gdk::Display,
    theme: Theme,
    terminal_padding: TerminalPadding,
) {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(&application_css_for(theme, terminal_padding));
    gtk::style_context_add_provider_for_display(
        display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn application_css(terminal_padding: TerminalPadding) -> String {
    application_css_for(Theme::OneHalfDark, terminal_padding)
}

fn application_css_for(theme: Theme, terminal_padding: TerminalPadding) -> String {
    let palette = theme.palette();
    let tab_drop_target = palette.tab_drop_target.css();
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
        window.zter-window .zter-mini-header {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
            min-height: 10px;
            transition: none;
        }}
        window.zter-window .zter-mini-header.zter-mini-header-active {{
            background-color: rgba(171, 178, 191, 0.12);
        }}
        window.zter-window .zter-mini-controls {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            box-shadow: none;
            margin: 0 6px 0 0;
        }}
        window.zter-window .zter-mini-controls button.zter-mini-control,
        window.zter-window .zter-mini-controls button.zter-mini-control:hover,
        window.zter-window .zter-mini-controls button.zter-mini-control:active {{
            background-color: transparent;
            background-image: none;
            border-width: 0;
            border-radius: 0;
            box-shadow: none;
            color: {};
            min-height: 10px;
            min-width: 10px;
            margin: 0;
            padding: 0;
            transition: none;
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
            min-width: 80px;
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
        button.zter-tab-scroll-button {{
            background-color: {};
            background-image: none;
            border-width: 0;
            border-radius: 0;
            box-shadow: none;
            min-height: 36px;
            min-width: 28px;
            padding: 0;
            transition: background-color 180ms ease-out;
        }}
        button.zter-tab-scroll-button:hover {{
            background-color: {};
        }}
        popover.zter-context-menu {{
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            padding: 0;
        }}
        popover.zter-context-menu > contents {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 8px;
            box-shadow: none;
            padding: 4px;
        }}
        popover.zter-context-menu button.zter-context-menu-item {{
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
        popover.zter-context-menu button.zter-context-menu-item:hover {{
            background-color: {};
        }}
        popover.zter-context-menu button.zter-context-menu-item:disabled {{
            opacity: 0.45;
        }}
        popover.zter-context-menu .zter-context-menu-shortcut {{
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
        palette.foreground.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.header_background.css(),
        palette.foreground.css(),
        palette.tab_drop_target.css(),
        palette.header_background.css(),
        palette.tab_hover.css(),
        palette.selection.css(),
        palette.tab_drop_target.css(),
        palette.background.css(),
        palette.tab_close_hover.css(),
        palette.header_button_hover.css(),
        palette.header_background.css(),
        palette.header_button_hover.css(),
        palette.header_background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.selection.css(),
        palette.muted_foreground.css(),
        palette.header_background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.tab_hover.css(),
        palette.background.css(),
        palette.error.css(),
        palette.border.css(),
        terminal_padding.top(),
        terminal_padding.right(),
        terminal_padding.bottom(),
        terminal_padding.left(),
        palette.tab_close_hover.css()
    );
    css.push_str(&settings_window_css_for(theme));
    css
}

fn settings_window_css() -> String {
    settings_window_css_for(Theme::OneHalfDark)
}

fn settings_window_css_for(theme: Theme) -> String {
    let palette = theme.palette();
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
            border-color: {};
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox:checked check {{
            background-color: {};
            border-color: {};
            color: {};
        }}
        window.zter-settings-window checkbutton.zter-settings-checkbox:focus check {{
            border-color: {};
        }}
        window.zter-settings-window checkbutton.zter-settings-radio {{
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            color: {};
            margin: 0;
            padding: 0;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio radio {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            box-shadow: none;
            color: {};
            min-height: 14px;
            min-width: 14px;
        }}
        window.zter-settings-window checkbutton.zter-settings-radio:hover radio {{
            border-color: {};
        }}
        window.zter-settings-window checkbutton.zter-settings-radio:checked radio {{
            background-color: {};
            background-image: none;
            border-color: {};
            color: {};
        }}
        window.zter-settings-window checkbutton.zter-settings-radio:focus radio {{
            border-color: {};
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
            background-color: {};
            background-image: none;
            border-width: 0;
            border-radius: 999px;
            box-shadow: none;
            min-height: 6px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale highlight {{
            background-color: {};
            background-image: none;
            border-radius: 999px;
            box-shadow: none;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled trough,
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled highlight {{
            background-color: {};
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale slider {{
            background-color: {};
            background-image: none;
            border: 1px solid {};
            border-radius: 999px;
            box-shadow: none;
            min-height: 14px;
            min-width: 14px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:hover slider,
        window.zter-settings-window scale.zter-settings-opacity-scale:focus slider {{
            background-color: {};
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:focus slider {{
            border-color: {};
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled slider {{
            background-color: {};
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale value {{
            color: {};
            min-width: 34px;
        }}
        window.zter-settings-window scale.zter-settings-opacity-scale:disabled value {{
            color: {};
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
        palette.surface.css(),
        palette.border.css(),
        palette.header_background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.border.css(),
        palette.surface.css(),
        palette.muted_foreground.css(),
        palette.muted_foreground.css(),
        palette.muted_foreground.css(),
        palette.background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.muted_foreground.css(),
        palette.muted_foreground.css(),
        palette.muted_foreground.css(),
        palette.selection_foreground.css(),
        palette.foreground.css(),
        palette.foreground.css(),
        palette.background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.muted_foreground.css(),
        palette.muted_foreground.css(),
        palette.muted_foreground.css(),
        palette.selection_foreground.css(),
        palette.foreground.css(),
        palette.background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.header_background.css(),
        palette.muted_foreground.css(),
        palette.tab_close_hover.css(),
        palette.muted_foreground.css(),
        palette.tab_close_hover.css(),
        palette.foreground.css(),
        palette.foreground.css(),
        palette.tab_close_hover.css(),
        palette.foreground.css(),
        palette.muted_foreground.css(),
        palette.foreground.css(),
        palette.foreground.css(),
        palette.foreground.css(),
        palette.header_button_hover.css(),
        palette.border.css(),
        palette.error.css(),
        palette.header_background.css(),
        palette.border.css(),
        palette.foreground.css(),
        palette.tab_hover.css(),
        palette.header_button_hover.css()
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

    #[cfg(test)]
    fn luminance(self) -> f64 {
        let component = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };

        0.2126 * component(self.0) + 0.7152 * component(self.1) + 0.0722 * component(self.2)
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
    fn every_supported_theme_has_sixteen_ansi_colors() {
        for theme in Theme::ALL {
            assert_eq!(theme.palette().ansi.len(), 16);
        }
    }

    #[test]
    fn non_default_theme_css_uses_semantic_palette_roles() {
        let css = application_css_for(Theme::ForestCalm, TerminalPadding::default());
        let palette = Theme::ForestCalm.palette();

        assert!(css.contains(&format!(
            "background-color: {}",
            palette.header_background.css()
        )));
        assert!(css.contains(&format!("background-color: {}", palette.tab_hover.css())));
        assert!(css.contains(&format!(
            "outline: 1px solid {}",
            palette.tab_drop_target.css()
        )));
        assert!(css.contains(&format!("color: {}", palette.foreground.css())));
        assert!(css.contains(&format!("border: 1px solid {}", palette.border.css())));
        assert!(!css.contains("background-color: #303643"));
    }

    #[test]
    fn non_default_theme_uses_a_dark_terminal_background() {
        for theme in Theme::ALL
            .into_iter()
            .filter(|theme| *theme != Theme::OneHalfDark)
        {
            let background = theme.palette().background;

            assert!(background.luminance() < 0.08);
        }
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
    fn context_menus_are_compact_and_use_the_neutral_palette() {
        let css = application_css(TerminalPadding::default());
        let (_, surface_rule) = css
            .split_once("popover.zter-context-menu > contents")
            .unwrap();
        let (surface_rule, _) = surface_rule.split_once('}').unwrap();
        let (_, item_rule) = css.split_once("button.zter-context-menu-item {").unwrap();
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
    fn tabs_keep_a_readable_minimum_width_for_title_and_close_button() {
        let css = application_css(TerminalPadding::default());
        let (_, tab_rule) = css.split_once(".zter-header-tab {").unwrap();
        let (tab_rule, _) = tab_rule.split_once('}').unwrap();

        assert!(tab_rule.contains("min-width: 80px"));
    }

    #[test]
    fn tab_scroll_overlay_controls_use_neutral_edge_fills_without_shadows() {
        let css = application_css(TerminalPadding::default());
        let (_, button_rule) = css.split_once("button.zter-tab-scroll-button {").unwrap();
        let (button_rule, remainder) = button_rule.split_once('}').unwrap();
        let (_, hover_rule) = remainder
            .split_once("button.zter-tab-scroll-button:hover {")
            .unwrap();
        let (hover_rule, _) = hover_rule.split_once('}').unwrap();

        assert!(button_rule.contains("background-color: #303643"));
        assert!(button_rule.contains("min-width: 28px"));
        assert!(button_rule.contains("box-shadow: none"));
        assert!(button_rule.contains("transition: background-color 180ms ease-out"));
        assert!(hover_rule.contains("background-color: #444A55"));
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
