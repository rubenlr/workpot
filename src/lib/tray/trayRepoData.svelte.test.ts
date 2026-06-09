import { describe, expect, it, vi, beforeEach } from "vitest";
import type { RepoDto } from "$lib/types";
import { createTrayRepoData } from "./trayRepoData.svelte";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

function repo(path: string): RepoDto {
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
  };
}

describe("createTrayRepoData", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("loadRepos populates repos and clears error", async () => {
    const repos = [repo("/tmp/a"), repo("/tmp/b")];
    invoke.mockResolvedValueOnce(repos);

    const data = createTrayRepoData();
    await data.loadRepos();

    expect(invoke).toHaveBeenCalledWith("list_repos");
    expect(data.repos).toEqual(repos);
    expect(data.error).toBeNull();
  });

  it("loadRepos sets error on invoke failure", async () => {
    invoke.mockRejectedValueOnce(new Error("list failed"));

    const data = createTrayRepoData();
    await data.loadRepos();

    expect(data.error).toBe("Error: list failed");
    expect(data.repos).toEqual([]);
  });

  it("loadAllTags falls back to empty array on failure", async () => {
    invoke.mockRejectedValueOnce(new Error("tags failed"));
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    const data = createTrayRepoData();
    await data.loadAllTags();

    expect(data.allTags).toEqual([]);
    warn.mockRestore();
  });

  it("refresh loads repos and tags then calls onAfterRefresh", async () => {
    const repos = [repo("/tmp/x")];
    invoke
      .mockResolvedValueOnce(repos)
      .mockResolvedValueOnce(["work", "personal"]);
    const onAfterRefresh = vi.fn();

    const data = createTrayRepoData({ onAfterRefresh });
    await data.refresh();

    expect(data.allTags).toEqual(["work", "personal"]);
    expect(onAfterRefresh).toHaveBeenCalledWith(repos);
  });

  it("startBackgroundRefresh sets error when git refresh fails", async () => {
    invoke.mockRejectedValueOnce("refresh boom");

    const data = createTrayRepoData();
    await data.startBackgroundRefresh();

    expect(invoke).toHaveBeenCalledWith("refresh_all_git_state");
    expect(data.error).toBe("refresh boom");
  });

  it("setListError sets error without invoke", () => {
    const data = createTrayRepoData();
    data.setListError("custom error");
    expect(data.error).toBe("custom error");
  });
});
