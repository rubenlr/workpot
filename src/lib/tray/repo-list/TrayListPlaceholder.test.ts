import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import TrayListPlaceholder from "./TrayListPlaceholder.svelte";

describe("TrayListPlaceholder", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders message text", () => {
    const { getByText } = render(TrayListPlaceholder, {
      props: { message: "No repos indexed yet." },
    });
    expect(getByText("No repos indexed yet.")).toBeTruthy();
  });

  it("tone_error_applies_red_class", () => {
    const { container } = render(TrayListPlaceholder, {
      props: { message: "Error loading repos.", tone: "error" },
    });
    const p = container.querySelector("p");
    expect(p?.className).toContain("text-red");
  });

  it("tone_muted_applies_neutral_class", () => {
    const { container } = render(TrayListPlaceholder, {
      props: { message: "No match.", tone: "muted" },
    });
    const p = container.querySelector("p");
    expect(p?.className).toContain("text-neutral");
  });

  it("defaults_to_muted_tone", () => {
    const { container } = render(TrayListPlaceholder, {
      props: { message: "Empty." },
    });
    const p = container.querySelector("p");
    expect(p?.className).toContain("text-neutral");
    expect(p?.className).not.toContain("text-red");
  });
});
