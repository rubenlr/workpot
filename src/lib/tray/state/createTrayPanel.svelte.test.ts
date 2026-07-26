import { afterEach, describe, expect, it, vi, beforeEach } from "vitest";
import type { RepoDto } from "$lib/types";
import { clearGitRefreshWatchdog } from "$lib/tray/logic/handlers/gitRefreshWatchdog";
import { createTrayPanel } from "./createTrayPanel.svelte";

const invoke = vi.fn();
const unsubscribe = vi.fn();
const subscribeTrayPanelEvents = vi.fn().mockResolvedValue(unsubscribe);
const focus = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide: vi.fn().mockResolvedValue(undefined) }),
}));

vi.mock("$lib/tray/logic/handlers/trayPanelEvents", () => ({
  subscribeTrayPanelEvents: (...args: unknown[]) =>
    subscribeTrayPanelEvents(...args),
}));

function repo(path: string): RepoDto {
  return {
    path,
    name: path.split("/").pop()!,
    alias: null,
    branch: null,
    ahead: null,
    behind: null,
    is_dirty: null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: null,
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

describe("createTrayPanel", () => {
  beforeEach(() => {
    invoke.mockReset();
    subscribeTrayPanelEvents.mockClear();
    unsubscribe.mockClear();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "list_all_tags") return ["work"];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") return null;
      if (cmd === "get_repo_convert_status") return null;
      return undefined;
    });
  });

  afterEach(() => {
    clearGitRefreshWatchdog();
  });

  it("mount subscribes events, loads data, and focuses filter", async () => {
    const panel = createTrayPanel();
    const input = document.createElement("input");
    input.focus = focus;
    panel.bindFilterInput(input);

    await panel.mount();

    expect(subscribeTrayPanelEvents).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledWith("list_repos");
    expect(invoke).toHaveBeenCalledWith("list_all_tags");
    expect(invoke).toHaveBeenCalledWith("get_tray_config");
    expect(invoke).toHaveBeenCalledWith("get_repo_sync_status");
    expect(panel.allTags).toEqual(["work"]);
    expect(panel.listMaxHeightPx).toBe(10 * 44 + 52);
    expect(focus).toHaveBeenCalled();
  });

  it("mount restores activeSync when backend reports in-flight sync", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "list_all_tags") return [];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") {
        return {
          repo_path: "/tmp/a",
          branch: "main",
          direction: "push",
        };
      }
      return undefined;
    });

    const panel = createTrayPanel();
    await panel.mount();

    expect(panel.activeSync).toEqual({
      repoPath: "/tmp/a",
      branch: "main",
      direction: "push",
    });
  });

  it("destroy unsubscribes panel events", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    panel.destroy();
    expect(unsubscribe).toHaveBeenCalledOnce();
  });

  it("openDetail exposes detail state on panel", () => {
    const panel = createTrayPanel();
    const r = repo("/tmp/detail");
    panel.openDetail(r);
    expect(panel.detailRepo?.path).toBe(r.path);
    expect(panel.detailRepo?.name).toBe(r.name);
    panel.closeDetail();
    expect(panel.detailRepo).toBeNull();
  });

  it("filterQuery delegates to list selection", () => {
    const panel = createTrayPanel();
    panel.filterQuery = "tag:work";
    expect(panel.filterQuery).toBe("tag:work");
  });

  it("removeTagFromRepo invokes remove_tag and refreshes", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    invoke.mockClear();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    await panel.removeTagFromRepo("/tmp/a", "work");

    expect(invoke).toHaveBeenCalledWith("remove_tag", {
      repoPath: "/tmp/a",
      tag: "work",
    });
    expect(invoke).toHaveBeenCalledWith("list_repos");
  });

  it("sync events toggle syncing and reload list", async () => {
    vi.useFakeTimers();
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onSyncStarted();
    expect(panel.syncing).toBe(true);

    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "list_all_tags") return ["work"];
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    handlers.onSyncComplete({
      added: 1,
      removed: 0,
      skipped: 0,
      git_refreshed: 1,
      git_errors: 0,
    });
    await vi.runAllTimersAsync();
    expect(panel.syncing).toBe(false);
    expect(invoke).toHaveBeenCalledWith("list_repos");
    vi.useRealTimers();
  });

  it("sync_complete_sets_syncSuccess_then_clears", async () => {
    vi.useFakeTimers();
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onSyncStarted();
    handlers.onSyncComplete({
      added: 0,
      removed: 0,
      skipped: 0,
      git_refreshed: 0,
      git_errors: 0,
    });

    await vi.advanceTimersByTimeAsync(1000);
    expect(panel.syncing).toBe(false);
    expect(panel.syncSuccess).toBe(true);

    await vi.advanceTimersByTimeAsync(400);
    expect(panel.syncSuccess).toBe(false);
    vi.useRealTimers();
  });

  it("git_refresh_complete_clears_error_and_reloads", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    panel.selectedIndex = 2;

    invoke.mockClear();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a"), repo("/tmp/b")];
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    handlers.onGitRefreshComplete({
      refreshed: 2,
      errors: 0,
      any_dirty: false,
    });
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_repos");
    });

    expect(panel.selectedIndex).toBe(0);
    expect(panel.listError).toBeNull();
    expect(panel.listView).toEqual({ kind: "list" });
  });

  it("git_refresh_failed_sets_list_error", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onGitRefreshFailed("git fetch failed");

    expect(panel.listError).toBe("git fetch failed");
    expect(panel.listView).toEqual({ kind: "list" });
  });

  it("panel-closed resets detail filter and selection", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    panel.openDetail(repo("/tmp/detail"));
    panel.filterQuery = "foo";
    panel.selectedIndex = 3;

    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    handlers.onPanelClosed();

    expect(panel.detailRepo).toBeNull();
    expect(panel.filterQuery).toBe("");
    expect(panel.selectedIndex).toBe(0);
  });

  it("repo-sync-started sets activeSync", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onRepoSyncStarted({
      repo_path: "/tmp/a",
      branch: "main",
      direction: "push",
      error: null,
    });

    expect(panel.activeSync).toEqual({
      repoPath: "/tmp/a",
      branch: "main",
      direction: "push",
    });
  });

  it("repo-sync-failed clears activeSync, sets error, keeps list visible", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onRepoSyncStarted({
      repo_path: "/tmp/a",
      branch: "main",
      direction: "push",
      error: null,
    });
    handlers.onRepoSyncFailed({
      repo_path: "/tmp/a",
      branch: "main",
      direction: "push",
      error: "git push rejected",
    });

    expect(panel.activeSync).toBeNull();
    expect(panel.listError).toBe("git push rejected");
    expect(panel.listView).toEqual({ kind: "list" });
  });

  it("repo-sync-complete clears activeSync and refreshes repos", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    invoke.mockClear();

    handlers.onRepoSyncStarted({
      repo_path: "/tmp/a",
      branch: "main",
      direction: "push",
      error: null,
    });
    await handlers.onRepoSyncComplete({
      repo_path: "/tmp/a",
      branch: "main",
      direction: "push",
      error: null,
    });

    expect(panel.activeSync).toBeNull();
    expect(invoke).toHaveBeenCalledWith("list_repos");
  });

  it("startSync invokes refresh_sync and surfaces permission errors", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "refresh_sync") {
        throw new Error("command refresh_sync not allowed");
      }
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "list_all_tags") return [];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    await panel.startSync();

    expect(invoke).toHaveBeenCalledWith("refresh_sync");
    expect(panel.listError).toBe("Error: command refresh_sync not allowed");
    expect(panel.listView).toEqual({ kind: "list" });
  });

  it("handleSync invoke rejection surfaces error via repo-sync-failed event", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "sync_repo_branch") {
        throw new Error("network unreachable");
      }
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "list_all_tags") return [];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    await panel.handleSync("/tmp/a", "main", "push");

    expect(invoke).toHaveBeenCalledWith("sync_repo_branch", {
      repoPath: "/tmp/a",
      branch: "main",
      direction: "push",
    });
    expect(panel.listError).toBeNull();

    handlers.onRepoSyncFailed({
      repo_path: "/tmp/a",
      branch: "main",
      direction: "push",
      error: "network unreachable",
    });
    expect(panel.listError).toBe("network unreachable");
    expect(panel.listView).toEqual({ kind: "list" });
  });

  it("mount restores activeConvert when backend reports in-flight convert", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "list_all_tags") return [];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") return null;
      if (cmd === "get_repo_convert_status") {
        return { repo_path: "/tmp/a", error: null };
      }
      return undefined;
    });

    const panel = createTrayPanel();
    await panel.mount();

    expect(panel.activeConvert).toEqual({ repoPath: "/tmp/a" });
  });

  it("handlePinReorder invokes set_pin_order and refreshes", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    invoke.mockClear();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    await panel.handlePinReorder([{ path: "/tmp/a", order: 0 }]);

    expect(invoke).toHaveBeenCalledWith("set_pin_order", {
      items: [{ path: "/tmp/a", order: 0 }],
    });
    expect(invoke).toHaveBeenCalledWith("list_repos");
  });

  it("repo-context-action convert invokes convert_repo when convert_to is set", async () => {
    const r = { ...repo("/tmp/a"), convert_to: "bare" as const };
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [r];
      if (cmd === "list_all_tags") return [];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") return null;
      if (cmd === "get_repo_convert_status") return null;
      return undefined;
    });

    const panel = createTrayPanel();
    await panel.mount();
    invoke.mockClear();

    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    await handlers.onRepoContextAction({
      action: "convert",
      repo_path: "/tmp/a",
    });

    expect(invoke).toHaveBeenCalledWith("convert_repo", {
      repoPath: "/tmp/a",
      target: "bare",
    });
  });

  it("repo-context-action convert no-ops when convert_to is null", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    invoke.mockClear();

    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    await handlers.onRepoContextAction({
      action: "convert",
      repo_path: "/tmp/a",
    });

    expect(invoke).not.toHaveBeenCalledWith("convert_repo", expect.anything());
  });

  it("sync_failed clears syncing and sets list error", async () => {
    vi.useFakeTimers();
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onSyncStarted();
    expect(panel.syncing).toBe(true);

    handlers.onSyncFailed("index exploded");
    await vi.runAllTimersAsync();

    expect(panel.syncing).toBe(false);
    expect(panel.listError).toBe("index exploded");
    vi.useRealTimers();
  });

  it("panel-opened triggers git refresh handler", async () => {
    const panel = createTrayPanel();
    const input = document.createElement("input");
    input.focus = focus;
    panel.bindFilterInput(input);
    await panel.mount();
    invoke.mockClear();
    focus.mockClear();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [repo("/tmp/a")];
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    handlers.onPanelOpened();

    expect(invoke).toHaveBeenCalledWith("list_repos");
    expect(focus).toHaveBeenCalled();
  });

  it("repo-context-action invokes expected command", async () => {
    const r = { ...repo("/tmp/a"), pinned: false };
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [r];
      if (cmd === "list_all_tags") return [];
      if (cmd === "get_tray_config") {
        return {
          max_visible_rows: 10,
          max_recent_days: 14,
          min_recent_count: 3,
          max_pinned: 5,
          stale_dirty_days: 30,
        };
      }
      if (cmd === "get_repo_sync_status") return null;
      if (cmd === "get_repo_convert_status") return null;
      return undefined;
    });

    const panel = createTrayPanel();
    await panel.mount();
    invoke.mockClear();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_repos") return [r];
      if (cmd === "get_repo_sync_status") return null;
      return undefined;
    });

    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    await handlers.onRepoContextAction({ action: "pin", repo_path: "/tmp/a" });

    expect(invoke).toHaveBeenCalledWith("set_pin", {
      repoPath: "/tmp/a",
      pinned: true,
    });
    expect(invoke).toHaveBeenCalledWith("list_repos");
  });

  it("repo-convert-started sets activeConvert", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onRepoConvertStarted({ repo_path: "/tmp/a", error: null });

    expect(panel.activeConvert).toEqual({ repoPath: "/tmp/a" });
  });

  it("repo-convert-complete clears activeConvert and refreshes repos", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    invoke.mockClear();

    handlers.onRepoConvertStarted({ repo_path: "/tmp/a", error: null });
    await handlers.onRepoConvertComplete({ repo_path: "/tmp/a", error: null });

    expect(panel.activeConvert).toBeNull();
    expect(invoke).toHaveBeenCalledWith("list_repos");
  });

  it("repo-convert-failed clears activeConvert and surfaces error", async () => {
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onRepoConvertStarted({ repo_path: "/tmp/a", error: null });
    handlers.onRepoConvertFailed({
      repo_path: "/tmp/a",
      error: "convert failed",
    });

    expect(panel.activeConvert).toBeNull();
    expect(panel.listError).toBe("convert failed");
  });
});
