# AGENTS.md

This file helps coding agents quickly understand the Glitzer workspace, product goals, and expected development workflow.

## Workspace Overview

- Language: Rust
- App type: Terminal UI (TUI) using `ratatui` and `crossterm`
- Entry point: `src/main.rs`
- Main modules:
  - `src/app/` - TUI composition, view state, widgets, and keyboard interactions
  - `src/glitzer/` - Git data model and analysis logic (commits, diffs, file changes, hotspot signals)

Current active widget set:
- `src/app/widgets/hotspots.rs`
- `src/app/widgets/hotspot_detail.rs`
- `src/app/widgets/evidence.rs`

## Product Direction

Glitzer is a Git-history-driven refactoring assistant.

Primary user promise:
- show where refactoring attention should go
- show the evidence behind that ranking

Current UX shape:
- left: ranked hotspots
- right: selected hotspot details and rationale
- bottom: evidence tabs (`Commits`, `Co-change`, `Ownership`, `Notes`)

Near-term scope:
- keep analysis based on Git history signals (churn, touches, ownership spread, recency, coupling)
- time-window controls may be added later as a UI enhancement

## Engineering Expectations For Agents

- Preserve the current product framing: decision support for refactoring, not generic repo browsing.
- Prefer incremental changes that keep the app runnable after each step.
- Keep UI and analysis responsibilities separate:
  - app/view/widgets for rendering and interactions
  - glitzer module for repository analysis and evidence generation
- Avoid introducing unrelated architecture changes during focused feature work.

## Development Workflow (TDD)

- Use a test-driven workflow for code changes whenever feasible.
- Implement one or more failing tests first to capture the intended behavior/specification.
- Make the minimal code change required to pass those tests.
- Refactor after tests pass while keeping the suite green.
- Prefer small, incremental red-green-refactor cycles over large untested changes.

## UI Test Guidance

- Prefer widget-level render tests in the same file using `#[cfg(test)] mod tests`.
- For `ratatui` widgets, use snapshot-style assertions:
  - create a small fixed `Buffer` with `Buffer::empty(Rect::new(...))`
  - render the widget into that buffer
  - compare the rendered lines to an explicit visual `vec!["..."]` expectation
- Build the suite around user-visible behavior (specification), not private implementation details.
- Cover expected interaction states, not only the happy path:
  - basic render snapshot
  - selection/window movement snapshots (middle and end)
  - empty-state snapshot
  - state clamping/safety behavior (index bounds, `None` selection)
- Prefer assertions on full rendered output for layout/copy stability; add style assertions only when style is the behavior under test.
- Keep test fixtures small and explicit so score/rank/evidence text can be predicted and asserted exactly.
- Keep snapshots deterministic (fixed dimensions, fixed test data, no clock-dependent values unless injected).
- When updating widget layout/copy intentionally, update the expected snapshot strings in the same commit.

## Commit Guidance

- Keep agent-authored changes scoped so they fit in one commit or a small number of commits.
- Prefer short, atomic commits when possible.
- Do not create commits that leave the build or tests broken.
- Before committing, run validation commands and ensure they pass (or clearly document why a failure is expected).

Commit message template:

```text
<Title>
Prior to this change <...>
This change <...>
```

## Validation Commands

After making code changes, run all of the following:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

If `clippy` flags issues, fix them unless there is a strong reason not to.
