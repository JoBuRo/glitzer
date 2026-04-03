pub(crate) type PathAliases = std::collections::HashMap<String, String>;

pub(crate) fn resolve_canonical_path(path: &str, aliases: &PathAliases) -> String {
    let mut current = path.to_string();
    let mut seen = std::collections::HashSet::new();

    while let Some(next) = aliases.get(&current) {
        if !seen.insert(current.clone()) {
            break;
        }
        current = next.clone();
    }

    current
}

pub(crate) fn register_path_alias(aliases: &mut PathAliases, old_path: &str, new_path: &str) {
    let old_canonical = resolve_canonical_path(old_path, aliases);
    let new_canonical = resolve_canonical_path(new_path, aliases);

    if old_canonical != new_canonical {
        aliases.insert(old_canonical, new_canonical);
    }
}

#[cfg(test)]
mod tests {
    use super::{PathAliases, resolve_canonical_path};

    #[test]
    fn resolve_canonical_path_follows_alias_chain() {
        let mut aliases = PathAliases::new();
        aliases.insert("src/a.rs".to_string(), "src/b.rs".to_string());
        aliases.insert("src/b.rs".to_string(), "src/c.rs".to_string());

        assert_eq!(
            resolve_canonical_path("src/a.rs", &aliases),
            "src/c.rs".to_string()
        );
    }
}
