import { afterEach, describe, expect, it, vi, beforeEach } from "vitest";
import type { RepoDto } from "$lib/types";
import { clearSyncWatchdog } from "$lib/tray/logic/handlers/syncWatchdog";
import { createTrayPanel } from "./createTrayPanel.svelte";

const invoke = vi.fn();
const unsubscribe = vi.fn();
const subscribeTrayPanelEvents = vi.fn().mockResolvedValue(unsubscribe);
const focus = vi.fn();
const hide = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide: (...args: unknown[]) => hide(...args) }),
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
    hide.mockReset();
    hide.mockResolvedValue(undefined);
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
      if (cmd === "get_repo_clone_status") return [];
      return undefined;
    });
  });

  afterEach(() => {
    clearSyncWatchdog();
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

  it("sync_complete_clears_error_and_reloads", async () => {
    vi.useFakeTimers();
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

    handlers.onSyncStarted();
    handlers.onSyncComplete({
      added: 0,
      removed: 0,
      skipped: 0,
      git_refreshed: 2,
      git_errors: 0,
    });
    await vi.runAllTimersAsync();
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("list_repos");
    });

    expect(panel.selectedIndex).toBe(0);
    expect(panel.listError).toBeNull();
    expect(panel.listView).toEqual({ kind: "list" });
    vi.useRealTimers();
  });

  it("sync_complete_bumps_branch_revision", async () => {
    vi.useFakeTimers();
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    expect(panel.branchRevision).toBe(0);

    handlers.onSyncStarted();
    handlers.onSyncComplete({
      added: 0,
      removed: 0,
      skipped: 0,
      git_refreshed: 1,
      git_errors: 0,
    });
    await vi.runAllTimersAsync();

    expect(panel.branchRevision).toBe(1);
    vi.useRealTimers();
  });

  it("sync_failed_sets_list_error", async () => {
    vi.useFakeTimers();
    const panel = createTrayPanel();
    await panel.mount();
    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];

    handlers.onSyncStarted();
    handlers.onSyncFailed("git fetch failed");
    await vi.runAllTimersAsync();

    expect(panel.listError).toBe("git fetch failed");
    expect(panel.listView).toEqual({ kind: "list" });
    vi.useRealTimers();
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

  it("panel-opened focuses filter without list refresh", async () => {
    const panel = createTrayPanel();
    const input = document.createElement("input");
    input.focus = focus;
    panel.bindFilterInput(input);
    await panel.mount();
    invoke.mockClear();
    focus.mockClear();

    const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
    handlers.onPanelOpened();

    expect(invoke).not.toHaveBeenCalledWith("list_repos");
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

  describe("clone wiring", () => {
    function remoteRepo(
      path: string,
      policy: RepoDto["clone_policy"],
    ): RepoDto {
      return { ...repo(path), cloned: false, clone_policy: policy };
    }

    function mockListRepos(repos: RepoDto[]) {
      const base = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (cmd: string, ...rest: unknown[]) =>
        cmd === "list_repos" ? repos : base(cmd, ...rest),
      );
    }

    async function mountedPanel() {
      const panel = createTrayPanel();
      await panel.mount();
      const handlers = subscribeTrayPanelEvents.mock.calls[0][0];
      return { panel, handlers };
    }

    it("mount restores in-flight clones from get_repo_clone_status", async () => {
      const base = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (cmd: string, ...rest: unknown[]) =>
        cmd === "get_repo_clone_status"
          ? ["/code/acme/api", "/code/acme/web"]
          : base(cmd, ...rest),
      );

      const { panel } = await mountedPanel();

      expect(invoke).toHaveBeenCalledWith("get_repo_clone_status");
      expect(panel.cloningPaths).toEqual(["/code/acme/api", "/code/acme/web"]);
    });

    it("repo-clone-started adds the path once and repo-clone-failed removes it", async () => {
      const { panel, handlers } = await mountedPanel();

      handlers.onRepoCloneStarted({ repo_path: "/code/acme/api" });
      handlers.onRepoCloneStarted({ repo_path: "/code/acme/api" });
      expect(panel.cloningPaths).toEqual(["/code/acme/api"]);

      handlers.onRepoCloneStarted({ repo_path: "/code/acme/web" });
      handlers.onRepoCloneFailed({
        repo_path: "/code/acme/api",
        error: "clone boom",
      });

      expect(panel.cloningPaths).toEqual(["/code/acme/web"]);
      expect(panel.listError).toBe("clone boom");
    });

    it("repo-clone-complete removes the path and refreshes the list", async () => {
      const { panel, handlers } = await mountedPanel();
      handlers.onRepoCloneStarted({ repo_path: "/code/acme/api" });
      invoke.mockClear();

      handlers.onRepoCloneComplete({ repo_path: "/code/acme/api" });

      await vi.waitFor(() => expect(invoke).toHaveBeenCalledWith("list_repos"));
      expect(panel.cloningPaths).toEqual([]);
      expect(hide).not.toHaveBeenCalled();
    });

    it("repo-clone-complete with an IDE launch error surfaces it and keeps the panel open", async () => {
      const { panel, handlers } = await mountedPanel();

      handlers.onRepoCloneComplete({
        repo_path: "/code/acme/api",
        opened: true,
        error: "cursor missing",
      });

      await vi.waitFor(() => expect(panel.listError).toBe("cursor missing"));
      expect(hide).not.toHaveBeenCalled();
    });

    it("repo-clone-complete after clone-and-open hides the panel", async () => {
      const { panel, handlers } = await mountedPanel();

      handlers.onRepoCloneComplete({
        repo_path: "/code/acme/api",
        opened: true,
      });

      await vi.waitFor(() => expect(hide).toHaveBeenCalledOnce());
      expect(panel.launchError).toBeNull();
    });

    it("a rejected panel hide after clone-and-open is surfaced as launchError", async () => {
      hide.mockRejectedValue(new Error("window gone"));
      const { panel, handlers } = await mountedPanel();

      handlers.onRepoCloneComplete({
        repo_path: "/code/acme/api",
        opened: true,
      });

      await vi.waitFor(() =>
        expect(panel.launchError).toBe("Error: window gone"),
      );
    });

    it("cloneRepo tracks the path and invokes clone_repo without opening", async () => {
      const { panel } = await mountedPanel();
      let finishClone!: () => void;
      const base = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (cmd: string, ...rest: unknown[]) => {
        if (cmd === "clone_repo") {
          await new Promise<void>((resolve) => {
            finishClone = resolve;
          });
          return undefined;
        }
        return base(cmd, ...rest);
      });

      const pending = panel.cloneRepo(remoteRepo("/code/acme/api", "manual"));

      expect(panel.cloningPaths).toEqual(["/code/acme/api"]);
      expect(invoke).toHaveBeenCalledWith("clone_repo", {
        repoPath: "/code/acme/api",
        openAfter: false,
      });
      finishClone();
      await pending;
      // events, not the invoke result, end the spinner
      expect(panel.cloningPaths).toEqual(["/code/acme/api"]);
    });

    it("cloneRepo drops the spinner and shows the error when the invoke rejects", async () => {
      const { panel } = await mountedPanel();
      const base = invoke.getMockImplementation()!;
      invoke.mockImplementation(async (cmd: string, ...rest: unknown[]) => {
        if (cmd === "clone_repo") throw new Error("no network");
        return base(cmd, ...rest);
      });

      await panel.cloneRepo(remoteRepo("/code/acme/api", "manual"));

      expect(panel.cloningPaths).toEqual([]);
      expect(panel.listError).toBe("Error: no network");
    });

    it("openSelected on a not-cloned on_open repo clones with openAfter=true", async () => {
      mockListRepos([remoteRepo("/code/acme/api", "on_open")]);
      const { panel } = await mountedPanel();
      invoke.mockClear();

      await panel.openSelected(false);

      expect(invoke).toHaveBeenCalledWith("clone_repo", {
        repoPath: "/code/acme/api",
        openAfter: true,
      });
      expect(invoke).not.toHaveBeenCalledWith(
        "open_in_cursor",
        expect.anything(),
      );
      expect(panel.cloningPaths).toEqual(["/code/acme/api"]);
    });

    it("openSelected on a not-cloned manual repo shows a hint and does not clone", async () => {
      mockListRepos([remoteRepo("/code/acme/api", "manual")]);
      const { panel } = await mountedPanel();
      invoke.mockClear();

      await panel.openSelected(false);

      expect(invoke).not.toHaveBeenCalledWith("clone_repo", expect.anything());
      expect(panel.launchError).toContain("not cloned");
    });
  });
});
