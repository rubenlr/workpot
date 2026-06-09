import { describe, expect, it } from "vitest";
import { DEFAULT_SECTION_CFG } from "$lib/openSelection";
import { computeBackgroundOpenSelection } from "./trayLaunch";
import type { RepoDto } from "$lib/types";

function repo(name: string, path?: string): RepoDto {
  return {
    name,
    alias: null,
    path: path ?? `/tmp/${name}`,
    branch: null,
    is_dirty: null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: null,
    pinned: false,
    pin_order: null,
    notes: null,
    tags: [],
    branches: [],
  };
}

describe("computeBackgroundOpenSelection", () => {
  const repos = [repo("alpha"), repo("beta"), repo("gamma")];
  const sectionCfg = DEFAULT_SECTION_CFG;

  it("delegates to openSelection path lookup", () => {
    expect(
      computeBackgroundOpenSelection(repos, "", "/tmp/beta", sectionCfg),
    ).toBe(1);
  });

  it("respects active filter when resolving selection", () => {
    expect(
      computeBackgroundOpenSelection(repos, "gam", "/tmp/gamma", sectionCfg),
    ).toBe(0);
  });
});
