import { afterEach, describe, expect, it, vi } from "vitest";

const theme = vi.fn();
const onThemeChanged = vi.fn();
const unlisten = vi.fn();

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    theme,
    onThemeChanged,
  }),
}));

import { applyDocumentTheme, initSystemThemeSync } from "./syncSystemTheme";

describe("applyDocumentTheme", () => {
  afterEach(() => {
    document.documentElement.removeAttribute("data-theme");
  });

  it("sets data-theme for light and dark", () => {
    applyDocumentTheme("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");

    applyDocumentTheme("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("removes data-theme when following system", () => {
    applyDocumentTheme("dark");
    applyDocumentTheme(null);
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });
});

describe("initSystemThemeSync", () => {
  afterEach(() => {
    delete (globalThis as { isTauri?: boolean }).isTauri;
    document.documentElement.removeAttribute("data-theme");
    theme.mockReset();
    onThemeChanged.mockReset();
    unlisten.mockReset();
  });

  it("returns no-op cleanup outside Tauri", async () => {
    const cleanup = await initSystemThemeSync();

    expect(theme).not.toHaveBeenCalled();
    expect(onThemeChanged).not.toHaveBeenCalled();
    cleanup();
  });

  it("applies initial theme and registers listener in Tauri", async () => {
    (globalThis as { isTauri?: boolean }).isTauri = true;
    theme.mockResolvedValue("dark");
    onThemeChanged.mockResolvedValue(unlisten);

    const cleanup = await initSystemThemeSync();

    expect(theme).toHaveBeenCalled();
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(onThemeChanged).toHaveBeenCalledOnce();

    const callback = onThemeChanged.mock.calls[0][0] as (event: {
      payload: "light" | "dark";
    }) => void;
    callback({ payload: "light" });
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");

    cleanup();
    expect(unlisten).toHaveBeenCalled();
  });
});
