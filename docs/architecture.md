# Architecture

zter is one GTK4 desktop process. GTK owns the application lifecycle and
window, while VTE owns terminal emulation, the pseudo-terminal, and the child
shell connection.

## Runtime Flow

```text
src/main.rs
  -> load or create user settings in src/settings.rs
  -> combine settings and environment overrides in src/config.rs
  -> activate GTK application
      -> build the window and terminal surface in src/ui.rs
          -> spawn the user's shell through VTE
              -> close the window when the shell exits
```

## Ownership

- `src/main.rs` owns startup, failure reporting, and the GTK application.
- `src/settings.rs` owns the JSON schema, defaults, validation, migration, and
  atomic per-user persistence.
- `src/config.rs` combines settings with environment-derived runtime values and
  validates paths used for startup.
- `src/theme.rs` owns terminal surface and ANSI palette colors.
- `src/ui.rs` owns GTK widgets, VTE behavior, wallpaper composition, and shell
  spawning.

The tracked `config/settings.json` is the complete project template and is
embedded in the binary. The mutable user file is outside the repository.

Detailed behavior is documented in [Settings](settings.md) and
[Terminal runtime](terminal-runtime.md).
