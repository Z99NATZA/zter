# Conventions

## Rust Formatting And Readability

Rustfmt is the formatting authority. Format changed Rust code before finishing
and inspect the resulting diff for unrelated changes.

```bash
cargo fmt
```

Check formatting without changing files:

```bash
cargo fmt --check
```

- When alternatives have equivalent correctness and performance, prefer the
  implementation that is easier for humans to read and maintain.
- Keep naming, structure, control flow, and error handling consistent with the
  existing code. Extend an established pattern instead of introducing a second
  style for the same job.
- Prefer straightforward code over compact or clever expressions unless the
  latter provide a measured performance or correctness benefit.

## UI Styling

- Write the project and product name as lowercase `zter` in UI and prose. Keep
  uppercase forms only where technical conventions require them, such as
  `ZTER_WALLPAPER`.
- Do not add app-owned box or text shadows. Use a border when a visual boundary
  is necessary.
- Add borders only between meaningful content or interaction regions. Do not
  outline every nested surface or use borders as decoration.
- Use neutral One Half Dark colors for routine separation. Reserve red for
  errors that require immediate attention.
