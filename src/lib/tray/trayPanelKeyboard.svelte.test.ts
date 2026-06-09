import { describe, expect, it, vi } from "vitest";
import type { RepoDto } from "$lib/types";
import { createTrayDetail } from "./trayDetail.svelte";
import { createTrayLaunch } from "./trayLaunch.svelte";
import { createTrayListSelection } from "./trayListSelection.svelte";
import { createTrayPanelKeyboard } from "./trayPanelKeyboard.svelte";
import { createTrayRepoData } from "./trayRepoData.svelte";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide: vi.fn().mockResolvedValue(undefined) }),
}));

function repo(name: string): RepoDto {
  const path = `/tmp/${name}`;
  return {
    path,
    name,
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

function panelKeyboard() {
  const data = createTrayRepoData();
  const detail = createTrayDetail();
  const list = createTrayListSelection({
    getRepos: () => data.repos,
    getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
    getError: () => data.error,
  });
  const launch = createTrayLaunch({
    getSelectedRepo: () => list.getSelectedRepo(),
    getFilterQuery: () => list.filterQuery,
    getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
    getRepos: () => data.repos,
    refresh: vi.fn().mockResolvedValue(undefined),
    setSelectedIndex: (i) => {
      list.selectedIndex = i;
    },
  });
  return createTrayPanelKeyboard({ list, detail, launch, data });
}

describe("createTrayPanelKeyboard", () => {
  it("onPanelKeydown ArrowDown moves selection", () => {
    const list = createTrayListSelection({
      getRepos: () => [repo("a"), repo("b")],
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getError: () => null,
    });
    const detail = createTrayDetail();
    const launch = createTrayLaunch({
      getSelectedRepo: () => list.getSelectedRepo(),
      getFilterQuery: () => list.filterQuery,
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getRepos: () => [repo("a"), repo("b")],
      refresh: vi.fn(),
      setSelectedIndex: (i) => {
        list.selectedIndex = i;
      },
    });
    const kb = createTrayPanelKeyboard({
      list,
      detail,
      launch,
      data: createTrayRepoData(),
    });

    const e = new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true });
    const prevented = Object.assign(e, {
      preventDefault: vi.fn(),
    }) as KeyboardEvent;
    kb.onPanelKeydown(prevented);
    expect(list.selectedIndex).toBe(1);
    expect(prevented.preventDefault).toHaveBeenCalled();
  });

  it("onFilterKeydown ArrowDown at end of input moves selection", () => {
    const list = createTrayListSelection({
      getRepos: () => [repo("a"), repo("b")],
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getError: () => null,
    });
    const kb = createTrayPanelKeyboard({
      list,
      detail: createTrayDetail(),
      launch: createTrayLaunch({
        getSelectedRepo: () => list.getSelectedRepo(),
        getFilterQuery: () => list.filterQuery,
        getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
        getRepos: () => [repo("a"), repo("b")],
        refresh: vi.fn(),
        setSelectedIndex: (i) => {
          list.selectedIndex = i;
        },
      }),
      data: createTrayRepoData(),
    });

    const input = document.createElement("input");
    input.value = "tag:work";
    input.selectionStart = 8;
    input.selectionEnd = 8;
    const e = new KeyboardEvent("keydown", {
      key: "ArrowDown",
      bubbles: true,
    });
    Object.defineProperty(e, "currentTarget", { value: input });
    const prevented = Object.assign(e, {
      preventDefault: vi.fn(),
    }) as KeyboardEvent;

    kb.onFilterKeydown(prevented);
    expect(list.selectedIndex).toBe(1);
    expect(prevented.preventDefault).toHaveBeenCalled();
  });

  it("bindFilterInput enables focusFilter", () => {
    const kb = panelKeyboard();
    const input = document.createElement("input");
    input.focus = vi.fn();
    kb.bindFilterInput(input);
    kb.focusFilter();
    expect(input.focus).toHaveBeenCalled();
  });
});
