import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TagChip from "./TagChip.svelte";

describe("TagChip", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders_tag_text_with_hash", () => {
    const { getByText } = render(TagChip, { props: { tag: "backend" } });
    expect(getByText("#backend")).toBeTruthy();
  });

  it("remove_button_hidden_when_onRemove_undefined", () => {
    const { queryByLabelText } = render(TagChip, { props: { tag: "rust" } });
    expect(queryByLabelText("Remove tag rust")).toBeNull();
  });

  it("remove_button_shown_when_onRemove_provided", () => {
    const { getByLabelText } = render(TagChip, {
      props: { tag: "rust", onRemove: vi.fn() },
    });
    expect(getByLabelText("Remove tag rust")).toBeTruthy();
  });

  it("plain_click_calls_onFilter_not_onRemove", async () => {
    const onFilter = vi.fn();
    const onRemove = vi.fn();
    const { getByText } = render(TagChip, {
      props: { tag: "rust", onFilter, onRemove },
    });
    await fireEvent.click(getByText("#rust"));
    expect(onFilter).toHaveBeenCalledOnce();
    expect(onRemove).not.toHaveBeenCalled();
  });

  it("cmd_click_calls_onRemove_not_onFilter", async () => {
    const onFilter = vi.fn();
    const onRemove = vi.fn();
    const { getByText } = render(TagChip, {
      props: { tag: "rust", onFilter, onRemove },
    });
    await fireEvent.click(getByText("#rust"), { metaKey: true });
    expect(onRemove).toHaveBeenCalledOnce();
    expect(onFilter).not.toHaveBeenCalled();
  });

  it("remove_button_click_calls_onRemove", async () => {
    const onRemove = vi.fn();
    const { getByLabelText } = render(TagChip, {
      props: { tag: "rust", onRemove },
    });
    await fireEvent.click(getByLabelText("Remove tag rust"));
    expect(onRemove).toHaveBeenCalledOnce();
  });

  it("plain_click_with_no_handlers_does_not_throw", async () => {
    const { getByText } = render(TagChip, { props: { tag: "noop" } });
    await expect(fireEvent.click(getByText("#noop"))).resolves.not.toThrow();
  });
});
