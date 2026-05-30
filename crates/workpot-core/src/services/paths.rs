use std::path::Path;

/// Whether `path` lies under `root`, canonicalizing when the filesystem allows and
/// falling back to lexical prefix match on stored path strings (D-21).
pub(crate) fn path_under_root(path: &Path, root: &Path) -> bool {
    let root_canon = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => root.to_path_buf(),
    };
    match path.canonicalize() {
        Ok(path_canon) => path_starts_with_root(&path_canon, &root_canon),
        Err(_) => path_starts_with_root(path, &root_canon),
    }
}

/// Path prefix with an explicit component boundary so `/tmp/foo-bar` is not under `/tmp/foo`.
fn path_starts_with_root(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn lexical_prefix_does_not_match_sibling_directory_name() {
        let root = PathBuf::from("/tmp/foo");
        let sibling = PathBuf::from("/tmp/foo-bar/repo");
        assert!(!path_starts_with_root(&sibling, &root));
    }

    #[test]
    fn lexical_prefix_matches_child_directory() {
        let root = PathBuf::from("/tmp/foo");
        let child = PathBuf::from("/tmp/foo/repo");
        assert!(path_starts_with_root(&child, &root));
    }

    #[test]
    fn path_under_root_matches_exact_root() {
        let root = PathBuf::from("/tmp/foo");
        assert!(path_under_root(&root, &root));
    }
}
