# Desktop Integration

Release builds use `io.github.z99natza.zter` as the GTK application ID, desktop
launcher ID, and icon name. Debug builds use `io.github.z99natza.zter.Devel` for the
same three values. Keeping each set identical lets the desktop environment
match both running windows to distinct dock icons while allowing the builds to
run at the same time.

The original scalable icon is tracked at
`data/icons/hicolor/scalable/apps/io.github.z99natza.zter.svg`. It uses the zter
`>z` mark and the neutral One Half Dark palette. The launcher metadata is
tracked at `data/io.github.z99natza.zter.desktop`.

The development icon keeps the same `>z` mark on a light `#dee2e4` background
and is tracked at
`data/icons/hicolor/scalable/apps/io.github.z99natza.zter.Devel.svg`. Its desktop
metadata is `data/io.github.z99natza.zter.Devel.desktop`.

Both launchers expose the standard `new-window` desktop action. The release
action runs `zter --standalone`, and the development action runs
`zter-devel --standalone`, so desktop environments can explicitly request a
new process and window instead of activating the profile-matched instance.

The default zter background image is tracked at
`data/wallpapers/zter-wallpaper.png` and embedded in both debug and release
binaries. The user-local installer therefore does not need to copy a separate
image file.

## User-local installation

Install a release build and its desktop files for the current user:

```bash
./scripts/install-user.sh
```

The command installs these application paths:

- `~/.local/bin/zter`
- `~/.local/share/applications/io.github.z99natza.zter.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/io.github.z99natza.zter.svg`

The installer also checks fontconfig for `JetBrainsMono Nerd Font` and
`Noto Sans Thai UI`, the default terminal font families. Missing families are
downloaded over HTTPS into `~/.local/share/fonts/zter`, along with their OFL
licenses. JetBrainsMono includes regular, bold, italic, and bold italic faces;
Noto Sans Thai UI uses its variable font. Existing families are reused without
network access. The installer requires `fc-list`; when fonts are missing it
also requires `curl`, `fc-scan`, and `fc-cache` (the `fontconfig` package on
Ubuntu supplies these font tools).

Downloads use pinned versions from [Nerd Fonts](https://github.com/ryanoasis/nerd-fonts/tree/v3.4.0/patched-fonts/JetBrainsMono)
and [Google Fonts](https://github.com/google/fonts/tree/47d54ca1ebb14fd864695f1d39dc734f39017c33/ofl/notosansthaiui).
All downloaded fonts are checked for the expected family before installation,
then the font cache is refreshed. Download or validation failures stop release
installation before replacing the binary. Installation preserves user settings;
`settings apply` explicitly copies the defaults embedded at build time.

When `XDG_DATA_HOME` is set, the launcher, icon, and fonts use that data directory
instead of `~/.local/share`. Uninstall keeps fonts because other applications
may use them.

Run `sh tests/install-fonts.sh` to check real downloads, offline reuse, missing
fallback repair, and download failures with isolated temporary font directories.
This integration check requires network access.

After changing the desktop files, the install and uninstall commands refresh
the desktop database when `update-desktop-database` is available. They also
refresh the user-local hicolor icon cache with `gtk4-update-icon-cache`, or
`gtk-update-icon-cache` as a fallback, so a running desktop shell can promptly
observe icon changes. Both commands also remove launcher and icon files that
used the previous `io.github.znnn.zter` identity.

Installation does not close running terminals. If an installed zter process is
still using the replaced binary, the installer prints its process ID and asks
the user to close all installed zter windows before reopening the launcher.
Until then, the unique GTK application activates that previous process and its
in-memory code.

The binary directory must be present in the graphical session's `PATH`. On a
new Ubuntu user session, `~/.local/bin` is normally added after signing out and
back in.

Remove the same user-local files with:

```bash
./scripts/uninstall-user.sh
```

With the default paths, neither command writes to system directories.

## Development integration

Install the development desktop metadata and icon for the current user once:

```bash
./scripts/install-dev-user.sh
```

The command installs these paths:

- `~/.local/bin/zter-devel`, as a symlink to the repository development runner
- `~/.local/share/applications/io.github.z99natza.zter.Devel.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/io.github.z99natza.zter.Devel.svg`

The desktop entry has `NoDisplay=true`: it supplies GNOME with the identity and
icon for `cargo run` without adding a development launcher to Applications.
The `zter-devel` runner resolves the repository through its symlink and invokes
`cargo run`, so it also builds and starts the current debug code from any
working directory. The installed release remains unchanged. `cargo run
--release` uses the release identity and can therefore activate an already
running installed release process.

Debug and release builds also use separate settings paths as documented in
[Settings](settings.md).

Remove only the development metadata and icon with:

```bash
./scripts/uninstall-dev-user.sh
```

The development install and uninstall commands also remove metadata and icon
files that used the previous `io.github.znnn.zter.Devel` identity.
