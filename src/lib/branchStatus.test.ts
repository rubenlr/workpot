import { describe, expect, it } from "vitest";
import {
  branchBadgeAriaLabel,
  branchListItemLabel,
  branchTrackingIcon,
  formatBranchAheadBehind,
  isCheckoutable,
} from "./branchStatus";
import type { BranchListItemDto } from "./types";

function branch(
  partial: Partial<BranchListItemDto> &
    Pick<BranchListItemDto, "name" | "checked_out" | "tracking">,
): BranchListItemDto {
  return {
    ahead: null,
    behind: null,
    hidden: false,
    ...partial,
  };
}

describe("branchTrackingIcon", () => {
  it("maps each tracking variant", () => {
    expect(branchTrackingIcon("local_only")).toBe("◆");
    expect(branchTrackingIcon("remote_only")).toBe("☁");
    expect(branchTrackingIcon("local_remote")).toBe("⎇");
  });
});

describe("branchListItemLabel", () => {
  it("combines checked_out with tracking", () => {
    expect(
      branchListItemLabel(
        branch({
          name: "wip",
          checked_out: true,
          tracking: "local_only",
        }),
      ),
    ).toBe("Checked out, Local only");
    expect(
      branchListItemLabel(
        branch({
          name: "feat",
          checked_out: false,
          tracking: "local_remote",
        }),
      ),
    ).toBe("Local with remote");
  });
});

describe("formatBranchAheadBehind", () => {
  it("omits when upstream missing", () => {
    expect(formatBranchAheadBehind(null, null)).toBe("");
    expect(formatBranchAheadBehind(1, null)).toBe("");
  });

  it("uses unicode arrows like git_display", () => {
    expect(formatBranchAheadBehind(2, 1)).toBe("\u{2191}2\u{2193}1");
    expect(formatBranchAheadBehind(0, 3)).toBe("\u{2193}3");
  });
});

describe("branchBadgeAriaLabel", () => {
  it("includes sync counts when present", () => {
    const label = branchBadgeAriaLabel(
      branch({
        name: "master",
        checked_out: true,
        tracking: "local_remote",
        ahead: 2,
        behind: 1,
      }),
    );
    expect(label).toContain("master");
    expect(label).toContain("Checked out");
  });
});

describe("isCheckoutable", () => {
  it("allows checkout only when not already checked out", () => {
    expect(isCheckoutable(false)).toBe(true);
    expect(isCheckoutable(true)).toBe(false);
  });
});
