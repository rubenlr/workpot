import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayListBody from "./TrayListBody.svelte";
import type { TrayListView } from "$lib/tray/logic/list/listState";
import type { SectionedRepos } from "$lib/tray/logic/list/sort";
import type { RepoDto } from "$lib/types";
import {
  TRAY_EMPTY_LIST_MESSAGE,
  TRAY_NO_MATCH_MESSAGE,
} from "$lib/tray/logic/handlers/constants";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const emptySections: SectionedRepos = {
  pinned: [],
  dirty: [],
  recent: [],
  rest: [],
  remote: [],
};

const noop = vi.fn();

function repo(name: string): RepoDto {
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
  };
}

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
    },
  });
}

describe("TrayListBody", () => {
  afterEach(() => {
    cleanup();
  });

  it("error_view_renders_nothing_in_body", () => {
    const { container } = renderBody({ kind: "error", message: "Load failed" });
    expect(container.textContent?.trim()).toBe("");
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
      },
    });
    expect(getByText("Nothing here yet.")).toBeTruthy();
  });

  it("no_match_view_shows_no_match_message", () => {
    const { getByText } = renderBody({ kind: "no-match" });
    expect(getByText(TRAY_NO_MATCH_MESSAGE)).toBeTruthy();
  });

  it("list_view_renders_list_not_placeholder", () => {
    const workpot = repo("workpot");
    const sections: SectionedRepos = { ...emptySections, rest: [workpot] };
    const { getAllByRole } = render(TrayListBody, {
      props: {
        listView: { kind: "list" },
        sectionedRepos: sections,
        flatIndexByPath: new Map([[workpot.path, 0]]),
        onPinReorder: noop,
        onSelectRow: noop,
        onOpen: noop,
        onDetail: noop as (repo: RepoDto, index: number) => void,
      },
    });
    expect(getAllByRole("list").length).toBeGreaterThan(0);
  });

  it("forwards_cloningPaths_and_onClone_to_the_not_cloned_rows", async () => {
    const api: RepoDto = {
      ...repo("api"),
      path: "/code/acme/api",
      cloned: false,
      remote_full_path: "acme/platform/api",
    };
    const web: RepoDto = {
      ...repo("web"),
      path: "/code/acme/web",
      cloned: false,
      remote_full_path: "acme/platform/web",
    };
    const onClone = vi.fn();
    const { getByRole } = render(TrayListBody, {
      props: {
        listView: { kind: "list" },
        sectionedRepos: { ...emptySections, remote: [api, web] },
        flatIndexByPath: new Map([
          [api.path, 0],
          [web.path, 1],
        ]),
        cloningPaths: [api.path],
        onClone,
        onPinReorder: noop,
        onSelectRow: noop,
        onOpen: noop,
        onDetail: noop as (repo: RepoDto, index: number) => void,
      },
    });

    const apiButton = getByRole("button", { name: "Clone api" });
    expect((apiButton as HTMLButtonElement).disabled).toBe(true);
    expect(apiButton.textContent).toContain("Cloning");

    await fireEvent.click(getByRole("button", { name: "Clone web" }));
    expect(onClone).toHaveBeenCalledExactlyOnceWith(web);
  });
});
