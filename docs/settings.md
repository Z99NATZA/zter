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

## Keys

Every settings file contains every supported key.

| Key | Type | Default | Behavior |
| --- | --- | --- | --- |
| `schema_version` | integer | `2` | Selects the settings schema understood by this zter version. |
| `shell` | string or `null` | `null` | Shell executable. `null` uses `$SHELL`, then `/bin/sh` if the environment value is missing or empty. |
| `wallpaper` | string or `null` | `null` | Image path behind the terminal. `null` disables the wallpaper. |
| `theme` | string | `"one-half-dark"` | Terminal and ANSI color theme. One Half Dark is the supported theme. |
| `font_family` | string | `"Monospace"` | Terminal font family. It must not be empty. |
| `font_size` | number | `12.0` | Font size in points, from `6` through `72`. |
| `padding_top` | integer | `0` | Inner terminal padding above the content in pixels, from `0` through `128`. |
| `padding_right` | integer | `0` | Inner terminal padding to the right of the content in pixels, from `0` through `128`. |
| `padding_bottom` | integer | `0` | Inner terminal padding below the content in pixels, from `0` through `128`. |
| `padding_left` | integer | `0` | Inner terminal padding to the left of the content in pixels, from `0` through `128`. |
| `scrollback_lines` | integer | `10000` | Retained terminal history, from `0` through `1000000` lines. |
| `wallpaper_opacity` | number | `0.10` | Screen-blended wallpaper opacity, from `0` through `0.6`. |

`ZTER_WALLPAPER` overrides the `wallpaper` key for one process. Setting the
environment variable to an empty value disables the configured wallpaper for
that process.

## Loading And Failure Handling

During normal startup, zter migrates schema version `1` by replacing
`wallpaper_shade` with `wallpaper_opacity`. The migrated opacity is the inverse
of the shade, capped at the supported maximum of `0.6`, which retains the
previous image contribution where possible. zter also adds missing supported
keys from the embedded project settings and atomically replaces the file while
retaining existing values.

Each padding field independently and silently uses its default when its value is
not an integer from `0` through `128`. Unknown keys, malformed JSON, unsupported
schema versions, invalid values for other settings, and wallpaper paths that are
not files stop startup with an error. Normal startup does not overwrite
malformed JSON or invalid non-padding values; `settings apply` is the explicit
recovery path.
