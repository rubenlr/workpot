import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type FontsStub = {
  check: ReturnType<typeof vi.fn>;
  load: ReturnType<typeof vi.fn>;
  ready: Promise<unknown>;
};

const originalFonts = Object.getOwnPropertyDescriptor(document, "fonts");

function stubFonts(fonts: FontsStub | undefined): void {
  Object.defineProperty(document, "fonts", {
    value: fonts,
    configurable: true,
  });
}

async function freshModule() {
  vi.resetModules();
  return import("./materialSymbolsFont.svelte");
}

describe("materialSymbolsFont probe", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    if (originalFonts) {
      Object.defineProperty(document, "fonts", originalFonts);
    } else {
      Reflect.deleteProperty(document, "fonts");
    }
  });

  it("stays unavailable when the FontFaceSet API is missing", async () => {
    stubFonts(undefined);
    const mod = await freshModule();

    mod.probeMaterialSymbolsFont();

    expect(mod.materialSymbolsFontAvailable()).toBe(false);
  });

  it("marks available when the face is already loaded and refreshes after load/ready", async () => {
    const check = vi.fn().mockReturnValue(true);
    const load = vi.fn().mockResolvedValue([]);
    stubFonts({ check, load, ready: Promise.resolve() });
    const mod = await freshModule();

    mod.probeMaterialSymbolsFont();
    expect(mod.materialSymbolsFontAvailable()).toBe(true);

    await vi.waitFor(() =>
      expect(check.mock.calls.length).toBeGreaterThanOrEqual(3),
    );
    expect(load).toHaveBeenCalledOnce();
  });

  it("probes only once", async () => {
    const load = vi.fn().mockResolvedValue([]);
    stubFonts({
      check: vi.fn().mockReturnValue(false),
      load,
      ready: Promise.resolve(),
    });
    const mod = await freshModule();

    mod.probeMaterialSymbolsFont();
    mod.probeMaterialSymbolsFont();

    expect(load).toHaveBeenCalledOnce();
  });

  it("falls back to unavailable when loading the face rejects", async () => {
    const check = vi.fn().mockReturnValue(true);
    const load = vi.fn().mockRejectedValue(new Error("network"));
    stubFonts({ check, load, ready: new Promise(() => {}) });
    const mod = await freshModule();

    mod.probeMaterialSymbolsFont();
    expect(mod.materialSymbolsFontAvailable()).toBe(true);

    await vi.waitFor(() =>
      expect(mod.materialSymbolsFontAvailable()).toBe(false),
    );
  });

  it("drops to unavailable if the FontFaceSet disappears before load settles", async () => {
    let settle: () => void = () => {};
    const load = vi
      .fn()
      .mockReturnValue(new Promise<void>((resolve) => (settle = resolve)));
    stubFonts({
      check: vi.fn().mockReturnValue(true),
      load,
      ready: new Promise(() => {}),
    });
    const mod = await freshModule();

    mod.probeMaterialSymbolsFont();
    expect(mod.materialSymbolsFontAvailable()).toBe(true);

    stubFonts(undefined);
    settle();

    await vi.waitFor(() =>
      expect(mod.materialSymbolsFontAvailable()).toBe(false),
    );
  });

  it("resetMaterialSymbolsFontProbeForTests forces state and locks the probe", async () => {
    const load = vi.fn().mockResolvedValue([]);
    stubFonts({
      check: vi.fn().mockReturnValue(false),
      load,
      ready: Promise.resolve(),
    });
    const mod = await freshModule();

    mod.resetMaterialSymbolsFontProbeForTests(true);
    mod.probeMaterialSymbolsFont();

    expect(mod.materialSymbolsFontAvailable()).toBe(true);
    expect(load).not.toHaveBeenCalled();
  });
});
