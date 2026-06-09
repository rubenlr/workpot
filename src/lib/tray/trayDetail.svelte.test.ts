import { describe, expect, it } from "vitest";
import type { RepoDto } from "$lib/types";
import { createTrayDetail } from "./trayDetail.svelte";

function repo(path: string, overrides: Partial<RepoDto> = {}): RepoDto {
  return {
    path,
    name: path.split("/").pop()!,
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

describe("createTrayDetail", () => {
  it("openDetail and closeDetail toggle detailRepo", () => {
    const detail = createTrayDetail();
    const r = repo("/tmp/foo");

    detail.openDetail(r);
    expect(detail.detailRepo).toEqual(r);

    detail.closeDetail();
    expect(detail.detailRepo).toBeNull();
  });

  it("openDetailWithTagFocus sets focus flag until cleared", () => {
    const detail = createTrayDetail();
    const r = repo("/tmp/foo");

    detail.openDetailWithTagFocus(r);
    expect(detail.detailRepo).toEqual(r);
    expect(detail.focusTagOnDetailOpen).toBe(true);

    detail.clearTagFocusRequest();
    expect(detail.focusTagOnDetailOpen).toBe(false);
  });

  it("resync updates detail repo when path still exists", () => {
    const detail = createTrayDetail();
    const original = repo("/tmp/foo", { branch: "main" });
    detail.openDetail(original);

    const updated = repo("/tmp/foo", { branch: "feature" });
    detail.resync([updated, repo("/tmp/bar")]);

    expect(detail.detailRepo).toEqual(updated);
  });

  it("resync clears detail when repo path disappears", () => {
    const detail = createTrayDetail();
    detail.openDetail(repo("/tmp/gone"));
    detail.resync([repo("/tmp/other")]);
    expect(detail.detailRepo).toBeNull();
  });

  it("resync is noop when detail is closed", () => {
    const detail = createTrayDetail();
    detail.resync([repo("/tmp/foo")]);
    expect(detail.detailRepo).toBeNull();
  });
});
