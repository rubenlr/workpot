import { describe, expect, it } from "vitest";
import { resolveContextAction } from "./trayContextAction";
import type { RepoDto } from "$lib/types";

function repo(overrides: Partial<RepoDto> = {}): RepoDto {
  return {
    path: "/tmp/foo",
    name: "foo",
    alias: null,
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
    ...overrides,
  };
}

describe("resolveContextAction", () => {
  it("toggles pin when repo exists", () => {
    expect(
      resolveContextAction("pin", repo({ pinned: false }), "/tmp/foo"),
    ).toEqual({
      kind: "toggle_pin",
      repoPath: "/tmp/foo",
      pinned: true,
    });
    expect(
      resolveContextAction("pin", repo({ pinned: true }), "/tmp/foo"),
    ).toEqual({
      kind: "toggle_pin",
      repoPath: "/tmp/foo",
      pinned: false,
    });
  });

  it("noop pin when repo missing", () => {
    expect(resolveContextAction("pin", null, "/tmp/foo")).toEqual({
      kind: "noop",
    });
  });

  it("removes sole tag directly", () => {
    expect(
      resolveContextAction("remove_tag", repo({ tags: ["work"] }), "/tmp/foo"),
    ).toEqual({
      kind: "remove_tag",
      repoPath: "/tmp/foo",
      tag: "work",
    });
  });

  it("opens detail for multi-tag remove", () => {
    const r = repo({ tags: ["a", "b"] });
    expect(resolveContextAction("remove_tag", r, "/tmp/foo")).toEqual({
      kind: "open_detail_tag_focus",
      repo: r,
    });
  });

  it("noop remove_tag when repo missing", () => {
    expect(resolveContextAction("remove_tag", null, "/tmp/foo")).toEqual({
      kind: "noop",
    });
  });

  it("opens detail for add_tag", () => {
    const r = repo();
    expect(resolveContextAction("add_tag", r, "/tmp/foo")).toEqual({
      kind: "open_detail_tag_focus",
      repo: r,
    });
  });

  it("noop unknown action", () => {
    expect(resolveContextAction("unknown", repo(), "/tmp/foo")).toEqual({
      kind: "noop",
    });
  });
});
