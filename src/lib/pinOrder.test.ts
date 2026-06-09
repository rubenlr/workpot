import { describe, expect, it } from "vitest";
import { reorderPinned, toPinOrderPayload } from "./pinOrder";
import type { RepoDto } from "./types";

type RepoWithPin = RepoDto & { pin_order?: number | null };

function repo(
  partial: Partial<RepoWithPin> & Pick<RepoDto, "name">,
): RepoWithPin {
  return {
    path: partial.path ?? `/tmp/${partial.name}`,
    name: partial.name,
    alias: partial.alias ?? null,
    branch: partial.branch ?? null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: partial.last_opened_at ?? null,
    git_state_error: null,
    pinned: partial.pinned ?? false,
    pin_order: partial.pin_order ?? null,
    notes: partial.notes ?? null,
    tags: partial.tags ?? [],
    branches: partial.branches ?? [],
  };
}

describe("reorderPinned", () => {
  it("moves first item to end with contiguous pin_order", () => {
    const a = repo({ name: "a", path: "/a" });
    const b = repo({ name: "b", path: "/b" });
    const c = repo({ name: "c", path: "/c" });
    const out = reorderPinned([a, b, c], 0, 2);
    expect(out.map((r) => r.path)).toEqual(["/b", "/c", "/a"]);
    expect(out.map((r) => r.pin_order)).toEqual([0, 1, 2]);
  });

  it("moves last item to front", () => {
    const a = repo({ name: "a", path: "/a" });
    const b = repo({ name: "b", path: "/b" });
    const c = repo({ name: "c", path: "/c" });
    const out = reorderPinned([a, b, c], 2, 0);
    expect(out.map((r) => r.path)).toEqual(["/c", "/a", "/b"]);
    expect(out.map((r) => r.pin_order)).toEqual([0, 1, 2]);
  });

  it("noops when from equals to", () => {
    const a = repo({ name: "a", path: "/a", pin_order: 0 });
    const b = repo({ name: "b", path: "/b", pin_order: 1 });
    const out = reorderPinned([a, b], 1, 1);
    expect(out).toEqual([a, b]);
  });

  it("resequences pin_order after middle move", () => {
    const a = repo({ name: "a", path: "/a" });
    const b = repo({ name: "b", path: "/b" });
    const c = repo({ name: "c", path: "/c" });
    const out = reorderPinned([a, b, c], 0, 1);
    expect(out.map((r) => r.path)).toEqual(["/b", "/a", "/c"]);
    expect(out.every((r, i) => r.pin_order === i)).toBe(true);
  });
});

describe("toPinOrderPayload", () => {
  it("maps paths to contiguous order indices", () => {
    const a = repo({ name: "a", path: "/a" });
    const b = repo({ name: "b", path: "/b" });
    expect(toPinOrderPayload([a, b])).toEqual([
      { path: "/a", order: 0 },
      { path: "/b", order: 1 },
    ]);
  });
});
