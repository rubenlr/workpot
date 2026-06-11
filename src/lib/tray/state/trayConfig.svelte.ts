import { invoke } from "@tauri-apps/api/core";
import { trayListMaxHeightPx } from "$lib/tray/logic/list/panelLayout";
import { DEFAULT_SECTION_CFG } from "$lib/tray/logic/list/openSelection";
import type { SectionConfig } from "$lib/tray/logic/list/sort";
import type { TrayConfigDto } from "$lib/types";
import { DEFAULT_MAX_VISIBLE_ROWS } from "$lib/tray/logic/handlers/constants";

export function createTrayConfig() {
  let trayConfig = $state<TrayConfigDto | null>(null);

  let maxVisibleRows = $derived(
    trayConfig?.max_visible_rows ?? DEFAULT_MAX_VISIBLE_ROWS,
  );
  let listMaxHeightPx = $derived(trayListMaxHeightPx(maxVisibleRows));
  let sectionCfg = $derived<SectionConfig>({
    maxRecentDays:
      trayConfig?.max_recent_days ?? DEFAULT_SECTION_CFG.maxRecentDays,
    minRecentCount:
      trayConfig?.min_recent_count ?? DEFAULT_SECTION_CFG.minRecentCount,
  });

  async function loadConfig(): Promise<void> {
    try {
      trayConfig = await invoke<TrayConfigDto>("get_tray_config");
    } catch (e) {
      console.warn("get_tray_config failed", e);
    }
  }

  return {
    get sectionCfg() {
      return sectionCfg;
    },
    get listMaxHeightPx() {
      return listMaxHeightPx;
    },
    loadConfig,
  };
}

export type TrayConfig = ReturnType<typeof createTrayConfig>;
