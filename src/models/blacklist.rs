const DEFAULT_LOCKFILE_BASENAMES: &[&str] = &[
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

const DEFAULT_PREFIXES: &[&str] = &[
    "vendor/",
    "third_party/",
    "node_modules/",
    "target/",
    "dist/",
    "build/",
    "coverage/",
];

#[derive(Debug, Clone)]
pub struct HotspotBlacklist {
    exact_paths: Vec<String>,
    prefixes: Vec<String>,
}

impl HotspotBlacklist {
    pub fn with_additional_rules(rules: &[String]) -> Self {
        let mut exact_paths = DEFAULT_LOCKFILE_BASENAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>();
        let mut prefixes = DEFAULT_PREFIXES
            .iter()
            .map(|prefix| (*prefix).to_string())
            .collect::<Vec<_>>();

        for rule in rules {
            let normalized = normalize(rule);
            if normalized.is_empty() {
                continue;
            }

            if normalized.ends_with('/') {
                prefixes.push(normalized);
            } else {
                exact_paths.push(normalized);
            }
        }

        HotspotBlacklist {
            exact_paths,
            prefixes,
        }
    }

    pub fn is_excluded(&self, path: &str) -> bool {
        let normalized = normalize(path);

        if self.exact_paths.iter().any(|rule| rule == &normalized) {
            return true;
        }

        self.prefixes
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
    }
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::HotspotBlacklist;

    #[test]
    fn defaults_exclude_lockfiles_and_known_prefixes() {
        let blacklist = HotspotBlacklist::with_additional_rules(&[]);

        assert!(blacklist.is_excluded("Cargo.lock"));
        assert!(blacklist.is_excluded("vendor/lib.c"));
        assert!(blacklist.is_excluded("dist/app.js"));
        assert!(!blacklist.is_excluded("src/main.rs"));
    }

    #[test]
    fn additional_rules_append_to_defaults() {
        let blacklist = HotspotBlacklist::with_additional_rules(&[
            "generated/".to_string(),
            "tmp.log".to_string(),
        ]);

        assert!(blacklist.is_excluded("generated/file.txt"));
        assert!(blacklist.is_excluded("tmp.log"));
        assert!(blacklist.is_excluded("Cargo.lock"));
    }

    #[test]
    fn normalizes_windows_paths_and_dot_prefixes() {
        let blacklist = HotspotBlacklist::with_additional_rules(&["custom\\".to_string()]);

        assert!(blacklist.is_excluded("./custom/path.txt"));
    }
}
