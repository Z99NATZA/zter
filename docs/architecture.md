# Architecture

zter is one GTK4 desktop process. GTK owns the application lifecycle, windows,
the unified titlebar tab controls, and the hidden-tab page container, while each
tab has one VTE instance that owns terminal emulation, its pseudo-terminal, and
its child shell connection.

## Runtime Flow

```text
src/main.rs
  -> select the debug or release identity in src/identity.rs
  -> load or create user settings in src/settings.rs
  -> combine settings and environment overrides in src/config.rs
  -> activate GTK application
      -> build the window, titlebar tab strip, and page container in src/ui.rs
          -> create one terminal surface and VTE instance per tab
              -> spawn one user shell per VTE instance
                  -> close the tab when its shell exits
                      -> close the window after the last tab exits
```

## Ownership

- `src/main.rs` owns startup, failure reporting, and the GTK application.
- `src/identity.rs` owns the profile-specific application ID, display name,
  icon name, and settings namespace.
- `src/settings.rs` owns the JSON schema, defaults, validation, migration, and
  atomic per-user persistence.
- `src/config.rs` combines settings with environment-derived runtime values and
  validates paths used for startup.
- `src/theme.rs` owns terminal surface and ANSI palette colors.
- `src/ui.rs` owns GTK widgets, VTE behavior, wallpaper composition, and shell
  spawning.
- `data/` owns the desktop launcher metadata and scalable application icon.
- `scripts/` owns the user-local desktop installation and removal commands.

The tracked `config/settings.json` is the complete project template and is
embedded in the binary. The mutable user file is outside the repository.

Detailed behavior is documented in [Settings](settings.md) and
[Terminal runtime](terminal-runtime.md). Desktop launcher and icon behavior is
documented in [Desktop integration](desktop-integration.md).
