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

fn hotspot_by_location<'a>(hotspots: &'a [Hotspot], location: &str) -> Option<&'a Hotspot> {
    hotspots
        .iter()
        .find(|hotspot| hotspot.location() == location)
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

#[test]
fn merge_history_uses_first_parent_traversal() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "README.md", "base\n")?;
    commit_at(repo.path(), "initial", "2025-01-01T00:00:00Z")?;

    git(repo.path(), &["checkout", "-b", "feature"])?;
    write_file(
        repo.path(),
        "src/feature.rs",
        "pub fn feature() -> u8 {\n    1\n}\n",
    )?;
    commit_at(repo.path(), "feature step 1", "2025-01-02T00:00:00Z")?;

    write_file(
        repo.path(),
        "src/feature.rs",
        "pub fn feature() -> u8 {\n    2\n}\n",
    )?;
    commit_at(repo.path(), "feature step 2", "2025-01-03T00:00:00Z")?;

    git(repo.path(), &["checkout", "main"])?;
    write_file(repo.path(), "src/main.rs", "pub fn mainline() {}\n")?;
    commit_at(repo.path(), "mainline change", "2025-01-04T00:00:00Z")?;

    git(
        repo.path(),
        &["merge", "--no-ff", "feature", "-m", "merge feature"],
    )?;

    let hotspots = analyze(repo.path())?;
    let feature_hotspot = hotspots
        .iter()
        .find(|hotspot| hotspot.location() == "src/feature.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/feature.rs"))?;

    assert_eq!(
        feature_hotspot.touches(),
        1,
        "first-parent traversal should attribute feature file through the merge commit only"
    );

    Ok(())
}

#[test]
fn rename_commit_is_tracked_as_rewrite() -> Result<()> {
    let repo = init_repo()?;

    write_file(
        repo.path(),
        "src/old_name.rs",
        "pub fn value() -> u8 { 1 }\n",
    )?;
    commit_at(repo.path(), "add old name", "2025-01-01T00:00:00Z")?;

    git(repo.path(), &["mv", "src/old_name.rs", "src/new_name.rs"])?;
    commit_at(repo.path(), "rename file", "2025-01-02T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;

    let new = hotspot_by_location(&hotspots, "src/new_name.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/new_name.rs"))?;

    assert!(
        hotspot_by_location(&hotspots, "src/old_name.rs").is_none(),
        "old path should be merged into new path after rename"
    );
    assert_eq!(
        new.touches(),
        2,
        "new path should include the pre-rename touch and the rename rewrite touch"
    );

    Ok(())
}

#[test]
fn move_commit_is_tracked_as_rewrite() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/module.rs", "pub fn module() {}\n")?;
    commit_at(repo.path(), "add module", "2025-01-01T00:00:00Z")?;

    fs::create_dir_all(repo.path().join("src/core"))?;
    git(repo.path(), &["mv", "src/module.rs", "src/core/module.rs"])?;
    commit_at(repo.path(), "move module", "2025-01-02T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;

    let new = hotspot_by_location(&hotspots, "src/core/module.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/core/module.rs"))?;

    assert!(
        hotspot_by_location(&hotspots, "src/module.rs").is_none(),
        "old path should be merged into moved path hotspot"
    );
    assert_eq!(
        new.touches(),
        2,
        "moved path should include the pre-move touch and the move rewrite touch"
    );

    Ok(())
}

