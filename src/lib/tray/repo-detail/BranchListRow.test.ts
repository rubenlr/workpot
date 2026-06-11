import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import BranchListRow from "./BranchListRow.svelte";
import type { BranchListItemDto } from "$lib/types";

const branch: BranchListItemDto = {
  name: "feature",
  presence: "local_remote",
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
});
