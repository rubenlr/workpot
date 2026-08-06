import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { armSyncWatchdog, clearSyncWatchdog } from "./syncWatchdog";

describe("syncWatchdog", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    clearSyncWatchdog();
    vi.useRealTimers();
  });

  it("fires onTimeout after 90 seconds", () => {
    const onTimeout = vi.fn();
    armSyncWatchdog(onTimeout);

    vi.advanceTimersByTime(89_999);
    expect(onTimeout).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onTimeout).toHaveBeenCalledOnce();
  });

  it("clearSyncWatchdog cancels pending timeout", () => {
    const onTimeout = vi.fn();
    armSyncWatchdog(onTimeout);
    clearSyncWatchdog();

    vi.advanceTimersByTime(90_000);
    expect(onTimeout).not.toHaveBeenCalled();
  });

  it("re-arm replaces previous watchdog", () => {
    const first = vi.fn();
    const second = vi.fn();
    armSyncWatchdog(first);
    vi.advanceTimersByTime(30_000);
    armSyncWatchdog(second);

    vi.advanceTimersByTime(89_999);
    expect(first).not.toHaveBeenCalled();
    expect(second).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledOnce();
  });
});
