import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import BranchBadge from "./BranchBadge.svelte";
import type { BranchListItemDto } from "$lib/types";

function branch(
  partial: Partial<BranchListItemDto> &
    Pick<BranchListItemDto, "name" | "checked_out" | "tracking">,
): BranchListItemDto {
  return { ahead: null, behind: null, ...partial };
}

describe("BranchBadge", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders branch name", () => {
    const { getByText } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "main",
          checked_out: true,
          tracking: "local_remote",
        }),
      },
    });
    expect(getByText("main")).toBeTruthy();
  });

  it("checked_out_uses_aria_label_with_checked_out", () => {
    const { container } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "main",
          checked_out: true,
          tracking: "local_remote",
        }),
      },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.getAttribute("aria-label")).toContain("main");
    expect(span?.getAttribute("aria-label")?.toLowerCase()).toContain(
      "checked out",
    );
  });

  it("non_checked_out_aria_label_differs", () => {
    const { container } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "feat",
          checked_out: false,
          tracking: "local_only",
        }),
      },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.getAttribute("aria-label")).toContain("feat");
    expect(span?.getAttribute("aria-label")?.toLowerCase()).not.toContain(
      "checked out",
    );
  });

  it("sync_suffix_shown_when_ahead_and_behind", () => {
    const { container } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "main",
          checked_out: true,
          tracking: "local_remote",
          ahead: 2,
          behind: 1,
        }),
      },
    });
    expect(container.textContent).toContain("↑");
    expect(container.textContent).toContain("↓");
  });

  it("sync_suffix_omitted_when_no_upstream", () => {
    const { container } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "main",
          checked_out: true,
          tracking: "local_remote",
          ahead: null,
          behind: null,
        }),
      },
    });
    expect(container.textContent).not.toContain("↑");
    expect(container.textContent).not.toContain("↓");
  });

  it("checked_out_applies_blue_styling", () => {
    const { container } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "main",
          checked_out: true,
          tracking: "local_remote",
        }),
      },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.className).toContain("bg-tag-blue-bg");
  });

  it("non_checked_out_applies_neutral_styling", () => {
    const { container } = render(BranchBadge, {
      props: {
        branch: branch({
          name: "feat",
          checked_out: false,
          tracking: "remote_only",
        }),
      },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.className).toContain("bg-card-surface");
  });
});
