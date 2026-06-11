import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import BranchBadge from "./BranchBadge.svelte";
import type { BranchListItemDto } from "$lib/types";

function branch(
  partial: Partial<BranchListItemDto> &
    Pick<BranchListItemDto, "name" | "presence">,
): BranchListItemDto {
  return { ahead: null, behind: null, ...partial };
}

describe("BranchBadge", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders branch name", () => {
    const { getByText } = render(BranchBadge, {
      props: { branch: branch({ name: "main", presence: "checkout" }) },
    });
    expect(getByText("main")).toBeTruthy();
  });

  it("checkout_presence_uses_aria_label_with_checked_out", () => {
    const { container } = render(BranchBadge, {
      props: { branch: branch({ name: "main", presence: "checkout" }) },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.getAttribute("aria-label")).toContain("main");
    expect(span?.getAttribute("aria-label")?.toLowerCase()).toContain(
      "checked out",
    );
  });

  it("non_checkout_presence_aria_label_differs", () => {
    const { container } = render(BranchBadge, {
      props: { branch: branch({ name: "feat", presence: "local_only" }) },
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
          presence: "checkout",
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
          presence: "checkout",
          ahead: null,
          behind: null,
        }),
      },
    });
    expect(container.textContent).not.toContain("↑");
    expect(container.textContent).not.toContain("↓");
  });

  it("checkout_presence_applies_blue_styling", () => {
    const { container } = render(BranchBadge, {
      props: { branch: branch({ name: "main", presence: "checkout" }) },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.className).toContain("bg-blue");
  });

  it("non_checkout_presence_applies_neutral_styling", () => {
    const { container } = render(BranchBadge, {
      props: { branch: branch({ name: "feat", presence: "remote_only" }) },
    });
    const span = container.querySelector("span[aria-label]");
    expect(span?.className).toContain("bg-neutral");
  });
});
