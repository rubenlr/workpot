import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TagAutocomplete from "./TagAutocomplete.svelte";

function renderAutocomplete(opts: {
  allTags?: string[];
  visible?: boolean;
  prefix?: string;
  onSelect?: (tag: string) => void;
}) {
  const onSelect = opts.onSelect ?? vi.fn();
  return {
    ...render(TagAutocomplete, {
      props: {
        allTags: opts.allTags ?? ["rust", "frontend", "backend"],
        visible: opts.visible ?? true,
        prefix: opts.prefix ?? "",
        onSelect,
      },
    }),
    onSelect,
  };
}

describe("TagAutocomplete", () => {
  afterEach(() => {
    cleanup();
  });

  it("hidden_when_visible_false", () => {
    const { queryByRole } = renderAutocomplete({ visible: false });
    expect(queryByRole("listbox")).toBeNull();
  });

  it("shown_when_visible_true", () => {
    const { getByRole } = renderAutocomplete({ visible: true });
    expect(getByRole("listbox")).toBeTruthy();
  });

  it("shows_all_tags_when_filter_empty", () => {
    const { getAllByRole } = renderAutocomplete({
      allTags: ["rust", "frontend"],
      visible: true,
    });
    const options = getAllByRole("option");
    expect(options.length).toBe(2);
  });

  it("filters_tags_by_input_value", async () => {
    const { container, getAllByRole } = renderAutocomplete({
      allTags: ["rust", "frontend", "backend"],
      visible: true,
    });
    const input = container.querySelector("input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "front" } });
    await waitFor(() => {
      const options = getAllByRole("option");
      expect(options.length).toBe(1);
      expect(options[0].textContent).toContain("frontend");
    });
  });

  it("click_on_option_calls_onSelect", async () => {
    const onSelect = vi.fn();
    const { getAllByRole } = renderAutocomplete({
      allTags: ["rust"],
      visible: true,
      onSelect,
    });
    const [option] = getAllByRole("option");
    await fireEvent.click(option!);
    expect(onSelect).toHaveBeenCalledWith("rust");
  });

  it("enter_key_selects_highlighted_option", async () => {
    const onSelect = vi.fn();
    const { getByRole } = renderAutocomplete({
      allTags: ["rust", "frontend"],
      visible: true,
      onSelect,
    });
    const listbox = getByRole("listbox");
    await fireEvent.keyDown(listbox, { key: "ArrowDown" });
    await fireEvent.keyDown(listbox, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("rust");
  });

  it("arrow_down_moves_highlight_forward", async () => {
    const { getByRole, getAllByRole } = renderAutocomplete({
      allTags: ["rust", "frontend"],
      visible: true,
    });
    const listbox = getByRole("listbox");
    await fireEvent.keyDown(listbox, { key: "ArrowDown" });
    await fireEvent.keyDown(listbox, { key: "ArrowDown" });
    const options = getAllByRole("option");
    expect(options[1]?.getAttribute("aria-selected")).toBe("true");
  });

  it("arrow_up_does_not_go_below_zero", async () => {
    const { getByRole, getAllByRole } = renderAutocomplete({
      allTags: ["rust", "frontend"],
      visible: true,
    });
    const listbox = getByRole("listbox");
    await fireEvent.keyDown(listbox, { key: "ArrowDown" });
    await fireEvent.keyDown(listbox, { key: "ArrowUp" });
    const options = getAllByRole("option");
    expect(options[0]?.getAttribute("aria-selected")).toBe("true");
  });

  it("enter_with_no_highlight_submits_input_value", async () => {
    const onSelect = vi.fn();
    const { container, getByRole } = renderAutocomplete({
      allTags: ["rust"],
      visible: true,
      onSelect,
    });
    const input = container.querySelector("input") as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "newTag" } });
    const listbox = getByRole("listbox");
    await fireEvent.keyDown(listbox, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledWith("newTag");
  });

  it("prefix_filters_to_tags_starting_with_prefix", () => {
    const { queryByText } = renderAutocomplete({
      allTags: ["rust", "frontend", "react"],
      visible: true,
      prefix: "re",
    });
    // only "react" starts with "re"
    expect(queryByText("#react")).toBeTruthy();
    expect(queryByText("#rust")).toBeNull();
    expect(queryByText("#frontend")).toBeNull();
  });
});
