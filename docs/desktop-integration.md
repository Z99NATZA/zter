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

The default zter background image is tracked at
`data/wallpapers/zter-wallpaper.png` and embedded in both debug and release
binaries. The user-local installer therefore does not need to copy a separate
image file.

## User-local installation

Install a release build and its desktop files for the current user:

```bash
./scripts/install-user.sh
```

The command installs only these paths:

- `~/.local/bin/zter`
- `~/.local/share/applications/io.github.z99natza.zter.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/io.github.z99natza.zter.svg`

When `XDG_DATA_HOME` is set, the launcher and icon use that data directory
instead of `~/.local/share`.

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
