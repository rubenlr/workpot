import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayFilterBar from "./TrayFilterBar.svelte";

function renderBar(
  opts: {
    filterQuery?: string;
    allTags?: string[];
    tagAutocompletePrefix?: string;
    onFilterKeydown?: (e: KeyboardEvent) => void;
    onTagSelect?: (tag: string) => void;
    bindFilterInput?: (el: HTMLInputElement | null) => void;
  } = {},
) {
  return render(TrayFilterBar, {
    props: {
      filterQuery: opts.filterQuery ?? "",
      allTags: opts.allTags ?? [],
      tagAutocompletePrefix: opts.tagAutocompletePrefix ?? "",
      onFilterKeydown: opts.onFilterKeydown ?? vi.fn(),
      onTagSelect: opts.onTagSelect ?? vi.fn(),
      bindFilterInput: opts.bindFilterInput ?? vi.fn(),
    },
  });
}

describe("TrayFilterBar", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders filter input with placeholder", () => {
    const { getByPlaceholderText } = renderBar();
    expect(getByPlaceholderText("Filter repos…")).toBeTruthy();
  });

  it("input_is_type_search", () => {
    const { container } = renderBar();
    const input = container.querySelector("input");
    expect(input?.type).toBe("search");
  });

  it("autocomplete_hidden_when_query_has_no_hash", () => {
    const { queryByRole } = renderBar({ filterQuery: "workpot" });
    expect(queryByRole("list")).toBeNull();
  });

  it("autocomplete_visible_when_query_contains_hash", () => {
    const { getByRole } = renderBar({
      filterQuery: "#rust",
      allTags: ["rust"],
    });
    expect(getByRole("list")).toBeTruthy();
  });

  it("keydown_on_input_calls_onFilterKeydown", async () => {
    const onFilterKeydown = vi.fn();
    const { container } = renderBar({ onFilterKeydown });
    const input = container.querySelector("input") as HTMLInputElement;
    await fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(onFilterKeydown).toHaveBeenCalledOnce();
  });

  it("bindFilterInput_called_with_input_element_on_mount", () => {
    const bindFilterInput = vi.fn();
    renderBar({ bindFilterInput });
    expect(bindFilterInput).toHaveBeenCalled();
  });
});
