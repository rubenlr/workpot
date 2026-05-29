use std::path::Path;

/// Whether `path` lies under `root`, canonicalizing when the filesystem allows and
/// falling back to lexical prefix match on stored path strings (D-21).
pub(crate) fn path_under_root(path: &Path, root: &Path) -> bool {
    let root_canon = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => root.to_path_buf(),
    };
    match path.canonicalize() {
        Ok(path_canon) => path_canon.starts_with(&root_canon),
        Err(_) => path.starts_with(&root_canon),
    }
}
