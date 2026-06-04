import { SvelteMap } from "svelte/reactivity";
import { trayListView } from "$lib/listState";
import { clampSelectionIndex, moveSelectionIndex } from "$lib/selection";
import type { SectionConfig } from "$lib/sort";
import {
  appendTagToFilterQuery,
  replaceTrailingTagAutocomplete,
  trailingTagAutocompletePrefix,
} from "$lib/tagFilter";
import { filterAndSectionRepos, flatSectioned } from "$lib/trayList";
import type { RepoDto } from "$lib/types";

export interface TrayListSelectionDeps {
  getRepos: () => RepoDto[];
  getSectionCfg: () => SectionConfig;
  getError: () => string | null;
}

export function createTrayListSelection(deps: TrayListSelectionDeps) {
  let filterQuery = $state("");
  let rawSelectedIndex = $state(0);

  let sectionedRepos = $derived(
    filterAndSectionRepos(deps.getRepos(), filterQuery, deps.getSectionCfg()),
  );
  let flatVisible = $derived(flatSectioned(sectionedRepos));
  let flatIndexByPath = $derived(
    new SvelteMap(flatVisible.map((r, i) => [r.path, i] as const)),
  );
  let tagAutocompletePrefix = $derived(
    trailingTagAutocompletePrefix(filterQuery),
  );
  let listView = $derived(
    trayListView(
      deps.getError(),
      deps.getRepos().length,
      filterQuery,
      flatVisible.length,
    ),
  );

  function moveSelection(delta: number) {
    rawSelectedIndex = moveSelectionIndex(
      clampSelectionIndex(rawSelectedIndex, flatVisible.length),
      delta,
      flatVisible.length,
    );
  }

  function appendTagFilter(tag: string) {
    filterQuery = appendTagToFilterQuery(filterQuery, tag);
  }

  function onTagAutocompleteSelect(tag: string) {
    filterQuery = replaceTrailingTagAutocomplete(filterQuery, tag);
  }

  function getSelectedRepo(): RepoDto | undefined {
    return flatVisible[
      clampSelectionIndex(rawSelectedIndex, flatVisible.length)
    ];
  }

  return {
    get filterQuery() {
      return filterQuery;
    },
    set filterQuery(value: string) {
      if (filterQuery !== value) {
        rawSelectedIndex = 0;
      }
      filterQuery = value;
    },
    get selectedIndex() {
      return clampSelectionIndex(rawSelectedIndex, flatVisible.length);
    },
    set selectedIndex(value: number) {
      rawSelectedIndex = value;
    },
    get sectionedRepos() {
      return sectionedRepos;
    },
    get flatVisible() {
      return flatVisible;
    },
    get flatIndexByPath() {
      return flatIndexByPath;
    },
    get tagAutocompletePrefix() {
      return tagAutocompletePrefix;
    },
    get listView() {
      return listView;
    },
    moveSelection,
    appendTagFilter,
    onTagAutocompleteSelect,
    getSelectedRepo,
  };
}

export type TrayListSelection = ReturnType<typeof createTrayListSelection>;
