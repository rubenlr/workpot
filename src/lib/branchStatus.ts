import type { BranchListItemDto, BranchPresence } from "./types";

/** Presence glyph (matches CLI/repo sync icon vocabulary). */
export function branchPresenceIcon(presence: BranchPresence): string {
  switch (presence) {
    case "checkout":
      return "●";
    case "local_only":
      return "◆";
    case "remote_only":
      return "☁";
    case "local_remote":
      return "⎇";
  }
}

export function branchPresenceLabel(presence: BranchPresence): string {
  switch (presence) {
    case "checkout":
      return "Checked out";
    case "local_only":
      return "Local only";
    case "remote_only":
      return "Remote only";
    case "local_remote":
      return "Local with remote";
  }
}

/** ↑↓ suffix when upstream is configured (same arrows as `format_git_state`). */
export function formatBranchAheadBehind(
  ahead: number | null,
  behind: number | null,
): string {
  if (ahead == null || behind == null) {
    return "";
  }
  let out = "";
  if (ahead > 0) {
    out += `\u{2191}${ahead}`;
  }
  if (behind > 0) {
    out += `\u{2193}${behind}`;
  }
  return out;
}

export function branchBadgeAriaLabel(branch: BranchListItemDto): string {
  const sync = formatBranchAheadBehind(branch.ahead, branch.behind);
  const syncPart = sync ? `, ${sync}` : "";
  return `${branch.name}, ${branchPresenceLabel(branch.presence)}${syncPart}`;
}

export function branchBadgeTitle(branch: BranchListItemDto): string {
  const sync = formatBranchAheadBehind(branch.ahead, branch.behind);
  return sync
    ? `${branchPresenceLabel(branch.presence)} ${sync}`
    : branchPresenceLabel(branch.presence);
}

export function isCheckoutable(presence: BranchPresence): boolean {
  switch (presence) {
    case "local_only":
    case "local_remote":
    case "remote_only":
      return true;
    case "checkout":
      return false;
  }
}
