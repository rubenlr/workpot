import { afterEach, describe, expect, it, vi } from "vitest";
import { observePanelHeight } from "./observePanelHeight";

describe("observePanelHeight", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("no_op_when_callback_missing", () => {
    const node = document.createElement("div");
    const action = observePanelHeight(node);
    expect(action.destroy).toBeTypeOf("function");
    action.destroy();
  });

  it("reports_initial_height_via_resize_observer", () => {
    const onHeight = vi.fn();
    const node = document.createElement("div");
    const observe = vi.fn();
    const disconnect = vi.fn();
    const ResizeObserverMock = vi.fn(function (this: ResizeObserver, cb) {
      this.observe = observe;
      this.disconnect = disconnect;
      cb([{ contentRect: { height: 123.6 } } as ResizeObserverEntry], this);
    });
    vi.stubGlobal("ResizeObserver", ResizeObserverMock);

    const action = observePanelHeight(node, onHeight);

    expect(observe).toHaveBeenCalledWith(node);
    expect(onHeight).toHaveBeenCalledWith(124);

    action.destroy();
    expect(disconnect).toHaveBeenCalled();
  });

  it("falls_back_to_bounding_rect_when_resize_observer_unavailable", () => {
    const original = globalThis.ResizeObserver;
    // @ts-expect-error test stub
    delete globalThis.ResizeObserver;

    const onHeight = vi.fn();
    const node = document.createElement("div");
    vi.spyOn(node, "getBoundingClientRect").mockReturnValue({
      height: 88.4,
    } as DOMRect);

    const action = observePanelHeight(node, onHeight);
    expect(onHeight).toHaveBeenCalledWith(88);
    action.destroy();

    globalThis.ResizeObserver = original;
  });
});
