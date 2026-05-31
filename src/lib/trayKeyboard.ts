import { shouldSuppressTrayListKeyWhenDetailOpen } from "./detailNavigation";
import type { RepoDto } from "./types";

export function isTrayRefreshShortcut(metaKey: boolean, key: string): boolean {
  return metaKey && (key === "r" || key === "R");
}

export interface TrayNavCtx {
  detailRepo: RepoDto | null;
  getSelectedRepo: () => RepoDto | undefined;
}

export interface TrayNavActions {
  onRefresh: () => void;
  onCloseDetail: () => void;
  onHidePanel: () => void;
  onOpenDetailForSelection: () => void;
  onMoveSelection: (delta: number) => void;
  onOpenSelected: (background: boolean) => void;
}

/**
 * Shared tray list navigation for filter input and panel window handlers.
 * Returns true when the caller should stop processing the event.
 */
export function applyTrayNavigationKey(
  e: KeyboardEvent,
  ctx: TrayNavCtx,
  actions: TrayNavActions,
  mode: "filter" | "panel",
): boolean {
  if (isTrayRefreshShortcut(e.metaKey, e.key)) {
    e.preventDefault();
    actions.onRefresh();
    return true;
  }

  if (ctx.detailRepo !== null) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      actions.onCloseDetail();
      return true;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      actions.onCloseDetail();
      actions.onHidePanel();
      return true;
    }
    if (shouldSuppressTrayListKeyWhenDetailOpen(e.key, e.metaKey)) {
      return true;
    }
  }

  if (e.key === "ArrowRight" && ctx.detailRepo === null && ctx.getSelectedRepo()) {
    e.preventDefault();
    actions.onOpenDetailForSelection();
    return true;
  }

  if (mode === "panel") {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      actions.onMoveSelection(1);
      return true;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      actions.onMoveSelection(-1);
      return true;
    }
    if (e.key === "Tab" && !e.shiftKey) {
      e.preventDefault();
      actions.onMoveSelection(1);
      return true;
    }
  }

  if (e.key === "Escape") {
    e.preventDefault();
    actions.onCloseDetail();
    actions.onHidePanel();
    return true;
  }

  if (e.key === "Enter") {
    e.preventDefault();
    actions.onOpenSelected(e.metaKey);
    return true;
  }

  return false;
}
