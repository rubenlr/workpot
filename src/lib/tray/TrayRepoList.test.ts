import { cleanup, render } from "@testing-library/svelte";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import TrayRepoList from "./TrayRepoList.svelte";
import type { SectionedRepos } from "$lib/sort";
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
    ...overrides,
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
  return {
    ...render(TrayRepoList, {
      props: {
        sectionedRepos,
        flatIndexByPath,
        selectedIndex: opts.selectedIndex ?? 0,
        onPinReorder: vi.fn(),
        onSelectRow: vi.fn(),
        onOpen,
        onDetail,
      },
    }),
    onOpen,
    onDetail,
  };
}

const empty: SectionedRepos = { pinned: [], dirty: [], recent: [], rest: [] };

describe("TrayRepoList", () => {
  beforeAll(() => {
    // jsdom does not implement scrollIntoView
    Element.prototype.scrollIntoView = vi.fn();
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
});
