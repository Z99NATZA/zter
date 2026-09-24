# Settings

## Quick Use

| Task | Development | Release |
| --- | --- | --- |
| [Apply project settings](#apply-project-settings) | `cargo run -- settings apply` | `cargo run --release -- settings apply` |
| [Reload changed background settings](#reload-running-background-settings) | `cargo run -- settings reload` | `zter settings reload` |
| [Full header](#header-modes) | `cargo run -- header full` | `zter header full` |
| [Mini header](#header-modes) | `cargo run -- header mini` | `zter header mini` |
| [Hide header](#header-modes) | `cargo run -- header hide` | `zter header hide` |
| [Show header](#header-modes) | `cargo run -- header show` | `zter header show` |

For the development header commands, you can also run `make header full`,
`make header mini`, `make header hide`, or `make header show`.

Use the settings button for interactive editing; see [Settings Window](#settings-window).
For available keys and defaults, see [Keys](#keys).

## Settings Files

zter keeps a complete default template at
`config/settings.json`. The project settings are tracked by Git and embedded in
the binary, so a clone contains every supported key and an installed binary can
create settings without the repository being present.

Release builds read `$XDG_CONFIG_HOME/zter/settings.json`. Debug builds use the
separate `$XDG_CONFIG_HOME/zter-devel/settings.json`, so development changes do
not affect the installed application. If `XDG_CONFIG_HOME` is missing or empty,
the paths are `$HOME/.config/zter/settings.json` and
`$HOME/.config/zter-devel/settings.json`. The selected file is created from the
embedded template on first run. These per-user files are outside the repository
and are not tracked by the project Git history.

## Settings Window

The settings button beside the window controls opens one compact modal for its
terminal window. It edits the active debug or release profile shared by all
zter windows in that application.

OK atomically saves the complete draft. Font, theme, padding, scrollback,
background image, background image opacity, and window opacity changes then
apply to every current window and tab. The configured font size replaces each
tab's runtime zoom and resets every tab to 100%; tabs can be zoomed
independently again after the save. A shell change applies only to tabs opened
after the save and does not restart current shells. Closing the modal, pressing
Escape, clicking Cancel, or closing the parent terminal discards unsaved edits.
A save or runtime-configuration error is shown in the modal and retains the
draft.

An unboxed radio group selects the Default, Custom, or None background image
mode. Custom can browse local image formats supported by GdkPixbuf and place the
selected path in the draft. Background image opacity and window opacity each
have a checkbox before the label; clicking the label toggles the checkbox.
Both opacity controls start checked. An unchecked opacity control applies the
embedded default and keeps the current slider value in the draft; a checked
opacity control applies the slider value. Opacity sliders show two-decimal
values.

Unchecked checkbox and radio indicators use the settings background and border
tones. Checked indicators, enabled slider highlights, and resting slider thumbs
use the muted theme foreground with the main settings background for indicator
marks. Hovered indicators use the same muted foreground for their borders.
Focused indicators and hovered or focused slider thumbs use theme white; no
colored accent is used. Disabled settings controls use `0.3` opacity, and
disabled slider tracks are gray. The header close control uses the terminal
window's native control style. Numeric decrement and increment controls keep
circular `28px` background boxes inside their fields, with transparent resting
fills and neutral `#444A55` hover fills. Settings controls change state without
transition durations.

## Apply Project Settings

After editing `config/settings.json`, use the appropriate `settings apply`
command in [Quick Use](#quick-use) to copy all project values to the development
or release per-user file. The command validates the project settings before
changing the per-user file.

If the per-user file exists, zter first saves its exact previous contents as
`settings.json.bak` in the same directory, then atomically replaces
`settings.json`. The command can therefore replace malformed per-user settings
that prevent normal startup. If no per-user file exists, it creates one without
creating a backup.

The current binary contains the project settings available when it was built.
With `cargo run`, changing `config/settings.json` causes Cargo to rebuild before
the command applies those values.

## Reload Running Background Settings

After changing the release settings or a referenced local image, ask a running
installed application to reload its background image source, background image
opacity, and window opacity with `settings reload`. Use the development command
in [Quick Use](#quick-use) for the separate development application.

The command prepares the replacement background on a temporary worker thread
and updates every current tab together; tabs opened later share the replacement
texture. This command reloads only background image and opacity settings; the
settings window has the broader live-apply behavior documented above. A
preparation failure warns and keeps the current background. If the matching
application is not running, the command succeeds without opening a window
because the next startup reads the current settings.

## Header Modes

`show` selects `full`. Each header command in [Quick Use](#quick-use) atomically
updates `header_mode` in the active profile's settings file.

A running profile-matched application updates all of its current windows
immediately, and windows opened afterward use the saved mode. Standalone
instances read the saved mode when they next start. On first load, schema 3
`header_visible: true` becomes `full`, and `false` becomes `hidden` without
discarding other valid settings.

## Key Bindings

Keyboard actions are configured in the `key_bindings` object. Each action maps
to an ordered array of bindings, so one action can have multiple shortcuts. An
empty array disables that action, while a missing action uses its embedded
default. Key bindings are edited in `settings.json`; the settings window does
not provide a key-binding editor. Changes apply when the application next
starts.

```json
"key_bindings": {
  "new_tab": [
    { "key": "t", "modifiers": ["control"], "match": "logical" }
  ],
  "copy": [
    { "key": "c", "modifiers": ["control"], "match": "physical" }
  ]
}
```

`key` uses a GDK key name such as `t`, `Page_Up`, `equal`, or `0`.
`modifiers` accepts `control`, `shift`, `alt`, `super`, `meta`, and `hyper`; an
empty or omitted array binds the key without a modifier. `match` accepts
`logical` or `physical` and defaults to `logical` when omitted. Logical matching
uses the character produced by the active keyboard layout. Physical matching
uses the hardware key position resolved from `key`; the default Copy and Paste
bindings therefore continue to work across keyboard layouts. A physical
binding does not match when its key is unavailable on the active display.

The supported actions are `new_tab`, `previous_tab`, `next_tab`, `copy`,
`paste`, `zoom_in`, `zoom_out`, and `zoom_reset`. One key and modifier
combination cannot be assigned to more than one action. `Ctrl+D`, `Ctrl+Z`, and
`Ctrl+Shift+Z` remain reserved for terminal process protection. `Ctrl+C` may be
assigned only to Copy because it also retains terminal interrupt handling.
Mouse-wheel zoom and hyperlink activation are gestures and are not key
bindings.

An invalid key name, modifier, match mode, conflict, reserved binding, or
unknown action uses the complete embedded `key_bindings` defaults without
discarding other valid settings or overwriting the source file. The Copy and
Paste context menu shows the first configured binding for each action; no hint
is shown when that action is disabled.

## Keys

Every settings file contains every supported key.

| Key | Type | Default | Behavior |
| --- | --- | --- | --- |
| `schema_version` | integer | `4` | Selects the settings schema understood by this zter version. |
| `shell` | string or `null` | `null` | Shell executable. `null` or an empty string uses `$SHELL`, then `/bin/sh` if the environment value is missing or empty. |
| `background_image` | string or `null` | `"builtin"` | `"builtin"` selects the default image embedded in zter, another non-empty string selects a local image path, and `null` or an empty string disables the image layer. |
| `header_mode` | `"full"`, `"mini"`, or `"hidden"` | `"full"` | Selects the persistent terminal header mode. `show` is an alias for `full`, and `hide` selects `hidden`. |
| `key_bindings` | object | See [Key Bindings](#key-bindings) | Configures keyboard shortcuts for application actions. |
| `theme` | string | `"one-half-dark"` | Terminal and ANSI color theme. One Half Dark is the supported theme. |
| `font_family` | string | `"Monospace"` | Terminal font family. It must not be empty. |
| `font_size` | number | `12.0` | Font size in points, from `6` through `72`. |
| `padding_top` | integer | `16` | Inner terminal padding above the content in pixels, from `0` through `128`. |
| `padding_right` | integer | `16` | Inner terminal padding to the right of the content in pixels, from `0` through `128`. |
| `padding_bottom` | integer | `16` | Inner terminal padding below the content in pixels, from `0` through `128`. |
| `padding_left` | integer | `16` | Inner terminal padding to the left of the content in pixels, from `0` through `128`. |
| `scrollback_lines` | integer | `10000` | Retained terminal history, from `0` through `1000000` lines. |
| `background_image_opacity` | number | `0.1` | Screen-blended background image opacity, from `0` through `0.6`. It has no visible effect when `background_image` is disabled. |
| `window_opacity` | number | `1.0` | Terminal background opacity over the desktop, from `0.6` through `1.0`. It does not make terminal text, the cursor, tabs, the titlebar, or modals transparent. |

`ZTER_BACKGROUND_IMAGE` overrides the `background_image` key for one process.
It accepts the same `"builtin"` value or a local image path. Setting the
environment variable to an empty value disables the configured image for that
process. `ZTER_WALLPAPER` remains a compatibility alias when
`ZTER_BACKGROUND_IMAGE` is not set. A running application retains its startup
environment override during `settings reload`. The same override remains
effective when the settings window saves a background image value.

## Loading And Failure Handling

During normal startup, zter resolves each supported key independently. A value
with the wrong type, an unsupported value, or a number outside its range uses
the embedded project default for only that key. `null` and an empty string have
the optional behaviors documented above; for other keys they select that key's
default. Unknown keys are ignored. Individual invalid or unknown keys are
handled silently and do not prevent the terminal from opening or discard other
valid values.

zter migrates schema versions `1` and `2` to schema version `3`. The `wallpaper`
key becomes `background_image`, `wallpaper_opacity` becomes
`background_image_opacity`, and schema `1` still replaces `wallpaper_shade`
with the inverse opacity capped at the supported maximum of `0.6`.
`window_opacity` and other missing keys are added from the embedded project
settings. These normalized settings are written atomically only when the source
contains no invalid or unknown values.

Malformed or non-UTF-8 JSON, a non-object top-level value, and an unsupported
schema version use the complete embedded defaults without overwriting the
original file. Read or normalization-write failures also warn and continue with
safe settings. `settings apply` remains the explicit strict path that validates
and replaces the per-user file while retaining its backup.
