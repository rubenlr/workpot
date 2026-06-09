import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayPanelChrome from "./TrayPanelChrome.svelte";
import type { SectionedRepos } from "$lib/sort";
import type { RepoDto } from "$lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

const emptySections: SectionedRepos = {
  pinned: [],
  dirty: [],
  recent: [],
  rest: [],
};

const baseRepo: RepoDto = {
  path: "/tmp/testrepo",
  name: "testrepo",
  alias: null,
  branch: "main",
  is_dirty: false,
  parent_dir: "~/tmp",
  last_opened_at: null,
  git_state_error: null,
  pinned: false,
  pin_order: null,
  notes: null,
  tags: [],
  branches: [],
};

function renderChrome(
  opts: {
    launchError?: string | null;
    detailRepo?: RepoDto | null;
  } = {},
) {
  return render(TrayPanelChrome, {
    props: {
      listMaxHeightPx: 600,
      launchError: opts.launchError ?? null,
      onDismissLaunchError: vi.fn(),
      filterQuery: "",
      allTags: [],
      tagAutocompletePrefix: null,
      onFilterKeydown: vi.fn(),
      onTagSelect: vi.fn(),
      bindFilterInput: vi.fn(),
      listView: { kind: "empty-list" as const },
      sectionedRepos: emptySections,
      flatIndexByPath: new Map<string, number>(),
      selectedIndex: 0,
      onPinReorder: vi.fn(),
      onSelectRow: vi.fn(),
      onOpen: vi.fn(),
      onDetail: vi.fn() as (repo: RepoDto, index: number) => void,
      onTagRemove: vi.fn(),
      onTagFilter: vi.fn(),
      detailRepo: opts.detailRepo ?? null,
      onCloseDetail: vi.fn(),
      onDetailMutated: vi.fn(),
    },
  });
}

describe("TrayPanelChrome", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders filter input", () => {
    const { getByPlaceholderText } = renderChrome();
    expect(getByPlaceholderText("Filter repos…")).toBeTruthy();
  });

  it("launch_error_banner_hidden_when_no_error", () => {
    const { queryByRole } = renderChrome({ launchError: null });
    expect(queryByRole("alert")).toBeNull();
  });

  it("launch_error_banner_shown_when_error_set", () => {
    const { getByRole, getByText } = renderChrome({
      launchError: "cursor: command not found",
    });
    expect(getByRole("alert")).toBeTruthy();
    expect(getByText("cursor: command not found")).toBeTruthy();
  });

  it("shows_list_body_when_no_detail_repo", () => {
    const { queryByText } = renderChrome({ detailRepo: null });
    // placeholder message visible when no repos
    expect(queryByText("No repos indexed yet.")).toBeTruthy();
  });

  it("shows_detail_pane_when_detailRepo_provided", () => {
    const { queryByRole } = renderChrome({ detailRepo: baseRepo });
    // list body hidden, detail pane visible (has heading for repo name)
    expect(queryByRole("heading", { level: 2 })).toBeTruthy();
  });

  it("hides_list_body_when_detail_pane_active", () => {
    const { queryByText } = renderChrome({ detailRepo: baseRepo });
    expect(queryByText("No repos indexed yet.")).toBeNull();
  });
});
