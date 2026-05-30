/// Per-repo git state snapshot. All fields are Option to encode:
///   branch=None  → detached HEAD short OID stored as String (or unborn branch)
///   is_dirty=None → bare repo (D-13)
///   ahead=None, behind=None → no upstream configured (D-04)
///   error=Some  → git2 open/query failed (D-09, D-16)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitState {
    pub branch: Option<String>,
    pub is_dirty: Option<bool>,
    pub ahead: Option<i64>,
    pub behind: Option<i64>,
    pub error: Option<String>,
}
