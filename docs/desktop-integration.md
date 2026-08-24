# Desktop Integration

zter uses `io.github.znnn.zter` as its GTK application ID, desktop launcher ID,
and icon name. Keeping these values identical lets the desktop environment match
the running window to its launcher and dock icon.

The original scalable icon is tracked at
`data/icons/hicolor/scalable/apps/io.github.znnn.zter.svg`. It uses the zter
`>z` mark and the neutral One Half Dark palette. The launcher metadata is
tracked at `data/io.github.znnn.zter.desktop`.

## User-local installation

Install a release build and its desktop files for the current user:

```bash
./scripts/install-user.sh
```

The command installs only these paths:

- `~/.local/bin/zter`
- `~/.local/share/applications/io.github.znnn.zter.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/io.github.znnn.zter.svg`

When `XDG_DATA_HOME` is set, the launcher and icon use that data directory
instead of `~/.local/share`.

The binary directory must be present in the graphical session's `PATH`. On a
new Ubuntu user session, `~/.local/bin` is normally added after signing out and
back in.

Remove the same user-local files with:

```bash
./scripts/uninstall-user.sh
```

With the default paths, neither command writes to system directories.
