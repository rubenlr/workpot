import { cleanup, fireEvent, render } from "@testing-library/svelte";
import {
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { invoke } from "@tauri-apps/api/core";
import TrayRepoList from "./TrayRepoList.svelte";
import type { SectionedRepos } from "$lib/tray/logic/list/sort";
import type { RepoDto } from "$lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

function repo(name: string, overrides: Partial<RepoDto> = {}): RepoDto {
  return {
    path: `/tmp/${name}`,
    name,
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
    convert_block_reason: null,
    ...overrides,
  };
}

function bindableSelectedIndex(initial = 0) {
  let value = initial;
  return {
    props: {
      get selectedIndex() {
        return value;
      },
      set selectedIndex(v: number) {
        value = v;
      },
    },
  };
}

function renderList(
  sectionedRepos: SectionedRepos,
  opts: {
    selectedIndex?: number;
    onOpen?: (i: number) => void;
    onDetail?: (repo: RepoDto, i: number) => void;
  } = {},
) {
  const repos = [
    ...sectionedRepos.pinned,
    ...sectionedRepos.dirty,
    ...sectionedRepos.recent,
    ...sectionedRepos.rest,
  ];
  const flatIndexByPath = new Map(repos.map((r, i) => [r.path, i]));
  const onOpen = opts.onOpen ?? vi.fn();
  const onDetail = opts.onDetail ?? vi.fn();
  const selectedIndex = bindableSelectedIndex(opts.selectedIndex ?? 0);
  const baseProps = {
    sectionedRepos,
    flatIndexByPath,
    ...selectedIndex.props,
    onPinReorder: vi.fn(),
    onSelectRow: vi.fn(),
    onOpen,
    onDetail,
  };
  const view = render(TrayRepoList, { props: baseProps });
  return {
    ...view,
    onOpen,
    onDetail,
    rerenderWithSelection(index: number) {
      selectedIndex.props.selectedIndex = index;
      return view.rerender({ ...baseProps, ...selectedIndex.props });
    },
  };
}

const empty: SectionedRepos = { pinned: [], dirty: [], recent: [], rest: [] };

describe("TrayRepoList", () => {
  beforeAll(() => {
    // jsdom does not implement scrollIntoView
    Element.prototype.scrollIntoView = vi.fn();
  });

  beforeEach(() => {
    vi.mocked(invoke).mockClear();
  });

  afterEach(() => {
    cleanup();
  });

  it("renders_list_container", () => {
    const { getAllByRole } = renderList({ ...empty, rest: [repo("workpot")] });
    expect(getAllByRole("list").length).toBeGreaterThan(0);
  });

  it("renders_repo_names_in_rest_section", () => {
    const { getByText } = renderList({
      ...empty,
      rest: [repo("workpot"), repo("myapp")],
    });
    expect(getByText("workpot")).toBeTruthy();
    expect(getByText("myapp")).toBeTruthy();
  });

  it("empty_sections_not_rendered", () => {
    const { queryByText } = renderList({ ...empty, rest: [repo("workpot")] });
    expect(queryByText("Pinned")).toBeNull();
    expect(queryByText("Dirty")).toBeNull();
    expect(queryByText("Recent")).toBeNull();
  });

  it("section_header_shown_for_non_empty_section", () => {
    const { getByText } = renderList({
      ...empty,
      pinned: [repo("pinned-repo", { pinned: true })],
    });
    expect(getByText("Pinned")).toBeTruthy();
  });

  it("multiple_sections_rendered_when_non_empty", () => {
    const { getByText } = renderList({
      ...empty,
      pinned: [repo("pinned-repo", { pinned: true })],
      rest: [repo("other")],
    });
    expect(getByText("Pinned")).toBeTruthy();
    expect(getByText("Rest")).toBeTruthy();
  });

  it("selected_row_has_data_row_index_attribute", () => {
    const { container } = renderList({
      ...empty,
      rest: [repo("a"), repo("b")],
    });
    const rows = container.querySelectorAll("[data-row-index]");
    expect(rows.length).toBe(2);
  });

  it("mouseenter updates selected row", async () => {
    const { container } = renderList({
      ...empty,
      rest: [repo("a"), repo("b")],
    });
    const row1 = container.querySelector('[data-row-index="1"]');
    expect(row1).toBeTruthy();
    await fireEvent.mouseEnter(row1!);
    expect(
      container.querySelector('[data-row-index="1"] .bg-primary'),
    ).toBeTruthy();
  });

  it("keyboard selection clears stale hover", async () => {
    const sectioned = { ...empty, rest: [repo("a"), repo("b")] };
    const { container, rerenderWithSelection } = renderList(sectioned);
    const row0 = container.querySelector('[data-row-index="0"]');
    expect(row0).toBeTruthy();
    await fireEvent.mouseEnter(row0!);
    await rerenderWithSelection(1);
    const row0Button = container.querySelector('[data-row-index="0"] button');
    expect(row0Button?.classList.contains("bg-hover-overlay")).toBe(false);
    const row1Selected = container.querySelector(
      '[data-row-index="1"] .bg-primary',
    );
    expect(row1Selected).toBeTruthy();
  });

  it("scrollIntoView_called_when_selectedIndex_changes", async () => {
    const scrollSpy = vi.spyOn(Element.prototype, "scrollIntoView");
    const { rerenderWithSelection } = renderList({
      ...empty,
      rest: [repo("a"), repo("b")],
    });
    scrollSpy.mockClear();

    await rerenderWithSelection(1);

    await vi.waitFor(() => {
      expect(scrollSpy).toHaveBeenCalledWith({ block: "nearest" });
    });
    scrollSpy.mockRestore();
  });

  it("contextmenu invokes show_repo_context_menu with flat camelCase payload", async () => {
    const workpot = repo("workpot", {
      path: "/tmp/workpot",
      pinned: false,
      tags: ["backend"],
      convert_to: "bare",
      convert_block_reason: "dirty working tree",
    });
    const { container } = renderList({ ...empty, rest: [workpot] });
    const row = container.querySelector('[data-row-index="0"]');
    expect(row).toBeTruthy();

    await fireEvent.contextMenu(row!);

    expect(invoke).toHaveBeenCalledOnce();
    expect(invoke).toHaveBeenCalledWith("show_repo_context_menu", {
      repoPath: "/tmp/workpot",
      isPinned: false,
      tags: ["backend"],
      convertTo: "bare",
      convertBlockReason: "dirty working tree",
    });
  });
});
