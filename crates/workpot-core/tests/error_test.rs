#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use workpot_core::WorkpotError;

#[test]
fn workpot_error_display_messages() {
    let cases: Vec<(WorkpotError, &str)> = vec![
        (
            WorkpotError::Config("bad field".into()),
            "config error: bad field",
        ),
        (
            WorkpotError::NotGitRepo(PathBuf::from("/tmp/x")),
            "path is not a git repository: /tmp/x",
        ),
        (
            WorkpotError::AlreadyRegistered("/tmp/r".into()),
            "repository already registered: /tmp/r",
        ),
        (
            WorkpotError::NotFound("/tmp/r".into()),
            "repository not found: /tmp/r",
        ),
        (
            WorkpotError::InvalidPath("not a dir".into()),
            "invalid path: not a dir",
        ),
        (
            WorkpotError::GitUnavailable(PathBuf::from("/tmp/bare")),
            "git unavailable for path: /tmp/bare",
        ),
        (
            WorkpotError::LimitsExceeded("too many".into()),
            "config limits exceeded: too many",
        ),
        (
            WorkpotError::InvalidInput("empty tag".into()),
            "invalid input: empty tag",
        ),
        (
            WorkpotError::PinCapExceeded { max: 5 },
            "pin cap exceeded: max 5 pinned repos",
        ),
        (
            WorkpotError::WatchRootNotFound("/tmp/root".into()),
            "watch root not found: /tmp/root",
        ),
        (
            WorkpotError::WatchRootAlreadyExists("/tmp/root".into()),
            "watch root already exists: /tmp/root",
        ),
        (
            WorkpotError::IndexCapExceeded {
                projected: 42,
                max: 40,
            },
            "index cap exceeded: projected 42 repos (max 40)",
        ),
        (
            WorkpotError::ConversionPreflight("dirty tree".into()),
            "conversion preflight failed: dirty tree",
        ),
        (
            WorkpotError::ConversionFailed("worktree exists".into()),
            "conversion failed: worktree exists",
        ),
        (
            WorkpotError::PathsUnavailable,
            "could not resolve platform config/data directories",
        ),
    ];

    for (err, expected) in cases {
        assert_eq!(err.to_string(), expected, "Display for {err:?}");
    }
}
