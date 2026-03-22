use color_eyre::eyre::Result;

use super::hotspot::Hotspot;

pub trait HotspotSource {
    fn hotspots(&self, max_commits: usize) -> Result<Vec<Hotspot>>;
}
