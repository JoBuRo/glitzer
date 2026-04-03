# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-03

### Added
- Initial public release of `glitzer`, a Rust TUI for identifying Git-history-driven refactoring hotspots.
- Hotspot ranking based on churn, touches, ownership spread, recency, and coupling signals.
- Evidence tabs for hotspot investigation: `Commits`, `Co-change`, `Ownership`, and `Notes`.
- Keyboard-driven navigation and scrolling in hotspot and evidence panes.
- CLI support for selecting a repository path via `--repo`.
- End-to-end and widget snapshot test suites, plus CI checks for format, lint, tests, and coverage.

### Changed
- Adopted `gix` as the repository backend and split Git analysis into focused modules.
- Refocused the UI on ranked hotspots with selected hotspot details and tabbed supporting evidence.

### Fixed
- Preserve nested file paths in diff attribution during recursive tree traversal.
- Preserve rename and move continuity using rewrite tracking and canonical path aliasing.
- Exclude deleted files from default hotspot rankings and prune irrelevant history during filtering.
- Skip submodule gitlink entries in diff line counting to avoid invalid line-diff assumptions.
- Use first-parent merge traversal for deterministic hotspot attribution on mainline history.

### Documentation
- Expanded contributor guidance, issue templates, and agent development workflow notes.
