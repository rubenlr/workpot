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

  it("onPanelKeydown ArrowRight opens detail for selected repo", () => {
    const repos = [repo("a"), repo("b")];
    const list = createTrayListSelection({
      getRepos: () => repos,
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getError: () => null,
    });
    const detail = createTrayDetail();
    const kb = createTrayPanelKeyboard({
      list,
      detail,
      launch: createTrayLaunch({
        getSelectedRepo: () => list.getSelectedRepo(),
        getFilterQuery: () => list.filterQuery,
        getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
        getRepos: () => repos,
        refresh: vi.fn(),
        setSelectedIndex: (i) => {
          list.selectedIndex = i;
        },
      }),
      data: createTrayRepoData(),
    });

    const e = new KeyboardEvent("keydown", {
      key: "ArrowRight",
      bubbles: true,
    });
    kb.onPanelKeydown(e);
    expect(detail.detailRepo?.path).toBe(repos[0].path);
  });

  it("onFilterKeydown delegates refresh shortcut to tray nav", () => {
    const data = createTrayRepoData();
    const startRefresh = vi.spyOn(data, "startBackgroundRefresh");
    const list = createTrayListSelection({
      getRepos: () => [],
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
        getRepos: () => [],
        refresh: vi.fn(),
        setSelectedIndex: (i) => {
          list.selectedIndex = i;
        },
      }),
      data,
    });
    const input = document.createElement("input");
    const e = new KeyboardEvent("keydown", {
      key: "r",
      metaKey: true,
      bubbles: true,
    });
    Object.defineProperty(e, "currentTarget", { value: input });
    kb.onFilterKeydown(e);
    expect(startRefresh).toHaveBeenCalledOnce();
  });

  it("onPanelKeydown ignores repo-filter input target", () => {
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
    input.id = "repo-filter";
    const e = new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true });
    Object.defineProperty(e, "target", { value: input });
    kb.onPanelKeydown(e);
    expect(list.selectedIndex).toBe(0);
  });

  it("onPanelKeydown Enter opens selected repo", async () => {
    const repos = [repo("a")];
    const list = createTrayListSelection({
      getRepos: () => repos,
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getError: () => null,
    });
    const launch = createTrayLaunch({
      getSelectedRepo: () => list.getSelectedRepo(),
      getFilterQuery: () => list.filterQuery,
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getRepos: () => repos,
      refresh: vi.fn(),
      setSelectedIndex: (i) => {
        list.selectedIndex = i;
      },
    });
    const openSelected = vi.spyOn(launch, "openSelected");
    const kb = createTrayPanelKeyboard({
      list,
      detail: createTrayDetail(),
      launch,
      data: createTrayRepoData(),
    });

    kb.onPanelKeydown(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    expect(openSelected).toHaveBeenCalledWith(false);
  });

  it("onPanelKeydown ignores detail form inputs", () => {
    const repos = [repo("a")];
    const list = createTrayListSelection({
      getRepos: () => repos,
      getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
      getError: () => null,
    });
    const detail = createTrayDetail();
    detail.openDetail(repos[0]);
    const kb = createTrayPanelKeyboard({
      list,
      detail,
      launch: createTrayLaunch({
        getSelectedRepo: () => list.getSelectedRepo(),
        getFilterQuery: () => list.filterQuery,
        getSectionCfg: () => ({ maxRecentDays: 14, minRecentCount: 3 }),
        getRepos: () => repos,
        refresh: vi.fn(),
        setSelectedIndex: (i) => {
          list.selectedIndex = i;
        },
      }),
      data: createTrayRepoData(),
    });

    const notes = document.createElement("textarea");
    const e = new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true });
    Object.defineProperty(e, "target", { value: notes });
    kb.onPanelKeydown(e);
    expect(list.selectedIndex).toBe(0);
  });
});
