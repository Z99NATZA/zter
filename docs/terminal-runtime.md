# Terminal Runtime

zter opens one GTK application window with one terminal tab. Every tab has an
independent VTE terminal and child shell. VTE starts each shell selected by the
user settings and environment with the UTF-8 entries from the parent process
environment and the directory from which zter was launched. A shell exit closes
only its tab; the window closes after the last tab exits.

Starting `zter` without an option activates the profile-matched application. If
that application is already running, the new window belongs to its existing
process. `zter -s` and `zter --standalone` start a separate application instance
and window in the new process, ignoring any existing instance. A settings reload
targets the normal profile-matched application rather than standalone instances.

## Tabs

Tabs share one titlebar row with the window controls. The pinned symbolic `+`
button beside the tab strip and `Ctrl+T` open a new tab, and the close button
closes the current tab. `Ctrl+PageUp` and `Ctrl+PageDown` select the previous or
next tab, and tabs can be reordered by dragging them. New tabs use the working
directory reported by the active tab's shell. If that directory is unavailable
or cannot be represented as UTF-8, they use the working directory captured when
the zter window started. This behavior is the same in normal and standalone
windows.

Closing an idle tab removes it immediately, including when it is the last tab.
If that tab has a foreground process other than its shell, zter instead shows a
modal with `A process is still running. Close this tab?` and the actions Cancel
and Close. Closing the window checks every tab and uses the message `Processes
are still running. Close zter?` when any foreground process is active. Cancel is
the default action, Escape cancels, and only Close confirms. The modal uses a
uniform `12px` corner radius, while Close uses a restrained red accent. A child
shell exit still removes its tab immediately without showing the modal.

Each tab has the same height as the titlebar. Tab titles are ellipsized at the
available width. The new-tab button follows the last tab while the tabs fit.
The expanding drag area after that button retains a minimum width of `40px`.
When the tabs exceed the available titlebar space, the strip scrolls
horizontally with a mouse wheel or trackpad and automatically reveals the
selected tab, while the new-tab button, a following minimum `40px` drag area,
and system window controls remain fixed. The titlebar content is `36px` high.
Its blank area moves the window and double-clicking that area toggles
maximization through GTK's window handle.

A tab initially uses a title such as `bash in zter`, derived from the configured
shell executable. VTE window-title updates from the running shell or terminal
program replace that title. Control characters are converted to spaces before
a title is shown, and the application window follows the active tab's title.
Double-clicking a tab title opens an inline editor. Enter or moving focus
elsewhere saves the title, Escape cancels the edit, and saving an empty title
returns the tab to automatic VTE title updates. A manual title lasts only until
that tab closes and is not overwritten by later VTE title changes.

## Shell Selection

The `shell` setting selects the executable when it is a non-empty string. A
`null` or empty value uses `$SHELL`; a missing or empty environment value falls
back to `/bin/sh`. A non-UTF-8 `$SHELL` value stops startup with an error. A
shell spawn failure is written to standard error and displayed inside the
terminal surface.

## Terminal Surface

The terminal uses the configured font family, font size, scrollback line count,
and theme. `Ctrl+=`, `Ctrl+-`, and Control-modified mouse or touchpad scrolling
change the active tab's font scale in one-point steps from `6` through `72`
points. Runtime zoom does not resize the active tab's PTY grid. The zoom lasts
until that tab closes, does not change other tabs or the settings file, and new
tabs start at the configured font size. Ordinary scrolling retains its terminal
history behavior. The terminal scrolls to input on a keystroke, hides the pointer
while typing, and recognizes hyperlinks.
`Ctrl+C` copies selected text; without a selection, it retains the terminal
interrupt behavior. `Ctrl+V` pastes clipboard text; when the clipboard offers no
text, zter passes the key to the terminal child so its application can handle
non-text content such as images. Secondary-click opens a compact One Half Dark
menu with Copy and Paste actions and right-aligned shortcut hints; Copy is
disabled when there is no selection.
`Ctrl+D` retains its normal shell behavior when no other foreground process is
running. While a foreground process owns the terminal, zter suppresses `Ctrl+D`
to prevent accidentally closing that program. Modified forms such as
`Ctrl+Shift+D` continue to reach the terminal child.
Each tab shows a vertical overlay scrollbar when its retained history exceeds
the visible page. The scrollbar uses that tab's VTE scroll adjustment and does
not participate in viewport measurement, so appearing or disappearing does not
change the terminal grid or reflow text.
Selected cells swap their existing foreground and background colors, so the
highlight adapts to colored terminal output instead of using one fixed color.
The composition layer paints the opaque One Half Dark background while VTE
remains transparent.

Each tab waits for its first positive viewport allocation, applies that terminal
grid, and only then starts its shell. During continuous window resizing, the
wallpaper follows the window while the terminal grid remains at its last applied
size. After the window allocation is stable for `120ms`, zter applies its latest
grid. Font zoom instead applies immediately through VTE's native font scale and
does not enter the deferred window-resize path.

App-owned surfaces do not use shadows. The app window has one outer `1px`
`#3E4451` border and `12px` rounded corners. The lower composition layer is
clipped to the same radius. The terminal content surface uses a top border of
the same color as the only header/content divider. Its top, right, bottom, and
left inner padding are independently configurable from `0px` through `128px`
and default to `16px`. The GTK titlebar's theme border is disabled so it does not
create a second dark line. The wallpaper does not add borders or shrink with
terminal padding. Window-manager or compositor decoration remains system-owned
and may include an outer window shadow beyond the app border.

The unified header and inactive tabs use `#303643`, tab hover uses `#353B48`,
and the active tab uses `#3E4451`. Active state is communicated by this neutral
fill change only. Outside valid drag-destination feedback, tabs have no
app-owned border, outline, or shadow. Header hover transitions last `180ms`.
While a tab is dragged over another tab, the valid destination shows a `1px`
white outline drawn inside its edge until the pointer leaves, the drag is
canceled, or the drop completes. The outline does not alter the tab allocation,
and the source tab does not highlight itself. Native window controls use compact
spacing and do not receive an additional app-owned hover fill. The new-tab
button and the first native window control retain a minimum `40px` draggable
gap while tabs overflow.

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

The `wallpaper` setting defaults to `"builtin"`, which selects the original
zter wallpaper embedded in every debug and release binary. Another non-empty
string selects an image path, while `null` or an empty string disables the
wallpaper. `ZTER_WALLPAPER` overrides that value for one process, and an empty
override disables the wallpaper. A missing or unreadable external image falls
back to the bundled wallpaper; if the bundled image cannot load, zter uses the
solid theme background.

Before presenting the window, zter decodes the selected image, reduces images
larger than the pixels needed to cover the connected display, and applies the
One Half Dark background, Screen blend mode, and configured opacity. The result
is one opaque texture shared by every tab. GTK scales that unchanged texture to
cover each terminal surface while preserving its aspect ratio; interactive
terminal redraws do not decode, resize, or blend the wallpaper again. Both VTE
background painting and the terminal widget's GTK CSS background remain
transparent.

`zter settings reload` rereads wallpaper-related settings in the running
profile-matched application. Image preparation runs on a temporary worker
thread, and the completed texture replaces the shared texture for all tabs on
the GTK main thread. Reloading does not restart terminal children; a failure
keeps the active texture.

Settings paths, defaults, ranges, and failure handling are documented in
[Settings](settings.md).
