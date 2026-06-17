import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import BranchListRow from "./BranchListRow.svelte";
import type { BranchListItemDto } from "$lib/types";

const branch: BranchListItemDto = {
  name: "feature",
  checked_out: false,
  tracking: "local_remote",
  ahead: null,
  behind: null,
};

describe("BranchListRow", () => {
  afterEach(() => {
    cleanup();
  });

  it("calls onActivate when branch name clicked", async () => {
    const onActivate = vi.fn();
    const { getByRole } = render(BranchListRow, {
      props: {
        branch,
        repoPath: "/tmp/repo",
        onActivate,
      },
    });
    await fireEvent.click(
      getByRole("button", { name: "Activate branch feature" }),
    );
    expect(onActivate).toHaveBeenCalledWith(branch);
  });

  it("renders 'remote' badge for remote_only branches", () => {
    const { queryByText } = render(BranchListRow, {
      props: {
        branch: { ...branch, tracking: "remote_only" },
        repoPath: "/tmp/repo",
      },
    });
    expect(queryByText("remote")).toBeTruthy();
    expect(queryByText("local")).toBeNull();
  });

  it("renders 'local' badge for local_only branches", () => {
    const { queryByText } = render(BranchListRow, {
      props: {
        branch: { ...branch, tracking: "local_only" },
        repoPath: "/tmp/repo",
      },
    });
    expect(queryByText("local")).toBeTruthy();
    expect(queryByText("remote")).toBeNull();
  });

  it("does not render badges for local_remote branches", () => {
    const { queryByText } = render(BranchListRow, {
      props: {
        branch: { ...branch, tracking: "local_remote" },
        repoPath: "/tmp/repo",
      },
    });
    expect(queryByText("remote")).toBeNull();
    expect(queryByText("local")).toBeNull();
  });

  it("renders check icon and local pill for checked-out local_only branch", () => {
    const { container, queryByText } = render(BranchListRow, {
      props: {
        branch: {
          name: "wip",
          checked_out: true,
          tracking: "local_only",
          ahead: null,
          behind: null,
        },
        repoPath: "/tmp/repo",
      },
    });
    expect(queryByText("local")).toBeTruthy();
    expect(
      container.querySelector(".material-symbols-outlined")?.textContent,
    ).toBe("check");
  });
});
