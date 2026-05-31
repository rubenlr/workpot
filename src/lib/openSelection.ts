import { filterAndSectionRepos, flatSectioned } from "./trayList";
import type { SectionConfig } from "./sort";
import type { RepoDto } from "./types";

const DEFAULT_SECTION_CFG: SectionConfig = {
  maxRecentDays: 14,
  minRecentCount: 3,
};

/** Restore list selection after Cmd+Enter background open (D-36). */
export function selectionIndexAfterBackgroundOpen(
  repos: RepoDto[],
  query: string,
  openedPath: string,
  sectionConfig: SectionConfig = DEFAULT_SECTION_CFG,
): number {
  const flat = flatSectioned(
    filterAndSectionRepos(repos, query, sectionConfig),
  );
  const idx = flat.findIndex((r) => r.path === openedPath);
  return idx >= 0 ? idx : 0;
}
