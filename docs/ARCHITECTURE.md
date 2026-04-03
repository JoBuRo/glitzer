# Glitzer Architecture

This document describes the current architecture of Glitzer at a high level.

## Overview

Glitzer is a Rust terminal UI (TUI) for identifying and investigating refactoring hotspots from Git history.

Runtime flow:

1. Parse CLI arguments (`--repo`) in `src/main.rs`.
2. Open repository and compute hotspot/evidence data through `src/git`.
3. Render interactive TUI and handle keyboard input through `src/ui`.

## Module Boundaries

### `src/main.rs`

- Program entry point.
- Installs error reporting (`color-eyre`).
- Parses CLI via `clap`.
- Wires `GixRepository` into `App`.

### `src/git`

Git analysis and data extraction layer.

- `gix_repo.rs`: main repository adapter using `gix`.
- `hotspot_aggregation.rs`: hotspot scoring and ranking assembly.
- `diff_changes.rs`: diff-to-file-change extraction.
- `path_continuity.rs`: rename/move continuity and canonical path handling.
- `git_objects.rs`: lower-level object traversal helpers.

Key behavior constraints:

- First-parent traversal for deterministic merge attribution.
- Rewrite tracking to preserve rename continuity.
- Deleted files excluded from default hotspot ranking.
- Submodule gitlinks skipped for line-diff semantics.

### `src/models`

Domain data structures passed between analysis and UI.

- `hotspot.rs`: hotspot model and derived display fields.
- `file_change.rs`: per-file change information.
- `hotspot_source.rs`: source/evidence typing for hotspot data.

### `src/ui`

Presentation and interaction layer built with `ratatui`.

- `app.rs`: app state + key handling + render loop integration.
- `view/main_view.rs`: overall layout composition.
- `widgets/hotspots.rs`: ranked hotspot list.
- `widgets/hotspot_detail.rs`: selected hotspot detail panel.
- `widgets/evidence.rs`: evidence tabs (`Commits`, `Co-change`, `Ownership`, `Notes`).

## Testing Strategy

- Unit and widget tests live next to implementation modules where practical.
- Widget tests use deterministic `ratatui` buffer snapshots.
- End-to-end Git behavior tests live in `tests/e2e_git_history.rs` using temporary repos.

## CI / Release Workflow Architecture

- `.github/workflows/rust.yml`: reusable Rust checks workflow (`workflow_call`).
- `.github/workflows/ci.yml`: push/PR entrypoint that calls reusable checks.
- `.github/workflows/release.yml`: tag (`v*.*.*`) entrypoint that calls reusable checks, then runs publish.
- Publish job is gated by the `crates-io-publish` environment for manual approval.

## Design Principles

- Keep analysis and rendering concerns separate (`git` vs `ui`).
- Prioritize deterministic, user-visible behavior.
- Favor small, test-driven, behavior-complete changes.