#[test]
fn rename_preserves_hotspot_continuity_across_history() -> Result<()> {
    let repo = init_repo()?;

    write_file(
        repo.path(),
        "src/old_continuity.rs",
        "pub fn value() -> u8 {\n    1\n}\n",
    )?;
    commit_at(
        repo.path(),
        "add old continuity file",
        "2025-01-01T00:00:00Z",
    )?;

    write_file(
        repo.path(),
        "src/old_continuity.rs",
        "pub fn value() -> u8 {\n    2\n}\n",
    )?;
    commit_at(
        repo.path(),
        "update old continuity file",
        "2025-01-02T00:00:00Z",
    )?;

    git(
        repo.path(),
        &["mv", "src/old_continuity.rs", "src/new_continuity.rs"],
    )?;
    commit_at(
        repo.path(),
        "rename continuity file",
        "2025-01-03T00:00:00Z",
    )?;

    write_file(
        repo.path(),
        "src/new_continuity.rs",
        "pub fn value() -> u8 {\n    3\n}\n",
    )?;
    commit_at(
        repo.path(),
        "update new continuity file",
        "2025-01-04T00:00:00Z",
    )?;

    let hotspots = analyze(repo.path())?;

    let new = hotspot_by_location(&hotspots, "src/new_continuity.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/new_continuity.rs"))?;

    assert!(
        hotspot_by_location(&hotspots, "src/old_continuity.rs").is_none(),
        "old path should be merged into new path hotspot after rename"
    );
    assert_eq!(
        new.touches(),
        4,
        "touches should include pre-rename edits, rename commit, and post-rename edit"
    );

    Ok(())
}

#[test]
fn rename_keeps_co_change_evidence_on_canonical_path() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/old_signal.rs", "pub fn old_signal() {}\n")?;
    write_file(repo.path(), "src/peer.rs", "pub fn peer() {}\n")?;
    commit_at(repo.path(), "add old and peer", "2025-01-01T00:00:00Z")?;

    git(
        repo.path(),
        &["mv", "src/old_signal.rs", "src/new_signal.rs"],
    )?;
    commit_at(repo.path(), "rename old to new", "2025-01-02T00:00:00Z")?;

    write_file(
        repo.path(),
        "src/new_signal.rs",
        "pub fn old_signal() { println!(\"n\"); }\n",
    )?;
    write_file(
        repo.path(),
        "src/peer.rs",
        "pub fn peer() { println!(\"p\"); }\n",
    )?;
    commit_at(repo.path(), "touch new and peer", "2025-01-03T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;
    let new_signal = hotspot_by_location(&hotspots, "src/new_signal.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/new_signal.rs"))?;

    assert!(
        hotspot_by_location(&hotspots, "src/old_signal.rs").is_none(),
        "old path should not remain as separate hotspot"
    );

    let co_change_lines = new_signal.co_change_evidence_lines();
    assert!(
        co_change_lines
            .iter()
            .any(|line| line.contains("src/peer.rs")),
        "co-change evidence should remain linked to peer file after rename"
    );
    assert!(
        co_change_lines
            .iter()
            .all(|line| !line.contains("src/old_signal.rs")),
        "co-change evidence should not mention the old path"
    );

    Ok(())
}

#[test]
fn chained_renames_collapse_into_final_path() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/a.rs", "pub fn v() -> u8 { 1 }\n")?;
    commit_at(repo.path(), "add a", "2025-01-01T00:00:00Z")?;

    write_file(repo.path(), "src/a.rs", "pub fn v() -> u8 { 2 }\n")?;
    commit_at(repo.path(), "update a", "2025-01-02T00:00:00Z")?;

    git(repo.path(), &["mv", "src/a.rs", "src/b.rs"])?;
    commit_at(repo.path(), "rename a to b", "2025-01-03T00:00:00Z")?;

    write_file(repo.path(), "src/b.rs", "pub fn v() -> u8 { 3 }\n")?;
    commit_at(repo.path(), "update b", "2025-01-04T00:00:00Z")?;

    git(repo.path(), &["mv", "src/b.rs", "src/c.rs"])?;
    commit_at(repo.path(), "rename b to c", "2025-01-05T00:00:00Z")?;

    write_file(repo.path(), "src/c.rs", "pub fn v() -> u8 { 4 }\n")?;
    commit_at(repo.path(), "update c", "2025-01-06T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;
    let c = hotspot_by_location(&hotspots, "src/c.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/c.rs"))?;

    assert!(
        hotspot_by_location(&hotspots, "src/a.rs").is_none(),
        "first path in rename chain should be collapsed"
    );
    assert!(
        hotspot_by_location(&hotspots, "src/b.rs").is_none(),
        "intermediate path in rename chain should be collapsed"
    );
    assert_eq!(
        c.touches(),
        6,
        "final path should accumulate all touches before and after both renames"
    );

    Ok(())
}

