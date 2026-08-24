# Agent Work Priority

Use this file as the first document to read before starting a new scoped agent task.

## Active Scope

No implementation scope is currently defined. Define the next concrete scope
here before making repository changes; do not infer follow-up work.

## Required Read Order

1. `docs/agent-work-priority.md`
2. The owning current-behavior documents named by the next scope.

## Required Outcome

No implementation outcome is currently authorized. Preserve the clean Rust
project baseline until a new scoped priority is defined.

## Documentation Rules

- Treat code, configuration, and tests as the source of truth.
- Keep `docs/` limited to current behavior, ownership, boundaries, limits,
  failure handling, and operating commands.
- Update the owning domain document; create a new file only when none exists.
- Lead with the outcome, use only necessary headings, and state each fact once.
- Prefer short paragraphs, compact lists, and tables for repeated mappings.
- Link to authoritative sources instead of copying schemas, configuration, or
  test catalogs. Retain exact contracts and safety-critical limits.
- Do not include status labels, implementation journals, milestones, rollout
  plans, open questions, recommendations, or future work.
- Put concrete failed approaches in `docs/lessons-learned/`, version history in
  `docs/release/`, and active task details in this file. Delete other stale or
  duplicated material.
- Before finishing, verify links and repository paths, search for stale
  plan/status language, and run `git diff --check`.

## Notes

- Keep this reusable template. After completing a priority, reset only
  `Active Scope`, `Required Read Order`, and `Required Outcome`.
- Keep this file short and task-focused.
- `docs/architecture.md` is an architecture index, not the source of detailed
  behavior once that document exists.
- Read `docs/lessons-learned/*` only for the subsystem being changed, when
  debugging a regression, or before reworking behavior with a known failure
  mode.
- alias of agent-work-priority.md = priority | pri
