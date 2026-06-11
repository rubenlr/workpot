import { cleanup, render, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayApp from "./TrayApp.svelte";

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
    listView: "flat" as const,
    sectionedRepos: { pinned: [], dirty: [], recent: [], rest: [] },
    flatIndexByPath: new Map<string, number>(),
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
});
