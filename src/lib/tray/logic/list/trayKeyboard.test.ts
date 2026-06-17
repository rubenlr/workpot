import { describe, expect, it, vi } from "vitest";
import { applyTrayNavigationKey } from "./trayKeyboard";
import type { RepoDto } from "$lib/types";

function repo(name: string): RepoDto {
  return {
    name,
    alias: null,
    path: `/tmp/${name}`,
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
    is_bare: false,
    convert_to: null,
    convert_block_reason: null,
  };
}

function keyEvent(
  key: string,
  init: Partial<KeyboardEventInit> = {},
): KeyboardEvent {
  return new KeyboardEvent("keydown", { key, bubbles: true, ...init });
}

describe("applyTrayNavigationKey", () => {
  const selected = repo("alpha");

  it("triggers refresh on Cmd+R", () => {
    const onRefresh = vi.fn();
    const e = keyEvent("r", { metaKey: true });
    const handled = applyTrayNavigationKey(
      e,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh,
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection: vi.fn(),
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(handled).toBe(true);
    expect(onRefresh).toHaveBeenCalledOnce();
  });

  it("closes detail on ArrowLeft when detail is open", () => {
    const onCloseDetail = vi.fn();
    const e = keyEvent("ArrowLeft");
    applyTrayNavigationKey(
      e,
      { detailRepo: selected, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail,
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection: vi.fn(),
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(onCloseDetail).toHaveBeenCalledOnce();
  });

  it("suppresses ArrowDown in filter mode when detail is open", () => {
    const onMoveSelection = vi.fn();
    const e = keyEvent("ArrowDown");
    const handled = applyTrayNavigationKey(
      e,
      { detailRepo: selected, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection,
        onOpenSelected: vi.fn(),
      },
      "filter",
    );
    expect(handled).toBe(true);
    expect(onMoveSelection).not.toHaveBeenCalled();
    expect(e.defaultPrevented).toBe(false);
  });

  it("moves selection on ArrowDown in panel mode", () => {
    const onMoveSelection = vi.fn();
    const e = keyEvent("ArrowDown");
    applyTrayNavigationKey(
      e,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection,
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(onMoveSelection).toHaveBeenCalledWith(1);
  });

  it("enter_calls_onOpenSelected_plain_open_without_meta", () => {
    const onOpenSelected = vi.fn();
    const onMoveSelection = vi.fn();
    const e = keyEvent("Enter");
    const handled = applyTrayNavigationKey(
      e,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection,
        onOpenSelected,
      },
      "panel",
    );
    expect(handled).toBe(true);
    expect(onOpenSelected).toHaveBeenCalledWith(false);
    expect(onMoveSelection).not.toHaveBeenCalled();
  });

  it("enter_with_meta_calls_onOpenSelected_as_background", () => {
    const onOpenSelected = vi.fn();
    const e = keyEvent("Enter", { metaKey: true });
    applyTrayNavigationKey(
      e,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection: vi.fn(),
        onOpenSelected,
      },
      "panel",
    );
    expect(onOpenSelected).toHaveBeenCalledWith(true);
  });

  it("escape_in_detail_closes_and_hides_panel", () => {
    const onCloseDetail = vi.fn();
    const onHidePanel = vi.fn();
    const e = keyEvent("Escape");
    const handled = applyTrayNavigationKey(
      e,
      { detailRepo: selected, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail,
        onHidePanel,
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection: vi.fn(),
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(handled).toBe(true);
    expect(onCloseDetail).toHaveBeenCalledOnce();
    expect(onHidePanel).toHaveBeenCalledOnce();
  });

  it("moves_selection_on_arrow_up_and_tab_in_panel_mode", () => {
    const onMoveSelection = vi.fn();
    const up = keyEvent("ArrowUp");
    applyTrayNavigationKey(
      up,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection,
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(onMoveSelection).toHaveBeenCalledWith(-1);

    onMoveSelection.mockClear();
    const tab = keyEvent("Tab");
    applyTrayNavigationKey(
      tab,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection,
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(onMoveSelection).toHaveBeenCalledWith(1);
  });

  it("escape_in_list_closes_detail_and_hides_panel", () => {
    const onCloseDetail = vi.fn();
    const onHidePanel = vi.fn();
    const e = keyEvent("Escape");
    const handled = applyTrayNavigationKey(
      e,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail,
        onHidePanel,
        onOpenDetailForSelection: vi.fn(),
        onMoveSelection: vi.fn(),
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(handled).toBe(true);
    expect(onCloseDetail).toHaveBeenCalledOnce();
    expect(onHidePanel).toHaveBeenCalledOnce();
  });

  it("arrow_right_opens_detail_for_selection", () => {
    const onOpenDetailForSelection = vi.fn();
    const e = keyEvent("ArrowRight");
    const handled = applyTrayNavigationKey(
      e,
      { detailRepo: null, getSelectedRepo: () => selected },
      {
        onRefresh: vi.fn(),
        onCloseDetail: vi.fn(),
        onHidePanel: vi.fn(),
        onOpenDetailForSelection,
        onMoveSelection: vi.fn(),
        onOpenSelected: vi.fn(),
      },
      "panel",
    );
    expect(handled).toBe(true);
    expect(onOpenDetailForSelection).toHaveBeenCalledOnce();
  });
});
