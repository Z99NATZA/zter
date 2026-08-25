# Terminal Runtime

zter opens one GTK application window with one terminal tab. Every tab has an
independent VTE terminal and child shell. VTE starts each shell selected by the
user settings and environment with the UTF-8 entries from the parent process
environment and the directory from which zter was launched. A shell exit closes
only its tab; the window closes after the last tab exits.

## Tabs

Tabs share one titlebar row with the window controls. The pinned symbolic `+`
button beside the tab strip and `Ctrl+Shift+T` open a new tab. The close button
and `Ctrl+Shift+W` close the current tab. `Ctrl+PageUp` and `Ctrl+PageDown`
select the previous or next tab, and tabs can be reordered by dragging them.
New tabs use the working directory captured when the zter window started.

Each tab has the same height as the titlebar. Tab titles are ellipsized at the
available width. The new-tab button follows the last tab while the tabs fit.
When the tabs exceed the available titlebar space, the strip scrolls
horizontally with a mouse wheel or trackpad and automatically reveals the
selected tab, while the new-tab button and system window controls remain fixed.
The titlebar content is `36px` high. Its blank area moves the window and
double-clicking that area toggles maximization through GTK's window handle.

A tab initially uses a title such as `bash in zter`, derived from the configured
shell executable. VTE window-title updates from the running shell or terminal
program replace that title. Control characters are converted to spaces before
a title is shown, and the application window follows the active tab's title.

## Shell Selection

The `shell` setting selects the executable when it is a string. A `null` value
uses `$SHELL`; a missing or empty environment value falls back to `/bin/sh`. A
non-UTF-8 `$SHELL` value stops startup with an error. A shell spawn failure is
written to standard error and displayed inside the terminal surface.

## Terminal Surface

The terminal uses the configured font family, font size, scrollback line count,
and theme. It scrolls to input on a keystroke, hides the pointer while typing,
and recognizes hyperlinks. `Ctrl+Shift+C` copies selected text and
`Ctrl+Shift+V` pastes clipboard text. The composition layer paints the opaque
One Half Dark background while VTE remains transparent.

App-owned surfaces do not use shadows. The app window has one outer `1px`
`#3E4451` border and `12px` rounded corners. The lower composition layer is
clipped to the same radius. The terminal content surface uses a top border of
the same color as the only header/content divider. Its top, right, bottom, and
left inner padding are independently configurable from `0px` through `128px`
and default to `0px`. The GTK titlebar's theme border is disabled so it does not
create a second dark line. The wallpaper does not add borders or shrink with
terminal padding. Window-manager or compositor decoration remains system-owned
and may include an outer window shadow beyond the app border.

The unified header and inactive tabs use `#303643`, tab hover uses `#353B48`,
and the active tab uses `#3E4451`. Active state is communicated by this neutral
fill change only; tabs have no app-owned border, outline, or shadow. Header
hover transitions last `180ms`. Native window controls use compact spacing and
do not receive an additional app-owned hover fill.

## Theme Palette

The terminal uses One Half Dark colors. Ordinary surfaces and interaction states
use neutral or blue colors; red is reserved for the normal and bright ANSI red
slots used by terminal programs for error semantics.

| Role       | Normal    | Bright    |
| ---------- | --------- | --------- |
| Black      | `#282C34` | `#5C6370` |
| Red        | `#E06C75` | `#E06C75` |
| Green      | `#98C379` | `#98C379` |
| Yellow     | `#E5C07B` | `#E5C07B` |
| Blue       | `#61AFEF` | `#61AFEF` |
| Magenta    | `#C678DD` | `#C678DD` |
| Cyan       | `#56B6C2` | `#56B6C2` |
| White      | `#DCDFE4` | `#FFFFFF` |

Foreground is `#DCDFE4`, the opaque background is `#282C34`, the cursor is
`#61AFEF`, selection is `#3E4451`, and the header background is `#303643`.

## Wallpaper

The `wallpaper` setting accepts a path to an image file. `ZTER_WALLPAPER`
overrides that value for one process, and an empty override disables the
wallpaper. zter verifies that the selected path is a file before opening the
application. zter centers and scales a valid image to cover the content while
preserving its aspect ratio. The image is drawn over the One Half Dark
background with the Screen blend mode and the configured opacity, so it adds
subtle light and color without replacing the readable theme base. Both VTE
background painting and the terminal widget's GTK CSS background remain
transparent.

Settings paths, defaults, ranges, and failure handling are documented in
[Settings](settings.md).
