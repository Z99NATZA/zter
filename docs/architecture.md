# Architecture

Zter is one GTK4 desktop process. GTK owns the application lifecycle and
window, while VTE owns terminal emulation, the pseudo-terminal, and the child
shell connection.

## Runtime Flow

```text
src/main.rs
  -> load environment configuration in src/config.rs
  -> activate GTK application
      -> build the window and terminal surface in src/ui.rs
          -> spawn the user's shell through VTE
              -> close the window when the shell exits
```

## Ownership

- `src/main.rs` owns startup, failure reporting, and the GTK application.
- `src/config.rs` owns environment-derived configuration and validation.
- `src/ui.rs` owns GTK widgets, VTE behavior, wallpaper composition, and shell
  spawning.

Detailed terminal and wallpaper behavior is documented in
[Terminal runtime](terminal-runtime.md).
