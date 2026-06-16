import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import RepoListRow from "./RepoListRow.svelte";
import type { RepoDto } from "$lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const mockRepo: RepoDto = {
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
  convert_block_reason: null,
};

function renderRow(
  repo: RepoDto,
  callbacks: {
    onOpen?: () => void;
    onDetail?: () => void;
    selected?: boolean;
  } = {},
) {
  const onOpen = callbacks.onOpen ?? vi.fn();
  const onDetail = callbacks.onDetail ?? vi.fn();
  const result = render(RepoListRow, {
    props: {
      repo,
      selected: callbacks.selected ?? false,
      onOpen,
      onDetail,
    },
  });
  return { ...result, onOpen, onDetail };
}

describe("RepoListRow", () => {
  afterEach(() => {
    cleanup();
  });

  it("plain_click_calls_onOpen_not_onDetail", async () => {
    const onOpen = vi.fn();
    const onDetail = vi.fn();
    const { getByRole } = renderRow(mockRepo, { onOpen, onDetail });
    const openBtn = getByRole("button", { name: "Open testrepo" });
    await fireEvent.click(openBtn);
    expect(onOpen).toHaveBeenCalledOnce();
    expect(onDetail).not.toHaveBeenCalled();
  });

  it("cmd_click_calls_onDetail_not_onOpen", async () => {
    const onOpen = vi.fn();
    const onDetail = vi.fn();
    const { getByRole } = renderRow(mockRepo, { onOpen, onDetail });
    const openBtn = getByRole("button", { name: "Open testrepo" });
    await fireEvent.click(openBtn, { metaKey: true });
    expect(onDetail).toHaveBeenCalledOnce();
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("info_badge_click_calls_onDetail_not_onOpen", async () => {
    const onOpen = vi.fn();
    const onDetail = vi.fn();
    const { getByLabelText } = renderRow(mockRepo, { onOpen, onDetail });
    const badge = getByLabelText("Open detail for testrepo");
    await fireEvent.click(badge);
    expect(onDetail).toHaveBeenCalledOnce();
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("branch_rendered_when_present", () => {
    const { getByText } = renderRow({ ...mockRepo, branch: "main" });
    expect(getByText("main")).toBeTruthy();
  });

  it("branch_omitted_when_null", () => {
    const { container } = renderRow({ ...mockRepo, branch: null });
    expect(container.textContent).not.toContain("—");
  });

  it("alias_shown_when_set", () => {
    const { getByText } = renderRow({
      ...mockRepo,
      name: "folder",
      alias: "myalias",
    });
    expect(getByText("myalias")).toBeTruthy();
  });

  it("folder_name_shown_when_alias_null", () => {
    const { getByText } = renderRow({
      ...mockRepo,
      name: "folder",
      alias: null,
    });
    expect(getByText("folder")).toBeTruthy();
  });

  it("sync badges are outside the open button", () => {
    const onSync = vi.fn();
    const { getByRole } = render(RepoListRow, {
      props: {
        repo: { ...mockRepo, ahead: 2, behind: 1 },
        onOpen: vi.fn(),
        onDetail: vi.fn(),
        onSync,
      },
    });
    const openBtn = getByRole("button", { name: "Open testrepo" });
    const pushBtn = getByRole("button", { name: /Push 2 commits on main/ });
    expect(openBtn.contains(pushBtn)).toBe(false);
  });

  it("selected row wrapper gets bg-primary and child buttons stay transparent", () => {
    const { container, getByRole } = render(RepoListRow, {
      props: {
        repo: mockRepo,
        selected: true,
        onOpen: vi.fn(),
        onDetail: vi.fn(),
      },
    });
    const rowWrapper = container.querySelector("li > div");
    expect(rowWrapper?.className).toContain("bg-primary");
    const openBtn = getByRole("button", { name: "Open testrepo" });
    const chevronBtn = getByRole("button", {
      name: "Open detail for testrepo",
    });
    expect(openBtn.className).toContain("bg-transparent");
    expect(chevronBtn.className).toContain("bg-transparent");
  });

  it("push chip click does not call onOpen", async () => {
    const onOpen = vi.fn();
    const onSync = vi.fn();
    const { getByRole } = render(RepoListRow, {
      props: {
        repo: { ...mockRepo, ahead: 2, behind: 0 },
        onOpen,
        onDetail: vi.fn(),
        onSync,
      },
    });
    await fireEvent.click(
      getByRole("button", { name: /Push 2 commits on main/ }),
    );
    expect(onSync).toHaveBeenCalledWith("/tmp/testrepo", "main", "push");
    expect(onOpen).not.toHaveBeenCalled();
  });
});
