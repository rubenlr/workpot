import { describe, expect, it } from "vitest";
import { selectionIndexAfterBackgroundOpen } from "./openSelection";
import type { RepoDto } from "./types";

function repo(name: string, path?: string): RepoDto {
  return {
    name,
    path: path ?? `/tmp/${name}`,
    branch: null,
    is_dirty: null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: null,
  };
}

describe("selectionIndexAfterBackgroundOpen", () => {
  const repos = [repo("alpha"), repo("beta"), repo("gamma")];

  it("finds row by path after reload", () => {
    expect(selectionIndexAfterBackgroundOpen(repos, "", "/tmp/beta")).toBe(1);
  });

  it("respects active filter", () => {
    expect(selectionIndexAfterBackgroundOpen(repos, "gam", "/tmp/gamma")).toBe(
      0,
    );
  });

  it("falls back to first row when path missing", () => {
    expect(selectionIndexAfterBackgroundOpen(repos, "", "/tmp/missing")).toBe(
      0,
    );
  });
});
