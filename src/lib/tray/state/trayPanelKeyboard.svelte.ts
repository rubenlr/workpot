import { shouldNavigateListOnFilterArrow } from "$lib/tray/logic/list/filterNavigation";
import { applyTrayNavigationKey } from "$lib/tray/logic/list/trayKeyboard";
import type { TrayDetail } from "./trayDetail.svelte";
import type { TrayLaunch } from "./trayLaunch.svelte";
import type { TrayListSelection } from "./trayListSelection.svelte";
import type { TrayRepoData } from "./trayRepoData.svelte";

export interface TrayPanelKeyboardDeps {
  list: TrayListSelection;
  detail: TrayDetail;
  launch: TrayLaunch;
  data: TrayRepoData;
}

export function createTrayPanelKeyboard(deps: TrayPanelKeyboardDeps) {
  let filterInput = $state<HTMLInputElement | null>(null);

  function focusFilter() {
    filterInput?.focus();
  }

  function bindFilterInput(el: HTMLInputElement | null) {
    filterInput = el;
  }

  function applyTrayNav(e: KeyboardEvent, mode: "filter" | "panel") {
    const { list, detail, launch, data } = deps;
    return applyTrayNavigationKey(
      e,
      {
        detailRepo: detail.detailRepo,
        getSelectedRepo: () => list.getSelectedRepo(),
      },
      {
        onRefresh: () => void data.startBackgroundRefresh(),
        onCloseDetail: () => detail.closeDetail(),
        onHidePanel: () => void launch.hidePanel(),
        onOpenDetailForSelection: () => {
          const repo = list.getSelectedRepo();
          if (repo) {
            detail.openDetail(repo);
          }
        },
        onMoveSelection: list.moveSelection,
        onOpenSelected: (background: boolean) =>
          void launch.openSelected(background),
      },
      mode,
    );
  }

  function onFilterKeydown(e: KeyboardEvent) {
    if (applyTrayNav(e, "filter")) {
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const input = e.currentTarget as HTMLInputElement;
      const start = input.selectionStart ?? 0;
      const end = input.selectionEnd ?? 0;
      if (
        shouldNavigateListOnFilterArrow(
          e.key,
          deps.list.filterQuery,
          start,
          end,
          input.value.length,
        )
      ) {
        e.preventDefault();
        deps.list.moveSelection(e.key === "ArrowDown" ? 1 : -1);
      }
    }
  }

  function onPanelKeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement && e.target.id === "repo-filter") {
      return;
    }
    if (
      deps.detail.detailRepo !== null &&
      (e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement)
    ) {
      return;
    }
    applyTrayNav(e, "panel");
  }

  return {
    bindFilterInput,
    focusFilter,
    onFilterKeydown,
    onPanelKeydown,
  };
}

export type TrayPanelKeyboard = ReturnType<typeof createTrayPanelKeyboard>;
