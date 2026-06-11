import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayListBody from "./TrayListBody.svelte";
import type { TrayListView } from "$lib/listState";
import type { SectionedRepos } from "$lib/sort";
import type { RepoDto } from "$lib/types";
import { TRAY_EMPTY_LIST_MESSAGE, TRAY_NO_MATCH_MESSAGE } from "./constants";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const emptySections: SectionedRepos = {
  pinned: [],
  dirty: [],
  recent: [],
  rest: [],
};

const noop = vi.fn();

function renderBody(
  listView: TrayListView,
  sections: SectionedRepos = emptySections,
) {
  return render(TrayListBody, {
    props: {
      listView,
      sectionedRepos: sections,
      flatIndexByPath: new Map<string, number>(),
      onPinReorder: noop,
      onSelectRow: noop,
      onOpen: noop,
      onDetail: noop as (repo: RepoDto, index: number) => void,
      onTagRemove: noop,
      onTagFilter: noop,
    },
  });
}

describe("TrayListBody", () => {
  afterEach(() => {
    cleanup();
  });

  it("error_view_shows_placeholder_with_error_message", () => {
    const { getByText } = renderBody({ kind: "error", message: "Load failed" });
    expect(getByText("Load failed")).toBeTruthy();
  });

  it("empty_list_view_shows_default_empty_message", () => {
    const { getByText } = renderBody({ kind: "empty-list" });
    expect(getByText(TRAY_EMPTY_LIST_MESSAGE)).toBeTruthy();
  });

  it("empty_list_view_shows_custom_empty_message", () => {
    const { getByText } = render(TrayListBody, {
      props: {
        listView: { kind: "empty-list" },
        emptyListMessage: "Nothing here yet.",
        sectionedRepos: emptySections,
        flatIndexByPath: new Map(),
        onPinReorder: noop,
        onSelectRow: noop,
        onOpen: noop,
        onDetail: noop as (repo: RepoDto, index: number) => void,
        onTagRemove: noop,
        onTagFilter: noop,
      },
    });
    expect(getByText("Nothing here yet.")).toBeTruthy();
  });

  it("no_match_view_shows_no_match_message", () => {
    const { getByText } = renderBody({ kind: "no-match" });
    expect(getByText(TRAY_NO_MATCH_MESSAGE)).toBeTruthy();
  });

  it("list_view_renders_list_not_placeholder", () => {
    const { queryByRole } = renderBody({ kind: "list" });
    expect(queryByRole("list")).toBeTruthy();
  });
});
