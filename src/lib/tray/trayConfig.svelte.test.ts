import { describe, expect, it, vi, beforeEach } from "vitest";
import { DEFAULT_SECTION_CFG } from "$lib/openSelection";
import type { TrayConfigDto } from "$lib/types";
import { DEFAULT_MAX_VISIBLE_ROWS } from "./constants";
import { createTrayConfig } from "./trayConfig.svelte";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

describe("createTrayConfig", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("uses defaults before loadConfig", () => {
    const config = createTrayConfig();
    expect(config.sectionCfg).toEqual(DEFAULT_SECTION_CFG);
    expect(config.listMaxHeightPx).toBe(DEFAULT_MAX_VISIBLE_ROWS * 44 + 52);
  });

  it("loadConfig applies tray config from invoke", async () => {
    const dto: TrayConfigDto = {
      max_visible_rows: 8,
      max_recent_days: 7,
      min_recent_count: 2,
      max_pinned: 5,
      stale_dirty_days: 30,
    };
    invoke.mockResolvedValueOnce(dto);

    const config = createTrayConfig();
    await config.loadConfig();

    expect(invoke).toHaveBeenCalledWith("get_tray_config");
    expect(config.sectionCfg).toEqual({
      maxRecentDays: 7,
      minRecentCount: 2,
    });
    expect(config.listMaxHeightPx).toBe(8 * 44 + 52);
  });

  it("loadConfig keeps defaults when invoke fails", async () => {
    invoke.mockRejectedValueOnce(new Error("ipc down"));
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    const config = createTrayConfig();
    await config.loadConfig();

    expect(config.sectionCfg).toEqual(DEFAULT_SECTION_CFG);
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });
});
