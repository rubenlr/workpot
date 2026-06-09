import { describe, expect, it } from "vitest";
import type { SectionConfig } from "$lib/sort";
import type { RepoDto } from "$lib/types";
import { computeBackgroundOpenSelection } from "./trayLaunch";

function repo(name: string): RepoDto {
  return {
    name,
    alias: null,
    path: `/tmp/${name}`,
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

const sectionCfg: SectionConfig = {
  maxRecentDays: 14,
  minRecentCount: 3,
};

describe("computeBackgroundOpenSelection", () => {
  const repos = [repo("alpha"), repo("beta"), repo("gamma")];

  it("returns index of opened repo in flat list", () => {
    expect(
      computeBackgroundOpenSelection(repos, "", "/tmp/beta", sectionCfg),
    ).toBe(1);
  });

  it("respects active filter when resolving selection", () => {
    expect(
      computeBackgroundOpenSelection(repos, "gam", "/tmp/gamma", sectionCfg),
    ).toBe(0);
  });

  it("falls back to first visible row when path is missing", () => {
    expect(
      computeBackgroundOpenSelection(repos, "", "/tmp/missing", sectionCfg),
    ).toBe(0);
  });
});
