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
    onRefresh?: () => void;
    refreshing?: boolean;
    refreshSuccess?: boolean;
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
      onRefresh: opts.onRefresh,
      refreshing: opts.refreshing ?? false,
      refreshSuccess: opts.refreshSuccess ?? false,
    },
  });
}

function refreshIcon(container: HTMLElement): HTMLElement | null {
  return container.querySelector('button[aria-label="Refresh index"] span');
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
    expect(queryByRole("listbox")).toBeNull();
  });

  it("autocomplete_visible_when_query_contains_hash", () => {
    const { getByRole } = renderBar({
      filterQuery: "#rust",
      allTags: ["rust"],
    });
    expect(getByRole("listbox")).toBeTruthy();
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

  it("refresh_button_shows_spin_while_refreshing", () => {
    const { container } = renderBar({
      onRefresh: vi.fn(),
      refreshing: true,
      refreshSuccess: false,
    });
    const icon = refreshIcon(container);
    expect(icon?.textContent).toBe("sync");
    expect(icon?.className).toContain("animate-spin");
  });

  it("refresh_button_shows_check_on_success", () => {
    const { container } = renderBar({
      onRefresh: vi.fn(),
      refreshing: false,
      refreshSuccess: true,
    });
    const icon = refreshIcon(container);
    expect(icon?.textContent).toBe("check");
    expect(icon?.className).not.toContain("animate-spin");
  });

  it("refresh_button_disabled_while_refreshing", () => {
    const { getByRole } = renderBar({
      onRefresh: vi.fn(),
      refreshing: true,
    });
    expect(
      getByRole("button", { name: "Refresh index" }).hasAttribute("disabled"),
    ).toBe(true);
  });
});
