import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayPanelChrome from "./TrayPanelChrome.svelte";
import type { SectionedRepos } from "$lib/tray/logic/list/sort";
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
  ahead: null,
  behind: null,
  is_dirty: false,
  parent_dir: "~/tmp",
  last_opened_at: null,
  git_state_error: null,
  pinned: false,
  pin_order: null,
  notes: null,
  tags: [],
  branches: [],
  is_bare: false,
  convert_to: null,
};

import type { TrayListView } from "$lib/tray/logic/list/listState";

function renderChrome(
  opts: {
    launchError?: string | null;
    listError?: string | null;
    onDismissListError?: () => void;
    listView?: TrayListView;
    detailRepo?: RepoDto | null;
    sectionedRepos?: SectionedRepos;
    flatIndexByPath?: Map<string, number>;
  } = {},
) {
  const sections = opts.sectionedRepos ?? emptySections;
  const workpot = { ...baseRepo, name: "workpot", path: "/tmp/workpot" };
  const populatedSections: SectionedRepos =
    sections === emptySections && opts.listView?.kind === "list"
      ? { ...emptySections, rest: [workpot] }
      : sections;

  return render(TrayPanelChrome, {
    props: {
      listMaxHeightPx: 600,
      launchError: opts.launchError ?? null,
      listError: opts.listError ?? null,
      onDismissListError: opts.onDismissListError,
      onDismissLaunchError: vi.fn(),
      filterQuery: "",
      allTags: [],
      tagAutocompletePrefix: null,
      onFilterKeydown: vi.fn(),
      onTagSelect: vi.fn(),
      bindFilterInput: vi.fn(),
      listView: opts.listView ?? { kind: "empty-list" as const },
      sectionedRepos: populatedSections,
      flatIndexByPath:
        opts.flatIndexByPath ??
        new Map(
          populatedSections.rest[0]
            ? [[populatedSections.rest[0].path, 0]]
            : [],
        ),
      selectedIndex: 0,
      onPinReorder: vi.fn(),
      onSelectRow: vi.fn(),
      onOpen: vi.fn(),
      onDetail: vi.fn() as (repo: RepoDto, index: number) => void,
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
    expect(queryByText("No repos indexed yet.")).toBeTruthy();
  });

  it("shows_detail_pane_when_detailRepo_provided", () => {
    const { queryByRole } = renderChrome({ detailRepo: baseRepo });
    expect(queryByRole("heading", { level: 2 })).toBeTruthy();
  });

  it("hides_list_body_when_detail_pane_active", () => {
    const { queryByText } = renderChrome({ detailRepo: baseRepo });
    expect(queryByText("No repos indexed yet.")).toBeNull();
  });

  it("hides_filter_bar_when_detail_pane_active", () => {
    const { container } = renderChrome({ detailRepo: baseRepo });
    expect(container.querySelector("#repo-filter")).toBeNull();
  });

  it("list_error_renders_dismissible_banner_when_handler_provided", async () => {
    const onDismissListError = vi.fn();
    const { getByRole, getByText } = renderChrome({
      listError: "SQLite database is locked",
      listView: { kind: "error", message: "SQLite database is locked" },
      onDismissListError,
    });
    expect(getByRole("alert")).toBeTruthy();
    expect(getByText("SQLite database is locked")).toBeTruthy();
    await getByText("DISMISS").click();
    expect(onDismissListError).toHaveBeenCalledOnce();
  });

  it("list_error_renders_banner_without_dismiss_when_handler_omitted", () => {
    const { getByRole, getByText, queryByText } = renderChrome({
      listError: "SQLite database is locked",
      listView: { kind: "error", message: "SQLite database is locked" },
    });
    expect(getByRole("alert")).toBeTruthy();
    expect(getByText("SQLite database is locked")).toBeTruthy();
    expect(queryByText("DISMISS")).toBeNull();
    expect(
      queryByText("SQLite database is locked", { selector: "p" }),
    ).toBeNull();
  });

  it("list_error_with_cached_repos_shows_banner_and_list_rows", () => {
    const workpot = { ...baseRepo, name: "workpot", path: "/tmp/workpot" };
    const sections: SectionedRepos = { ...emptySections, rest: [workpot] };
    const { getByRole, getByText, getAllByRole } = renderChrome({
      listError: "git push failed: rejected",
      listView: { kind: "list" },
      sectionedRepos: sections,
      flatIndexByPath: new Map([[workpot.path, 0]]),
    });
    expect(getByRole("alert")).toBeTruthy();
    expect(getByText("git push failed: rejected")).toBeTruthy();
    expect(getAllByRole("list").length).toBeGreaterThan(0);
  });
});
