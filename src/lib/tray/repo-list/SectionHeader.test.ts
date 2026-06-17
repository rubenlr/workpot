import { cleanup, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it } from "vitest";
import SectionHeader from "./SectionHeader.svelte";

describe("SectionHeader", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders label text", () => {
    const { getByText } = render(SectionHeader, { props: { label: "Pinned" } });
    expect(getByText("Pinned")).toBeTruthy();
  });

  it("renders different labels correctly", () => {
    const { getByText } = render(SectionHeader, { props: { label: "Recent" } });
    expect(getByText("Recent")).toBeTruthy();
  });
});
