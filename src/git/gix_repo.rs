use std::fmt;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use gix::{Commit, Id, ObjectId, Repository as Gix, discover};

use super::diff_changes::{DeltaProvider, FileDiffChange, compute_file_diff_changes};
use super::hotspot_aggregation::{
    HotspotDelta, build_hotspots_from_commits, get_hotspot_deltas_for_commit,
};
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

    fn get_hotspot_deltas(&self, commit: &Commit) -> Result<Vec<HotspotDelta>> {
        get_hotspot_deltas_for_commit(self.get_path(), self, commit)
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
}

impl HotspotSource for GixRepository {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>> {
        let commits = self.get_commits()?;
        build_hotspots_from_commits(&commits, self.get_path(), max_commits, |commit| {
            self.get_hotspot_deltas(commit)
        })
    }
}

impl fmt::Debug for GixRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GixRepository at {}", self.path.display())
    }
}
