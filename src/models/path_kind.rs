#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PathKind {
    Regular,
    Lockfile,
    Generated,
    Vendored,
}

const LOCKFILE_BASENAMES: &[&str] = &[
    "cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "composer.lock",
    "gemfile.lock",
    "poetry.lock",
];

const VENDORED_PREFIXES: &[&str] = &["vendor/", "third_party/", "node_modules/"];
const GENERATED_PREFIXES: &[&str] = &["target/", "dist/", "build/", "coverage/"];

pub fn classify_path_kind(path: &str) -> PathKind {
    let normalized = normalize(path);
    let basename = normalized.rsplit('/').next().unwrap_or(&normalized);

    if LOCKFILE_BASENAMES.contains(&basename) {
        return PathKind::Lockfile;
    }

    if VENDORED_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return PathKind::Vendored;
    }

    if GENERATED_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return PathKind::Generated;
    }

    PathKind::Regular
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lockfile_paths() {
        assert_eq!(classify_path_kind("Cargo.lock"), PathKind::Lockfile);
        assert_eq!(
            classify_path_kind("frontend/package-lock.json"),
            PathKind::Lockfile
        );
    }

    #[test]
    fn classifies_vendor_prefixes() {
        assert_eq!(classify_path_kind("vendor/lib/file.c"), PathKind::Vendored);
        assert_eq!(
            classify_path_kind("node_modules/pkg/index.js"),
            PathKind::Vendored
        );
        assert_eq!(
            classify_path_kind("third_party/tool/main.cc"),
            PathKind::Vendored
        );
    }

    #[test]
    fn classifies_generated_prefixes() {
        assert_eq!(classify_path_kind("target/debug/app"), PathKind::Generated);
        assert_eq!(classify_path_kind("dist/app.js"), PathKind::Generated);
        assert_eq!(classify_path_kind("build/out.txt"), PathKind::Generated);
    }

    #[test]
    fn normalizes_windows_paths() {
        assert_eq!(
            classify_path_kind(r"node_modules\pkg\index.js"),
            PathKind::Vendored
        );
    }

    #[test]
    fn leaves_regular_source_paths_as_regular() {
        assert_eq!(classify_path_kind("src/main.rs"), PathKind::Regular);
    }
}
