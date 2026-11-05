import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayApp from "./TrayApp.svelte";
import type { RepoDto } from "$lib/types";

const panelState = vi.hoisted(() => ({
  cloningPaths: [] as string[],
  sectionedRepos: {
    pinned: [],
    dirty: [],
    recent: [],
    rest: [],
    remote: [],
  } as import("$lib/tray/logic/list/sort").SectionedRepos,
  listView: {
    kind: "empty-list",
  } as import("$lib/tray/logic/list/listState").TrayListView,
  flatIndexByPath: new Map<string, number>(),
}));
const cloneRepo = vi.fn().mockResolvedValue(undefined);
const mount = vi.fn().mockResolvedValue(undefined);
const destroy = vi.fn();
const onPanelKeydown = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock("$lib/tray/state/createTrayPanel.svelte", () => ({
  createTrayPanel: () => ({
    mount,
    destroy,
    onPanelKeydown,
    listMaxHeightPx: 492,
    launchError: null,
    dismissLaunchError: vi.fn(),
    filterQuery: "",
    allTags: [],
    tagAutocompletePrefix: "",
    onFilterKeydown: vi.fn(),
    onTagAutocompleteSelect: vi.fn(),
    bindFilterInput: vi.fn(),
    listView: panelState.listView,
    sectionedRepos: panelState.sectionedRepos,
    flatIndexByPath: panelState.flatIndexByPath,
    cloningPaths: panelState.cloningPaths,
    cloneRepo,
    selectedIndex: 0,
    handlePinReorder: vi.fn(),
    openSelected: vi.fn(),
    openDetail: vi.fn(),
    removeTagFromRepo: vi.fn(),
    appendTagFilter: vi.fn(),
    detailRepo: null,
    focusTagOnDetailOpen: false,
    clearTagFocusRequest: vi.fn(),
    closeDetail: vi.fn(),
    refreshReposAndDetail: vi.fn(),
  }),
}));

describe("TrayApp", () => {
  afterEach(() => {
    cleanup();
    cloneRepo.mockClear();
    panelState.cloningPaths = [];
    panelState.listView = { kind: "empty-list" };
    panelState.sectionedRepos = {
      pinned: [],
      dirty: [],
      recent: [],
      rest: [],
      remote: [],
    };
    panelState.flatIndexByPath = new Map();
    mount.mockClear();
    destroy.mockClear();
  });

  it("mounts tray panel on load", async () => {
    render(TrayApp);
    await waitFor(() => {
      expect(mount).toHaveBeenCalledOnce();
    });
  });

  it("destroys tray panel on unmount", async () => {
    const { unmount } = render(TrayApp);
    await waitFor(() => {
      expect(mount).toHaveBeenCalledOnce();
    });
    unmount();
    expect(destroy).toHaveBeenCalledOnce();
  });

  describe("clone wiring", () => {
    const api: RepoDto = {
      path: "/code/acme/api",
      name: "api",
      alias: null,
      branch: null,
      ahead: null,
      behind: null,
      is_dirty: null,
      parent_dir: "~/code/acme",
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
      cloned: false,
      remote_full_path: "acme/platform/api",
    };

    function showRemoteRepo() {
      panelState.listView = { kind: "list" };
      panelState.sectionedRepos = {
        pinned: [],
        dirty: [],
        recent: [],
        rest: [],
        remote: [api],
      };
      panelState.flatIndexByPath = new Map([[api.path, 0]]);
    }

    it("clicking Clone on a not-cloned row calls panel.cloneRepo with that repo", async () => {
      showRemoteRepo();
      const { findByRole } = render(TrayApp);
      await fireEvent.click(await findByRole("button", { name: "Clone api" }));
      expect(cloneRepo).toHaveBeenCalledExactlyOnceWith(api);
    });

    it("passes panel.cloningPaths down so a running clone shows as Cloning", async () => {
      showRemoteRepo();
      panelState.cloningPaths = [api.path];
      const { findByRole } = render(TrayApp);
      const button = await findByRole("button", { name: "Clone api" });
      expect((button as HTMLButtonElement).disabled).toBe(true);
      expect(button.textContent).toContain("Cloning");
    });
  });
});
