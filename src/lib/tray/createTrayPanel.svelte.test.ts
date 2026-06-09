import { afterEach, describe, expect, it, vi, beforeEach } from "vitest";
import type { RepoDto } from "$lib/types";
import { clearGitRefreshWatchdog } from "./gitRefreshWatchdog";
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

vi.mock("./trayPanelEvents", () => ({
  subscribeTrayPanelEvents: (...args: unknown[]) =>
    subscribeTrayPanelEvents(...args),
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
    expect(panel.allTags).toEqual(["work"]);
    expect(panel.listMaxHeightPx).toBe(10 * 44 + 52);
    expect(focus).toHaveBeenCalled();
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
    expect(panel.detailRepo).toEqual(r);
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
      return undefined;
    });

    await panel.removeTagFromRepo("/tmp/a", "work");

    expect(invoke).toHaveBeenCalledWith("remove_tag", {
      repoPath: "/tmp/a",
      tag: "work",
    });
    expect(invoke).toHaveBeenCalledWith("list_repos");
  });
});
