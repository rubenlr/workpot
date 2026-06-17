import { describe, expect, it } from "vitest";
import { dirtyDotClass } from "./repoRow";
import type { RepoDto } from "$lib/types";

function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: `/tmp/${partial.name}`,
    name: partial.name,
    alias: partial.alias ?? null,
    branch: null,
    ahead: null,
    behind: null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: partial.git_state_error ?? null,
    pinned: false,
    pin_order: null,
    notes: null,
    tags: [],
    branches: [],
    is_bare: false,
    convert_to: null,
    convert_block_reason: null,
  };
}

describe("dirtyDotClass", () => {
  it("uses amber when dirty", () => {
    expect(dirtyDotClass(repo({ name: "a", is_dirty: true }))).toBe(
      "bg-dirty-amber shadow-[var(--shadow-dot-dirty)]",
    );
  });

  it("uses emerald when clean", () => {
    expect(dirtyDotClass(repo({ name: "a", is_dirty: false }))).toBe(
      "bg-clean-emerald shadow-[var(--shadow-dot-clean)]",
    );
  });

  it("uses neutral when git_state_error is set", () => {
    expect(
      dirtyDotClass(
        repo({ name: "a", is_dirty: true, git_state_error: "bare" }),
      ),
    ).toBe("bg-git-error-neutral shadow-[var(--shadow-dot-error)]");
  });

  it("uses neutral when dirty state is unknown", () => {
    expect(dirtyDotClass(repo({ name: "a", is_dirty: null }))).toBe(
      "bg-git-error-neutral shadow-[var(--shadow-dot-error)]",
    );
  });
});
