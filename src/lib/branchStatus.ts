import type { BranchListItemDto, BranchTracking } from "./types";

/** Tracking glyph (matches CLI/repo sync icon vocabulary). */
export function branchTrackingIcon(tracking: BranchTracking): string {
  switch (tracking) {
    case "local_only":
      return "◆";
    case "remote_only":
      return "☁";
    case "local_remote":
      return "⎇";
    default: {
      const _exhaustive: never = tracking;
      return _exhaustive;
    }
  }
}

export function branchTrackingLabel(tracking: BranchTracking): string {
  switch (tracking) {
    case "local_only":
      return "Local only";
    case "remote_only":
      return "Remote only";
    case "local_remote":
      return "Local with remote";
    default: {
      const _exhaustive: never = tracking;
      return _exhaustive;
    }
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

export function branchListItemLabel(branch: BranchListItemDto): string {
  const tracking = branchTrackingLabel(branch.tracking);
  if (branch.checked_out) {
    return `Checked out, ${tracking}`;
  }
  return tracking;
}

export function branchBadgeAriaLabel(branch: BranchListItemDto): string {
  const sync = formatBranchAheadBehind(branch.ahead, branch.behind);
  const syncPart = sync ? `, ${sync}` : "";
  return `${branch.name}, ${branchListItemLabel(branch)}${syncPart}`;
}

export function branchBadgeTitle(branch: BranchListItemDto): string {
  const sync = formatBranchAheadBehind(branch.ahead, branch.behind);
  const label = branchListItemLabel(branch);
  return sync ? `${label} ${sync}` : label;
}

export function isCheckoutable(checkedOut: boolean): boolean {
  return !checkedOut;
}
