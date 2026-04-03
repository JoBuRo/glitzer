use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use gix::{Commit, Id, ObjectId, Repository as Gix, discover};

use super::diff_changes::{
    DeltaProvider, FileDiffChange, FileDiffChangeMeta, compute_file_diff_change_metadata,
    compute_file_diff_changes, compute_file_diff_changes_filtered,
};
use super::hotspot_aggregation::{
    HotspotDelta, build_hotspots_from_commits, get_hotspot_deltas_for_commit_filtered,
};
use super::path_continuity::{PathAliases, register_path_alias, resolve_canonical_path};
use crate::models::hotspot::Hotspot;
use crate::models::hotspot_source::HotspotSource;

#[derive(Debug, Copy, Clone)]
enum TraversalPolicy {
    FirstParent,
}

const TRAVERSAL_POLICY: TraversalPolicy = TraversalPolicy::FirstParent;

pub struct GixRepository {
    repo: Gix,
    path: PathBuf,
}

impl GixRepository {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let repo = discover(path)?;
        let path = repo
            .workdir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo.path().to_path_buf());
        Ok(Self { repo, path })
    }

    fn head_hash(&self) -> Result<Id<'_>> {
        let mut head = self.repo.head()?;
        let id = head
            .try_peel_to_id()?
            .ok_or_else(|| eyre!("HEAD does not resolve to an object id"))?;
        Ok(id)
    }

    fn selected_parent_id(commit: &Commit<'_>) -> Option<ObjectId> {
        match TRAVERSAL_POLICY {
            TraversalPolicy::FirstParent => commit.parent_ids().next().map(Into::into),
        }
    }

    fn get_commits(&self) -> Result<Vec<Commit<'_>>> {
        let mut commits = Vec::new();
        let mut commit_id_opt: Option<ObjectId> = Some(self.head_hash()?.into());

        while let Some(commit_id) = commit_id_opt {
            let commit = self.repo.find_commit(commit_id)?;
            commit_id_opt = Self::selected_parent_id(&commit);
            commits.push(commit);
        }

        Ok(commits)
    }

    fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    fn tree_for_commit_hash(&self, object_id: ObjectId) -> Result<gix::Tree<'_>> {
        Ok(self.repo.find_commit(object_id)?.tree()?)
    }

    fn get_hotspot_deltas_filtered(
        &self,
        commit: &Commit,
        head_paths: &HashSet<String>,
        aliases: &PathAliases,
    ) -> Result<Vec<HotspotDelta>> {
        get_hotspot_deltas_for_commit_filtered(self.get_path(), self, commit, |change| {
            self.is_change_relevant(change, head_paths, aliases)
        })
    }

    fn head_tree(&self) -> Result<gix::Tree<'_>> {
        Ok(self.repo.head_commit()?.tree()?)
    }

    fn path_exists_in_head(&self, head_tree: &gix::Tree<'_>, location: &str) -> Result<bool> {
        Ok(head_tree
            .lookup_entry_by_path(Path::new(location))?
            .is_some())
    }

    fn collect_head_paths_recursive(
        &self,
        tree: &gix::Tree<'_>,
        prefix: Option<&Path>,
        out: &mut HashSet<String>,
    ) -> Result<()> {
        for entry in tree.iter() {
            let entry = entry?;
            let filename =
                PathBuf::from(String::from_utf8_lossy(entry.filename().as_ref()).into_owned());
            let joined = match prefix {
                Some(base) => base.join(filename),
                None => filename,
            };

            if entry.mode().is_tree() {
                let object = entry.object()?;
                let subtree = object.try_into_tree()?;
                self.collect_head_paths_recursive(&subtree, Some(&joined), out)?;
            } else {
                out.insert(joined.to_string_lossy().to_string());
            }
        }
        Ok(())
    }

    fn head_paths(&self) -> Result<HashSet<String>> {
        let head_tree = self.head_tree()?;
        let mut out = HashSet::new();
        self.collect_head_paths_recursive(&head_tree, None, &mut out)?;
        Ok(out)
    }

    fn build_rewrite_aliases(&self, commits: &[Commit<'_>]) -> Result<PathAliases> {
        let mut aliases = PathAliases::new();

        for commit in commits {
            let new_tree = self.tree_for_commit_hash(commit.id)?;
            let old_tree = match GixRepository::selected_parent_id(commit) {
                Some(parent_hash) => Some(self.tree_for_commit_hash(parent_hash)?),
                None => None,
            };

            let changes =
                compute_file_diff_change_metadata(&self.repo, old_tree.as_ref(), &new_tree)?;
            for change in changes {
                if let Some(previous) = change.previous_location {
                    let old_path = previous.to_string_lossy().to_string();
                    let new_path = change.location.to_string_lossy().to_string();
                    register_path_alias(&mut aliases, &old_path, &new_path);
                }
            }
        }

        Ok(aliases)
    }

    fn is_change_relevant(
        &self,
        change: &FileDiffChangeMeta,
        head_paths: &HashSet<String>,
        aliases: &PathAliases,
    ) -> bool {
        if change.is_tree {
            return false;
        }

        let location = change.location.to_string_lossy().to_string();
        let canonical_location = resolve_canonical_path(&location, aliases);
        if head_paths.contains(&canonical_location) {
            return true;
        }

        if let Some(previous) = &change.previous_location {
            let previous = previous.to_string_lossy().to_string();
            let canonical_previous = resolve_canonical_path(&previous, aliases);
            if head_paths.contains(&canonical_previous) {
                return true;
            }
        }

        false
    }
}

impl DeltaProvider<Commit<'_>> for GixRepository {
    fn delta_changes(&self, commit: &Commit<'_>) -> Result<Vec<FileDiffChange>> {
        let new_tree = self.tree_for_commit_hash(commit.id)?;
        let old_tree = match GixRepository::selected_parent_id(commit) {
            Some(parent_hash) => Some(self.tree_for_commit_hash(parent_hash)?),
            None => None,
        };

        compute_file_diff_changes(&self.repo, old_tree.as_ref(), &new_tree)
    }

    fn delta_changes_filtered<F>(
        &self,
        commit: &Commit<'_>,
        include: F,
    ) -> Result<Vec<FileDiffChange>>
    where
        F: FnMut(&FileDiffChangeMeta) -> bool,
    {
        let new_tree = self.tree_for_commit_hash(commit.id)?;
        let old_tree = match GixRepository::selected_parent_id(commit) {
            Some(parent_hash) => Some(self.tree_for_commit_hash(parent_hash)?),
            None => None,
        };

        compute_file_diff_changes_filtered(&self.repo, old_tree.as_ref(), &new_tree, include)
    }
}

impl HotspotSource for GixRepository {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>> {
        let commits = self.get_commits()?;
        let head_paths = self.head_paths()?;
        let aliases = self.build_rewrite_aliases(&commits)?;
        let hotspots =
            build_hotspots_from_commits(&commits, self.get_path(), max_commits, |commit| {
                self.get_hotspot_deltas_filtered(commit, &head_paths, &aliases)
            })?;

        let head_tree = self.head_tree()?;
        let mut filtered = Vec::with_capacity(hotspots.len());
        for hotspot in hotspots {
            if self.path_exists_in_head(&head_tree, hotspot.location())? {
                filtered.push(hotspot);
            }
        }

        Ok(filtered)
    }
}

impl fmt::Debug for GixRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GixRepository at {}", self.path.display())
    }
}
