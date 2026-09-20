import { render } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import MaterialIcon from "./MaterialIcon.svelte";
import { resetMaterialSymbolsFontProbeForTests } from "./materialSymbolsFont.svelte";

describe("MaterialIcon", () => {
  afterEach(() => {
    resetMaterialSymbolsFontProbeForTests(false);
  });

  it("renders UTF-8 fallback when Material Symbols is unavailable", () => {
    resetMaterialSymbolsFontProbeForTests(false);
    const { container } = render(MaterialIcon, {
      props: { name: "chevron_right", size: 20 },
    });
    const el = container.querySelector(".material-icon-fallback");
    expect(el?.textContent).toBe("›");
    expect(container.querySelector(".material-symbols-outlined")).toBeNull();
  });

  it("renders ligature name when Material Symbols is available", () => {
    resetMaterialSymbolsFontProbeForTests(true);
    const { container } = render(MaterialIcon, {
      props: { name: "chevron_right", size: 20 },
    });
    const el = container.querySelector(".material-symbols-outlined");
    expect(el?.textContent).toBe("chevron_right");
    expect(container.querySelector(".material-icon-fallback")).toBeNull();
  });

  it("uses filled-aware pin fallback", () => {
    resetMaterialSymbolsFontProbeForTests(false);
    const { container } = render(MaterialIcon, {
      props: { name: "keep", filled: true },
    });
    expect(
      container.querySelector(".material-icon-fallback")?.textContent,
    ).toBe("📌");
  });
});
