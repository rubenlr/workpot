import { describe, expect, it } from "vitest";
import {
  branchBadgeAriaLabel,
  branchPresenceIcon,
  formatBranchAheadBehind,
  isCheckoutable,
} from "./branchStatus";
import type { BranchListItemDto } from "./types";

function branch(
  partial: Partial<BranchListItemDto> &
    Pick<BranchListItemDto, "name" | "presence">,
): BranchListItemDto {
  return {
    ahead: null,
    behind: null,
    ...partial,
  };
}

describe("branchPresenceIcon", () => {
  it("maps each presence", () => {
    expect(branchPresenceIcon("checkout")).toBe("●");
    expect(branchPresenceIcon("local_only")).toBe("◆");
    expect(branchPresenceIcon("remote_only")).toBe("☁");
    expect(branchPresenceIcon("local_remote")).toBe("⎇");
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
        name: "main",
        presence: "checkout",
        ahead: 2,
        behind: 1,
      }),
    );
    expect(label).toContain("main");
    expect(label).toContain("Checked out");
  });
});

describe("isCheckoutable", () => {
  it("allows local and remote branches", () => {
    expect(isCheckoutable("local_only")).toBe(true);
    expect(isCheckoutable("local_remote")).toBe(true);
    expect(isCheckoutable("remote_only")).toBe(true);
    expect(isCheckoutable("checkout")).toBe(false);
  });
});
