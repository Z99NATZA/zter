# Settings

zter keeps a complete default template at
`config/settings.json`. The project settings are tracked by Git and embedded in
the binary, so a clone contains every supported key and an installed binary can
create settings without the repository being present.

At startup, zter reads `$XDG_CONFIG_HOME/zter/settings.json`. If
`XDG_CONFIG_HOME` is missing or empty, it reads
`$HOME/.config/zter/settings.json`. The file is created from the embedded
template on first run. This per-user file is outside the repository and is not
tracked by the project Git history.

## Apply Project Settings

After editing `config/settings.json`, apply all project values to the per-user
file with:

```bash
cargo run -- settings apply
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
| `schema_version` | integer | `1` | Selects the settings schema understood by this zter version. |
| `shell` | string or `null` | `null` | Shell executable. `null` uses `$SHELL`, then `/bin/sh` if the environment value is missing or empty. |
| `wallpaper` | string or `null` | `null` | Image path behind the terminal. `null` disables the wallpaper. |
| `theme` | string | `"one-half-dark"` | Terminal and ANSI color theme. One Half Dark is the supported theme. |
| `font_family` | string | `"Monospace"` | Terminal font family. It must not be empty. |
| `font_size` | number | `12.0` | Font size in points, from `6` through `72`. |
| `scrollback_lines` | integer | `10000` | Retained terminal history, from `0` through `1000000` lines. |
| `wallpaper_shade` | number | `0.42` | Black readability layer opacity, from `0` through `1`. |

`ZTER_WALLPAPER` overrides the `wallpaper` key for one process. Setting the
environment variable to an empty value disables the configured wallpaper for
that process.

## Loading And Failure Handling

During normal startup, when an older settings file is missing supported keys,
zter adds those keys from the embedded project settings and atomically replaces
the file while retaining existing values. Unknown keys, malformed JSON,
unsupported schema versions, invalid values, and wallpaper paths that are not
files stop startup with an error. Normal startup does not overwrite a malformed
or invalid file; `settings apply` is the explicit recovery path.
