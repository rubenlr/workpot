import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  armGitRefreshWatchdog,
  clearGitRefreshWatchdog,
} from "./gitRefreshWatchdog";

describe("gitRefreshWatchdog", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    clearGitRefreshWatchdog();
    vi.useRealTimers();
  });

  it("fires onTimeout after 90 seconds", () => {
    const onTimeout = vi.fn();
    armGitRefreshWatchdog(onTimeout);

    vi.advanceTimersByTime(89_999);
    expect(onTimeout).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onTimeout).toHaveBeenCalledOnce();
  });

  it("clearGitRefreshWatchdog cancels pending timeout", () => {
    const onTimeout = vi.fn();
    armGitRefreshWatchdog(onTimeout);
    clearGitRefreshWatchdog();

    vi.advanceTimersByTime(90_000);
    expect(onTimeout).not.toHaveBeenCalled();
  });

  it("re-arm replaces previous watchdog", () => {
    const first = vi.fn();
    const second = vi.fn();
    armGitRefreshWatchdog(first);
    vi.advanceTimersByTime(30_000);
    armGitRefreshWatchdog(second);

    vi.advanceTimersByTime(89_999);
    expect(first).not.toHaveBeenCalled();
    expect(second).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledOnce();
  });
});
