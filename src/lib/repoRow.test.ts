import { describe, expect, it } from "vitest";
import { dirtyDotClass } from "./repoRow";
import type { RepoDto } from "./types";

function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: `/tmp/${partial.name}`,
    name: partial.name,
    branch: null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: partial.git_state_error ?? null,
  };
}

describe("dirtyDotClass", () => {
  it("uses amber when dirty", () => {
    expect(dirtyDotClass(repo({ name: "a", is_dirty: true }))).toBe(
      "bg-amber-500",
    );
  });

  it("uses emerald when clean", () => {
    expect(dirtyDotClass(repo({ name: "a", is_dirty: false }))).toBe(
      "bg-emerald-500",
    );
  });

  it("uses neutral when git_state_error is set", () => {
    expect(
      dirtyDotClass(
        repo({ name: "a", is_dirty: true, git_state_error: "bare" }),
      ),
    ).toBe("bg-neutral-400");
  });

  it("uses neutral when dirty state is unknown", () => {
    expect(dirtyDotClass(repo({ name: "a", is_dirty: null }))).toBe(
      "bg-neutral-400",
    );
  });
});