#[test]
fn rename_preserves_commit_evidence_from_old_and_new_paths() -> Result<()> {
    let repo = init_repo()?;

    write_file(
        repo.path(),
        "src/old_evidence.rs",
        "pub fn v() -> u8 { 1 }\n",
    )?;
    commit_at(repo.path(), "old evidence add", "2025-01-01T00:00:00Z")?;

    write_file(
        repo.path(),
        "src/old_evidence.rs",
        "pub fn v() -> u8 { 2 }\n",
    )?;
    commit_at(repo.path(), "old evidence update", "2025-01-02T00:00:00Z")?;

    git(
        repo.path(),
        &["mv", "src/old_evidence.rs", "src/new_evidence.rs"],
    )?;
    commit_at(repo.path(), "rename evidence file", "2025-01-03T00:00:00Z")?;

    write_file(
        repo.path(),
        "src/new_evidence.rs",
        "pub fn v() -> u8 { 3 }\n",
    )?;
    commit_at(repo.path(), "new evidence update", "2025-01-04T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;
    let new = hotspot_by_location(&hotspots, "src/new_evidence.rs")
        .ok_or_else(|| eyre!("expected hotspot for src/new_evidence.rs"))?;

    let commit_lines = new.commit_evidence_lines();
    assert!(
        commit_lines
            .iter()
            .any(|line| line.contains("old evidence update")),
        "commit evidence should retain pre-rename updates"
    );
    assert!(
        commit_lines
            .iter()
            .any(|line| line.contains("new evidence update")),
        "commit evidence should include post-rename updates"
    );

    Ok(())
}

#[test]
fn deleted_file_is_excluded_from_default_hotspots() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/dead.rs", "pub fn dead() -> u8 { 1 }\n")?;
    commit_at(repo.path(), "add dead file", "2025-01-01T00:00:00Z")?;

    write_file(repo.path(), "src/dead.rs", "pub fn dead() -> u8 { 2 }\n")?;
    commit_at(repo.path(), "update dead file", "2025-01-02T00:00:00Z")?;

    git(repo.path(), &["rm", "src/dead.rs"])?;
    commit_at(repo.path(), "remove dead file", "2025-01-03T00:00:00Z")?;

    write_file(repo.path(), "src/live.rs", "pub fn live() {}\n")?;
    commit_at(repo.path(), "add live file", "2025-01-04T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;

    assert!(
        hotspot_by_location(&hotspots, "src/dead.rs").is_none(),
        "deleted path should be excluded from default hotspot ranking"
    );
    assert!(
        hotspot_by_location(&hotspots, "src/live.rs").is_some(),
        "active path should still appear in default hotspot ranking"
    );

    Ok(())
}

#[test]
fn deleted_high_churn_file_does_not_outrank_live_files() -> Result<()> {
    let repo = init_repo()?;

    write_file(repo.path(), "src/dead_hot.rs", "pub fn hot() -> u8 { 1 }\n")?;
    commit_at(repo.path(), "add hot dead file", "2025-01-01T00:00:00Z")?;

    write_file(repo.path(), "src/dead_hot.rs", "pub fn hot() -> u8 { 2 }\n")?;
    commit_at(
        repo.path(),
        "update hot dead file 1",
        "2025-01-02T00:00:00Z",
    )?;

    write_file(repo.path(), "src/dead_hot.rs", "pub fn hot() -> u8 { 3 }\n")?;
    commit_at(
        repo.path(),
        "update hot dead file 2",
        "2025-01-03T00:00:00Z",
    )?;

    git(repo.path(), &["rm", "src/dead_hot.rs"])?;
    commit_at(repo.path(), "remove hot dead file", "2025-01-04T00:00:00Z")?;

    write_file(repo.path(), "src/live_low.rs", "pub fn live_low() {}\n")?;
    commit_at(repo.path(), "add live low file", "2025-01-05T00:00:00Z")?;

    let hotspots = analyze(repo.path())?;

    assert!(
        hotspot_by_location(&hotspots, "src/dead_hot.rs").is_none(),
        "high-churn deleted file should not be listed"
    );
    assert_eq!(
        hotspots[0].location(),
        "src/live_low.rs",
        "top visible hotspot should be an active path"
    );

    Ok(())
}
