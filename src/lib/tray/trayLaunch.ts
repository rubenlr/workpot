import { selectionIndexAfterBackgroundOpen } from "$lib/openSelection";
import type { SectionConfig } from "$lib/sort";
import type { RepoDto } from "$lib/types";

/** Selection index after Cmd+Enter background open (testable without runes). */
export function computeBackgroundOpenSelection(
  repos: RepoDto[],
  query: string,
  openedPath: string,
  sectionCfg: SectionConfig,
): number {
  return selectionIndexAfterBackgroundOpen(
    repos,
    query,
    openedPath,
    sectionCfg,
  );
}
