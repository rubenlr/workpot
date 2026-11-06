import { describe, expect, it, vi, beforeEach } from "vitest";
import { DEFAULT_SECTION_CFG } from "$lib/tray/logic/list/openSelection";
import type { RepoDto } from "$lib/types";
import { createTrayLaunch } from "./trayLaunch.svelte";

const invoke = vi.fn();
const hide = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide }),
}));

function repo(path: string): RepoDto {
  return {
    path,
    name: path.split("/").pop()!,
    alias: null,
    branch: "main",
    ahead: null,
    behind: null,
    is_dirty: false,
    parent_dir: "",
    last_opened_at: 1,
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

function createLaunch(
  overrides: {
    selected?: RepoDto | undefined;
    repos?: RepoDto[];
    filterQuery?: string;
    cloneAndOpen?: (repo: RepoDto) => Promise<void>;
  } = {},
) {
  const repos = overrides.repos ?? [repo("/tmp/a"), repo("/tmp/b")];
  const refresh = vi.fn().mockResolvedValue(undefined);
  const setSelectedIndex = vi.fn();
  return {
    launch: createTrayLaunch({
      getSelectedRepo: () => overrides.selected ?? repos[0],
      getFilterQuery: () => overrides.filterQuery ?? "",
      getSectionCfg: () => DEFAULT_SECTION_CFG,
      getRepos: () => repos,
      refresh,
      setSelectedIndex,
      cloneAndOpen: overrides.cloneAndOpen,
    }),
    refresh,
    setSelectedIndex,
    repos,
  };
}

describe("createTrayLaunch", () => {
  beforeEach(() => {
    invoke.mockReset();
    hide.mockClear();
    invoke.mockResolvedValue(undefined);
  });

  it("openSelected foreground invokes open and hides panel", async () => {
    const { launch } = createLaunch({ selected: repo("/tmp/a") });
    await launch.openSelected(false);

    expect(invoke).toHaveBeenCalledWith("open_in_cursor", {
      path: "/tmp/a",
      background: false,
    });
    expect(hide).toHaveBeenCalledOnce();
    expect(launch.launchError).toBeNull();
  });

  it("openSelected background refreshes and updates selection without hiding", async () => {
    const repos = [repo("/tmp/a"), repo("/tmp/b")];
    const { launch, refresh, setSelectedIndex } = createLaunch({
      selected: repos[0],
      repos,
    });

    await launch.openSelected(true);

    expect(invoke).toHaveBeenCalledWith("open_in_cursor", {
      path: "/tmp/a",
      background: true,
    });
    expect(hide).not.toHaveBeenCalled();
    expect(refresh).toHaveBeenCalledWith(false);
    expect(setSelectedIndex).toHaveBeenCalled();
  });

  it("openSelected noop when no repo selected", async () => {
    const { launch } = createLaunch({ repos: [], selected: undefined });
    await launch.openSelected(false);
    expect(invoke).not.toHaveBeenCalled();
    expect(hide).not.toHaveBeenCalled();
  });

  it("openSelected sets launchError on invoke failure", async () => {
    invoke.mockRejectedValueOnce(new Error("cursor missing"));
    const { launch } = createLaunch({ selected: repo("/tmp/a") });

    await launch.openSelected(false);

    expect(launch.launchError).toBe("Error: cursor missing");
    expect(hide).not.toHaveBeenCalled();
  });

  it("dismissLaunchError clears launchError", async () => {
    invoke.mockRejectedValueOnce("fail");
    const { launch } = createLaunch({ selected: repo("/tmp/a") });
    await launch.openSelected(false);
    expect(launch.launchError).toBe("fail");

    launch.dismissLaunchError();
    expect(launch.launchError).toBeNull();
  });

  it("not-cloned repo with manual policy shows a hint instead of opening", async () => {
    const cloneAndOpen = vi.fn().mockResolvedValue(undefined);
    const remote = {
      ...repo("/code/acme/api"),
      cloned: false,
      clone_policy: "manual" as const,
    };
    const { launch } = createLaunch({ selected: remote, cloneAndOpen });
    await launch.openSelected(false);
    expect(launch.launchError).toContain("not cloned");
    expect(cloneAndOpen).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each(["on_open", "auto"] as const)(
    "not-cloned repo with %s policy clones then opens",
    async (policy) => {
      const cloneAndOpen = vi.fn().mockResolvedValue(undefined);
      const remote = {
        ...repo("/code/acme/api"),
        cloned: false,
        clone_policy: policy,
      };
      const { launch } = createLaunch({ selected: remote, cloneAndOpen });
      await launch.openSelected(false);
      expect(cloneAndOpen).toHaveBeenCalledWith(remote);
      expect(invoke).not.toHaveBeenCalled();
      expect(hide).not.toHaveBeenCalled();
    },
  );

  it("not-cloned repo with on_open policy but no cloneAndOpen dep shows the hint", async () => {
    const remote = {
      ...repo("/code/acme/api"),
      cloned: false,
      clone_policy: "on_open" as const,
    };
    const { launch } = createLaunch({ selected: remote });
    await launch.openSelected(false);
    expect(launch.launchError).toContain("not cloned");
    expect(invoke).not.toHaveBeenCalled();
    expect(hide).not.toHaveBeenCalled();
  });

  it.each([null, undefined] as const)(
    "not-cloned repo with clone_policy %s clones then opens",
    async (policy) => {
      const cloneAndOpen = vi.fn().mockResolvedValue(undefined);
      const remote = {
        ...repo("/code/acme/api"),
        cloned: false,
        clone_policy: policy,
      };
      const { launch } = createLaunch({ selected: remote, cloneAndOpen });
      await launch.openSelected(false);
      expect(cloneAndOpen).toHaveBeenCalledExactlyOnceWith(remote);
      expect(launch.launchError).toBeNull();
      expect(invoke).not.toHaveBeenCalled();
    },
  );

  it("background open of a not-cloned repo clones-and-opens without background refresh", async () => {
    const cloneAndOpen = vi.fn().mockResolvedValue(undefined);
    const remote = {
      ...repo("/code/acme/api"),
      cloned: false,
      clone_policy: "auto" as const,
    };
    const { launch, refresh, setSelectedIndex } = createLaunch({
      selected: remote,
      cloneAndOpen,
    });
    await launch.openSelected(true);
    expect(cloneAndOpen).toHaveBeenCalledExactlyOnceWith(remote);
    expect(invoke).not.toHaveBeenCalled();
    expect(refresh).not.toHaveBeenCalled();
    expect(setSelectedIndex).not.toHaveBeenCalled();
  });

  it("hidePanelReportingErrors hides the panel without error", async () => {
    hide.mockClear();
    const { launch } = createLaunch();
    await launch.hidePanelReportingErrors();
    expect(hide).toHaveBeenCalledOnce();
    expect(launch.launchError).toBeNull();
  });

  it("hidePanelReportingErrors surfaces a rejected hide as launchError", async () => {
    hide.mockRejectedValueOnce(new Error("window gone"));
    const { launch } = createLaunch();
    await expect(launch.hidePanelReportingErrors()).resolves.toBeUndefined();
    expect(launch.launchError).toBe("Error: window gone");
  });
});
