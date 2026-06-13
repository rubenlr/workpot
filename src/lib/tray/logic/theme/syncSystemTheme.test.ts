import { afterEach, describe, expect, it } from "vitest";
import { applyDocumentTheme } from "./syncSystemTheme";

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
