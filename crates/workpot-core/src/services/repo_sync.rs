use std::fmt;
use std::path::Path;
use std::process::Command;

use crate::AppState;

use crate::services::sync_cmd::build_sync_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncFailure {
    pub summary: String,
    pub full_detail: String,
}

impl fmt::Display for SyncFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.summary.fmt(f)
    }
}

fn sync_failure(summary: impl Into<String>) -> SyncFailure {
    let summary = summary.into();
    SyncFailure {
        full_detail: summary.clone(),
        summary,
    }
}

/// Strip ANSI escape sequences from terminal output.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.next() == Some('[') {
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == '@' || next == '`' {
                        break;
                    }
                }
            }
            continue;
        }
        result.push(c);
    }
    result
}

fn find_line_containing<'a>(text: &'a str, needle: &str) -> Option<&'a str> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && line.contains(needle))
}

fn last_non_empty_line(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .rev()
        .find(|line| !line.is_empty())
}

fn extract_hk_failure(detail: &str) -> Option<String> {
    for line in detail.lines() {
        let trimmed = line.trim();
        if let Some(step) = trimmed.strip_prefix('✗') {
            let step = step.trim();
            if !step.is_empty() {
                return Some(format!("pre-push hook failed: {step}"));
            }
        }
        if trimmed.contains("ERROR") {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn truncate_summary(mut summary: String, max_len: usize) -> String {
    if summary.chars().count() > max_len {
        summary = summary.chars().take(max_len).collect();
        summary.push('…');
    }
    summary
}

/// Build a short user-facing summary and full log detail from git/hook output.
pub fn format_sync_failure(
    program: &str,
    code: &str,
    stdout: &str,
    stderr: &str,
) -> (String, String) {
    let mut parts = Vec::new();
    if !stdout.trim().is_empty() {
        parts.push(stdout.trim());
    }
    if !stderr.trim().is_empty() {
        parts.push(stderr.trim());
    }
    let full_detail = if parts.is_empty() {
        String::new()
    } else {
        parts.join("\n")
    };
    let detail_stripped = strip_ansi(&full_detail);

    let summary =
        if let Some(line) = find_line_containing(&detail_stripped, "failed to push some refs") {
            line.to_string()
        } else if let Some(hk) = extract_hk_failure(&detail_stripped) {
            hk
        } else if let Some(line) = last_non_empty_line(&detail_stripped) {
            line.to_string()
        } else {
            format!("{program} exited with status {code}")
        };

    (truncate_summary(summary, 200), full_detail)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Push,
    Pull,
}

impl SyncDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncDirection::Push => "push",
            SyncDirection::Pull => "pull",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "push" => Ok(SyncDirection::Push),
            "pull" => Ok(SyncDirection::Pull),
            other => Err(format!("invalid sync direction: {other}")),
        }
    }
}

/// Run push or pull for an indexed repo branch, then refresh and persist git state.
pub fn run_repo_sync(
    ctx: &AppState,
    repo_path: &str,
    branch: &str,
    direction: SyncDirection,
) -> Result<(), SyncFailure> {
    let catalog_path = Path::new(repo_path);
    let launch_path = ctx
        .indexed_launch_path(catalog_path)
        .map_err(|e| sync_failure(e.to_string()))?;
    let config = ctx.config().map_err(|e| sync_failure(e.to_string()))?;
    let template = match direction {
        SyncDirection::Push => config.push_cmd.clone(),
        SyncDirection::Pull => config.pull_cmd.clone(),
    };
    let (program, args) =
        build_sync_command(&template, &launch_path, branch).map_err(sync_failure)?;
    let output = Command::new(&program)
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| sync_failure(format!("failed to run {program}: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        let (summary, full_detail) = format_sync_failure(&program, &code, &stdout, &stderr);
        return Err(SyncFailure {
            summary,
            full_detail,
        });
    }
    ctx.with_write_connection(|conn| {
        crate::services::git_state::refresh_and_persist_catalog_entry(
            conn,
            catalog_path,
            &launch_path,
        )
    })
    .map_err(|e| sync_failure(e.to_string()))?;
    Ok(())
}
