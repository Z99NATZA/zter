# Settings

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
zter windows in that application. The wallpaper field can browse local image
formats supported by GdkPixbuf and place the selected path in the draft. The
inline Default action selects the bundled wallpaper.

OK atomically saves the complete draft. Font, theme, padding, scrollback,
wallpaper, and opacity changes then apply to every current window and tab. The
configured font size replaces each tab's runtime zoom and resets every tab to
100%; tabs can be zoomed independently again after the save. A shell change
applies only to tabs opened after the save and does not restart current shells.
Closing the modal, pressing Escape, clicking Cancel, or closing the parent
terminal discards unsaved edits. A save or runtime-configuration error is shown
in the modal and retains the draft.

## Apply Project Settings

After editing `config/settings.json`, apply all project values to the per-user
file with:

```bash
cargo run -- settings apply
```

This command uses the debug profile and therefore updates the development
settings. Apply the current project settings to the release namespace with:

```bash
cargo run --release -- settings apply
```

The command validates the project settings before changing the per-user file.
If the per-user file exists, zter first saves its exact previous contents as
`settings.json.bak` in the same directory, then atomically replaces
`settings.json`. The command can therefore replace malformed per-user settings
that prevent normal startup. If no per-user file exists, it creates one without
creating a backup.

The current binary contains the project settings available when it was built.
With `cargo run`, changing `config/settings.json` causes Cargo to rebuild before
the command applies those values.

## Reload Running Wallpaper Settings

After changing the release settings or a referenced local image, ask a running
installed application to reload its wallpaper source and opacity with:

```bash
zter settings reload
```

Use `cargo run -- settings reload` for the separate development application.
The command prepares the replacement image on a temporary worker thread and
updates every current tab together; tabs opened later share the replacement
texture. This command reloads only wallpaper settings; the settings window has
the broader live-apply behavior documented above. A preparation failure warns
and keeps the current wallpaper. If the matching application is not running,
the command succeeds without opening a window because the next startup reads
the current settings.

## Keys

Every settings file contains every supported key.

| Key | Type | Default | Behavior |
| --- | --- | --- | --- |
| `schema_version` | integer | `2` | Selects the settings schema understood by this zter version. |
| `shell` | string or `null` | `null` | Shell executable. `null` or an empty string uses `$SHELL`, then `/bin/sh` if the environment value is missing or empty. |
| `wallpaper` | string or `null` | `"builtin"` | `"builtin"` selects the wallpaper embedded in zter, another non-empty string selects a local image path, and `null` or an empty string disables the wallpaper. |
| `theme` | string | `"one-half-dark"` | Terminal and ANSI color theme. One Half Dark is the supported theme. |
| `font_family` | string | `"Monospace"` | Terminal font family. It must not be empty. |
| `font_size` | number | `12.0` | Font size in points, from `6` through `72`. |
| `padding_top` | integer | `16` | Inner terminal padding above the content in pixels, from `0` through `128`. |
| `padding_right` | integer | `16` | Inner terminal padding to the right of the content in pixels, from `0` through `128`. |
| `padding_bottom` | integer | `16` | Inner terminal padding below the content in pixels, from `0` through `128`. |
| `padding_left` | integer | `16` | Inner terminal padding to the left of the content in pixels, from `0` through `128`. |
| `scrollback_lines` | integer | `10000` | Retained terminal history, from `0` through `1000000` lines. |
| `wallpaper_opacity` | number | `0.15` | Screen-blended wallpaper opacity, from `0` through `0.6`. |

`ZTER_WALLPAPER` overrides the `wallpaper` key for one process. It accepts the
same `"builtin"` value or a local image path. Setting the environment variable
to an empty value disables the configured wallpaper for that process. A running
application retains its startup environment override during `settings reload`.
The same override remains effective when the settings window saves a wallpaper
value.

## Loading And Failure Handling

During normal startup, zter resolves each supported key independently. A value
with the wrong type, an unsupported value, or a number outside its range uses
the embedded project default for only that key. `null` and an empty string have
the optional behaviors documented above; for other keys they select that key's
default. Unknown keys are ignored. Individual invalid or unknown keys are
handled silently and do not prevent the terminal from opening or discard other
valid values.

zter migrates schema version `1` by replacing `wallpaper_shade` with
`wallpaper_opacity`. The migrated opacity is the inverse of the shade, capped at
the supported maximum of `0.6`. Missing keys are added from the embedded
project settings. These normalized settings are written atomically only when
the source contains no invalid or unknown values.

Malformed or non-UTF-8 JSON, a non-object top-level value, and an unsupported
schema version use the complete embedded defaults without overwriting the
original file. Read or normalization-write failures also warn and continue with
safe settings. `settings apply` remains the explicit strict path that validates
and replaces the per-user file while retaining its backup.
