use std::fs;
use std::path::Path;
use std::process::Command;

use color_eyre::eyre::{Result, eyre};
use glitzer::git::gix_repo::GixRepository;
use glitzer::models::hotspot::Hotspot;
use glitzer::models::hotspot_source::HotspotSource;
use tempfile::TempDir;

fn init_repo() -> Result<TempDir> {
    let temp = TempDir::new()?;

    git(temp.path(), &["init", "--initial-branch=main"])?;
    git(temp.path(), &["config", "user.name", "Test User"])?;
    git(
        temp.path(),
        &["config", "user.email", "test.user@example.com"],
    )?;

    Ok(temp)
}

fn write_file(repo: &Path, relative_path: &str, contents: &str) -> Result<()> {
    let path = repo.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn commit_at(repo: &Path, message: &str, timestamp: &str) -> Result<()> {
    git(repo, &["add", "."])?;
    git_with_env(
        repo,
        &["commit", "-m", message],
        &[
            ("GIT_AUTHOR_DATE", timestamp),
            ("GIT_COMMITTER_DATE", timestamp),
        ],
    )
}

fn git(repo: &Path, args: &[&str]) -> Result<()> {
    git_with_env(repo, args, &[])
}

fn git_with_env(repo: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<()> {
    let mut command = Command::new("git");
    command.current_dir(repo).args(args);

    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(eyre!(
        "git command failed: git {}\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        stdout,
        stderr
    ))
}

fn analyze(repo: &Path) -> Result<Vec<Hotspot>> {
    let source = GixRepository::new(repo)?;
    source.hotspots(300)
}

#[test]
fn root_only_repository_is_analyzed() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/main.rs", "fn main() {}\n")?;
    commit_at(repo.path(), "initial commit", "2025-01-01T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;
    assert!(
        hotspots
            .iter()
            .any(|hotspot| hotspot.location() == "src/main.rs")
    );

    Ok(())
}

#[test]
fn detached_head_repository_is_analyzed() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/one.rs", "pub fn one() {}\n")?;
    commit_at(repo.path(), "add one", "2025-01-01T00:00:00Z")?;

    write_file(repo.path(), "src/two.rs", "pub fn two() {}\n")?;
    commit_at(repo.path(), "add two", "2025-01-02T00:00:00Z")?;

    git(repo.path(), &["checkout", "HEAD~1"])?;

    let hotspots = analyze(repo.path())?;
    assert!(
        hotspots
            .iter()
            .any(|hotspot| hotspot.location() == "src/one.rs")
    );
    assert!(
        !hotspots
            .iter()
            .any(|hotspot| hotspot.location() == "src/two.rs")
    );

    Ok(())
}

#[test]
fn packed_objects_are_read() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/lib.rs", "pub fn version() -> u8 { 1 }\n")?;
    commit_at(repo.path(), "v1", "2025-01-01T00:00:00Z")?;

    write_file(repo.path(), "src/lib.rs", "pub fn version() -> u8 { 2 }\n")?;
    commit_at(repo.path(), "v2", "2025-01-02T00:00:00Z")?;

    write_file(repo.path(), "src/lib.rs", "pub fn version() -> u8 { 3 }\n")?;
    commit_at(repo.path(), "v3", "2025-01-03T00:00:00Z")?;

    git(repo.path(), &["repack", "-ad"])?;
    git(repo.path(), &["prune-packed"])?;

    let pack_dir = repo.path().join(".git/objects/pack");
    let has_pack = fs::read_dir(&pack_dir)?
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.file_name().to_string_lossy().ends_with(".pack"));
    assert!(
        has_pack,
        "expected at least one .pack file in {:?}",
        pack_dir
    );

    let hotspots = analyze(repo.path())?;
    assert!(
        hotspots
            .iter()
            .any(|hotspot| hotspot.location() == "src/lib.rs")
    );

    Ok(())
}
