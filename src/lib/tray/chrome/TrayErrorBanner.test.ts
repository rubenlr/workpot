import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import TrayErrorBanner from "./TrayErrorBanner.svelte";

describe("TrayErrorBanner", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders message text", () => {
    const { getByText } = render(TrayErrorBanner, {
      props: { message: "Failed to open Cursor", onDismiss: vi.fn() },
    });
    expect(getByText("Failed to open Cursor")).toBeTruthy();
  });

  it("has role alert", () => {
    const { getByRole } = render(TrayErrorBanner, {
      props: { message: "Error", onDismiss: vi.fn() },
    });
    expect(getByRole("alert")).toBeTruthy();
  });

  it("dismiss_button_calls_onDismiss", async () => {
    const onDismiss = vi.fn();
    const { getByText } = render(TrayErrorBanner, {
      props: { message: "Error", onDismiss },
    });
    await fireEvent.click(getByText("DISMISS"));
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it("hides_dismiss_button_when_onDismiss_omitted", () => {
    const { queryByText } = render(TrayErrorBanner, {
      props: { message: "List load failed" },
    });
    expect(queryByText("DISMISS")).toBeNull();
  });

  it("renders_long_message_without_truncating", () => {
    const long = "A".repeat(200);
    const { getByText } = render(TrayErrorBanner, {
      props: { message: long, onDismiss: vi.fn() },
    });
    expect(getByText(long)).toBeTruthy();
  });
});
