use std::path::PathBuf;

use color_eyre::eyre::Result;
use gix::Repository as Gix;
use gix::prelude::TreeDiffChangeExt;

#[derive(Debug, Clone)]
pub(crate) struct FileDiffChange {
    pub(crate) location: PathBuf,
    pub(crate) previous_location: Option<PathBuf>,
    pub(crate) is_tree: bool,
    pub(crate) lines_added: u64,
    pub(crate) lines_removed: u64,
}

pub(crate) trait DeltaProvider<C> {
    fn delta_changes(&self, commit: &C) -> Result<Vec<FileDiffChange>>;
}

fn parse_change_location(
    change: &gix::object::tree::diff::ChangeDetached,
) -> (bool, PathBuf, Option<PathBuf>) {
    match change {
        gix::object::tree::diff::ChangeDetached::Addition {
            entry_mode,
            location,
            ..
        }
        | gix::object::tree::diff::ChangeDetached::Deletion {
            entry_mode,
            location,
            ..
        }
        | gix::object::tree::diff::ChangeDetached::Modification {
            entry_mode,
            location,
            ..
        } => (
            entry_mode.is_tree(),
            PathBuf::from(String::from_utf8_lossy(location.as_ref()).into_owned()),
            None,
        ),
        gix::object::tree::diff::ChangeDetached::Rewrite {
            entry_mode,
            location,
            source_location,
            ..
        } => (
            entry_mode.is_tree(),
            PathBuf::from(String::from_utf8_lossy(location.as_ref()).into_owned()),
            Some(PathBuf::from(
                String::from_utf8_lossy(source_location.as_ref()).into_owned(),
            )),
        ),
    }
}

fn line_counts_to_u64(line_counts: Option<(u32, u32)>) -> (u64, u64) {
    match line_counts {
        Some((insertions, removals)) => (u64::from(insertions), u64::from(removals)),
        None => (0, 0),
    }
}

pub(crate) fn compute_file_diff_changes(
    repo: &Gix,
    old_tree: Option<&gix::Tree<'_>>,
    new_tree: &gix::Tree<'_>,
) -> Result<Vec<FileDiffChange>> {
    let mut diff_opts = gix::diff::Options::default();
    diff_opts
        .track_path()
        .track_rewrites(Some(Default::default()));

    let changes = repo.diff_tree_to_tree(old_tree, Some(new_tree), Some(diff_opts))?;

    let mut resource_cache = repo.diff_resource_cache_for_tree_diff()?;
    let mut changes_for_commit = Vec::new();

    for change in changes {
        let attached = change.attach(repo, repo);

        let (is_tree, location, previous_location) = parse_change_location(&change);

        if is_tree {
            resource_cache.clear_resource_cache_keep_allocation();
            continue;
        }

        let line_counts = attached.diff(&mut resource_cache)?.line_counts()?;
        let (lines_added, lines_removed) =
            line_counts_to_u64(line_counts.map(|counts| (counts.insertions, counts.removals)));

        changes_for_commit.push(FileDiffChange {
            location,
            previous_location,
            is_tree,
            lines_added,
            lines_removed,
        });

        resource_cache.clear_resource_cache_keep_allocation();
    }

    Ok(changes_for_commit)
}

#[cfg(test)]
mod tests {
    use super::line_counts_to_u64;

    #[test]
    fn line_counts_to_u64_maps_none_to_zero_counts() {
        assert_eq!(line_counts_to_u64(None), (0, 0));
    }

    #[test]
    fn line_counts_to_u64_converts_insertions_and_removals() {
        assert_eq!(line_counts_to_u64(Some((7, 3))), (7, 3));
    }
}
