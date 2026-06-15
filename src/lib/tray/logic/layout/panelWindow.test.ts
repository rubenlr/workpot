import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LogicalSize } from "@tauri-apps/api/dpi";

const setSize = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setSize }),
}));

describe("syncPanelWindowHeight", () => {
  beforeEach(() => {
    setSize.mockClear();
    (globalThis as typeof globalThis & { isTauri?: boolean }).isTauri = true;
    vi.resetModules();
  });

  afterEach(() => {
    delete (globalThis as typeof globalThis & { isTauri?: boolean }).isTauri;
  });

  async function loadSync() {
    const mod = await import("./panelWindow");
    return mod.syncPanelWindowHeight;
  }

  it("no_op_outside_tauri_runtime", async () => {
    delete (globalThis as typeof globalThis & { isTauri?: boolean }).isTauri;
    const syncPanelWindowHeight = await loadSync();
    await syncPanelWindowHeight(320);
    expect(setSize).not.toHaveBeenCalled();
  });

  it("invokes_set_size_with_panel_width", async () => {
    const syncPanelWindowHeight = await loadSync();
    await syncPanelWindowHeight(320);
    expect(setSize).toHaveBeenCalledWith(new LogicalSize(400, 320));
  });

  it("dedupes_identical_height", async () => {
    const syncPanelWindowHeight = await loadSync();
    await syncPanelWindowHeight(320);
    await syncPanelWindowHeight(320);
    expect(setSize).toHaveBeenCalledTimes(1);
  });

  it("syncs_when_height_changes", async () => {
    const syncPanelWindowHeight = await loadSync();
    await syncPanelWindowHeight(320);
    await syncPanelWindowHeight(360);
    expect(setSize).toHaveBeenCalledTimes(2);
    expect(setSize).toHaveBeenLastCalledWith(new LogicalSize(400, 360));
  });
});
