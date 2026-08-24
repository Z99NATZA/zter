# Terminal Runtime

Zter opens one VTE terminal in one GTK application window. VTE starts the
configured shell with the UTF-8 entries from the parent process environment and
the directory from which Zter was launched. The window closes when that shell
exits.

## Shell Selection

`$SHELL` selects the executable. A missing or empty value falls back to
`/bin/sh`. A non-UTF-8 value stops startup with an error. A shell spawn failure
is written to standard error and displayed inside the terminal surface.

## Terminal Surface

The terminal keeps 10,000 lines of scrollback, scrolls to input on a keystroke,
hides the pointer while typing, and recognizes hyperlinks. `Ctrl+Shift+C` copies
selected text and `Ctrl+Shift+V` pastes clipboard text. Without a wallpaper, VTE
paints an opaque dark background.

## Wallpaper

`ZTER_WALLPAPER` accepts a path to an image file. Zter verifies that the path is
a file before opening the application. GTK scales a valid image to cover the
window while preserving its aspect ratio. VTE leaves its default background
transparent and Zter places a fixed translucent dark layer between the image
and terminal content for readability.
