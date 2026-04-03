# Contributing to Glitzer

Thanks for helping improve Glitzer.

## How To Contribute

- Open an issue before starting larger changes so we can align on direction.
- Keep pull requests focused and small when possible.
- Include context in PR descriptions: what problem is being solved and why.
- Add or update tests for behavior changes.
- Run local checks before opening a PR:

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

If `cargo llvm-cov` is not available locally, install it with:

```bash
cargo install cargo-llvm-cov --locked
```

## Reporting Issues

Please include enough detail for someone else to reproduce and investigate quickly.

### Issue Template

Copy this into a new GitHub issue:

```md
## Summary
Short description of the problem or request.

## Type
- [ ] Bug
- [ ] Feature request
- [ ] Documentation
- [ ] Question

## Environment
- OS:
- Rust version (`rustc --version`):
- Glitzer version/commit:

## Steps To Reproduce (for bugs)
1.
2.
3.

## Expected Behavior
What you expected to happen.

## Actual Behavior
What happened instead.

## Additional Context
Logs, screenshots, related issues, or extra details.
```

## Pull Request Checklist

Before submitting a PR:

- [ ] I linked the issue (or explained why no issue was needed).
- [ ] I added or updated tests for behavior changes.
- [ ] I ran `cargo fmt`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info`.
- [ ] I updated relevant documentation.

## CI Workflows

- `.github/workflows/rust.yml` contains the reusable Rust check jobs.
- `.github/workflows/ci.yml` runs the reusable checks for pushes and pull requests on `main`.
- `.github/workflows/release.yml` runs on `v*.*.*` tags, executes the same checks, then gates publish behind the `crates-io-publish` environment approval.
