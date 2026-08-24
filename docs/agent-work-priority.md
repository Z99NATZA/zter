# Agent Work Priority

Use this file as the first document to read before starting a new scoped agent task.

## Active Scope

No implementation scope is currently defined. Define the next concrete scope
here before making repository changes; do not infer follow-up work.

## Required Read Order

1. `docs/agent-work-priority.md`
2. `docs/conventions.md` when the task can change Rust code.
3. The owning current-behavior documents named by the next scope.

## Required Outcome

No implementation outcome is currently authorized. Preserve the clean Rust
project baseline until a new scoped priority and applicable standalone
authorization are added.

## Authorization And Verification

- Repository changes require the applicable standalone authorization command:
  `ok impl`, `ok refine`, `ok fix`, or `ok update`. Use the clearly agreed scope;
  if it is missing or ambiguous, ask for clarification. Without the applicable
  command, repository work is read-only, and other wording does not grant
  authorization.
- `ok impl`, `ok refine`, and `ok fix` include focused tests and fixes for
  failures caused by the authorized changes.
- Do not run full checks by default. `ok tests` permits, but does not require,
  running any local test suites. `ok ci` likewise permits any local equivalents
  in the repository CI workflow. Neither command authorizes repository changes.
- Reuse valid results. Do not weaken assertions or fix unrelated behavior.
  Report unrelated, pre-existing, skipped, or blocked checks with their reasons.
- No authorization command permits commits, pushes, pull requests, releases, or
  remote actions.
- Supports `+`, e.g., `ok impl + tests + <other>` = `ok impl + ok tests + ok <other>`.

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
- Put version history in `docs/release/` and active task details in this file.
  Delete other stale or duplicated material.
- Before finishing, verify links and repository paths, search for stale
  plan/status language, and run `git diff --check`.

## Notes

- Keep this reusable template. After completing a priority, reset only
  `Active Scope`, `Required Read Order`, and `Required Outcome`.
- Keep this file short and task-focused.
- `docs/architecture.md` is an architecture index, not the source of detailed
  behavior once that document exists.
- CI readiness requires stable build dependencies and check commands. Define CI
  as a concrete active scope before implementing its workflow.
- alias of agent-work-priority.md = priority | pri
