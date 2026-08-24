use gtk::gdk;
use vte4::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb(u8, u8, u8);

const BACKGROUND: Rgb = Rgb(0x28, 0x2c, 0x34);
const FOREGROUND: Rgb = Rgb(0xdc, 0xdf, 0xe4);
const CURSOR: Rgb = Rgb(0x61, 0xaf, 0xef);
const SELECTION: Rgb = Rgb(0x3e, 0x44, 0x51);

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

pub fn apply_to(terminal: &vte4::Terminal, transparent_background: bool) {
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
    terminal.set_clear_background(!transparent_background);
}

fn rgba(Rgb(red, green, blue): Rgb) -> gdk::RGBA {
    gdk::RGBA::new(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    )
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
}
