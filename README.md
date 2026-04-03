# ✨ Glitzer ✨

> Make your repositories ✨ shine ✨ , one refactor at a time

Glitzer is a Rust TUI for identifying refactoring hotspots in a Git repository.
It ranks files by change signals (churn, touches, ownership spread, and recency) and explains the ranking with tabbed evidence.

[![Build](https://github.com/JoBuRo/glitzer/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/JoBuRo/glitzer/actions/workflows/rust.yml)

---

## What It Does

- Ranks likely refactoring hotspots from Git history
- Shows selected-hotspot details and why it ranks highly
- Keeps default ranking focused on actionable files present in `HEAD`
- Provides evidence tabs:
  - `Commits`: recent commits and authors touching the file
  - `Co-change`: files frequently changed together
  - `Ownership`: contributor distribution for the file
  - `Notes`: risk and prioritization guidance

---

## Installation

You need [Rust and Cargo](https://www.rust-lang.org/tools/install).

```bash
git clone https://github.com/JoBuRo/glitzer.git
cd glitzer
cargo build --release
```

Install the local release build into your Cargo bin directory:

```bash
cargo install --path . --locked
```

After installation, run it from anywhere with:

```bash
glitzer
```

---

## Usage

Run Glitzer for the current repository:

```bash
./target/release/glitzer
```

Run Glitzer for a specific repository path:

```bash
./target/release/glitzer --repo /path/to/repo
```

### Keyboard Controls

- `j` / `k`: move selected hotspot
- `h` / `l`: switch evidence tabs
- `Ctrl+u` / `Ctrl+d`: scroll evidence up/down
- `q`: quit

---

## Current Scope

Glitzer is currently Git-history-driven. It does not yet include static complexity analysis or configurable time windows in the UI.
Merge history traversal currently follows a first-parent policy for deterministic hotspot attribution on mainline history.
Rename and move continuity is preserved using Git rewrite tracking, and deleted files are excluded from the default hotspot list.

---

## Contributing

Issues and pull requests are welcome.
See the contribution guide in `docs/CONTRIBUTING.md`.

---

## License

Apache-2.0
