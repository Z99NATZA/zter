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
