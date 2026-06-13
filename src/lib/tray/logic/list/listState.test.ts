import { describe, expect, it } from "vitest";
import { trayListView } from "./listState";

describe("trayListView", () => {
  it("shows error when list load failed with no cached repos", () => {
    expect(trayListView("db locked", 0, "", 0)).toEqual({
      kind: "error",
      message: "db locked",
    });
  });

  it("shows list when error is set but repos are cached", () => {
    expect(trayListView("git push failed", 5, "", 5)).toEqual({ kind: "list" });
    expect(trayListView("refresh_index not allowed", 3, "wp", 1)).toEqual({
      kind: "list",
    });
  });

  it("shows empty list message", () => {
    expect(trayListView(null, 0, "", 0)).toEqual({ kind: "empty-list" });
  });

  it("shows no-match when filter excludes all rows", () => {
    expect(trayListView(null, 3, "zzz", 0)).toEqual({ kind: "no-match" });
  });

  it("shows list when repos match filter", () => {
    expect(trayListView(null, 3, "wp", 1)).toEqual({ kind: "list" });
    expect(trayListView(null, 3, "", 3)).toEqual({ kind: "list" });
  });

  it("ignores whitespace-only filter for no-match", () => {
    expect(trayListView(null, 3, "   ", 0)).toEqual({ kind: "list" });
  });
});
