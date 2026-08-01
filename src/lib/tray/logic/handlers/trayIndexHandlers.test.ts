import { describe, expect, it, vi } from "vitest";
import {
  indexGitErrorMessage,
  onIndexComplete,
  onIndexFailed,
} from "./trayIndexHandlers";

describe("trayIndexHandlers", () => {
  it("indexGitErrorMessage surfaces total git failure", () => {
    expect(
      indexGitErrorMessage({
        added: 0,
        removed: 0,
        skipped: 0,
        git_refreshed: 0,
        git_errors: 2,
      }),
    ).toContain("failed");
    expect(
      indexGitErrorMessage({
        added: 1,
        removed: 0,
        skipped: 0,
        git_refreshed: 2,
        git_errors: 1,
      }),
    ).toBeNull();
  });

  it("onIndexComplete resets selection and refreshes", async () => {
    const setSelectedIndex = vi.fn();
    const refresh = vi.fn().mockResolvedValue(undefined);
    const resyncDetail = vi.fn();
    const setError = vi.fn();

    onIndexComplete(
      {
        added: 1,
        removed: 0,
        skipped: 0,
        git_refreshed: 1,
        git_errors: 0,
      },
      { setSelectedIndex, refresh, resyncDetail, setError },
    );

    expect(setSelectedIndex).toHaveBeenCalledWith(0);
    await refresh.mock.results[0]?.value;
    expect(resyncDetail).toHaveBeenCalledOnce();
    expect(setError).toHaveBeenCalledWith(null);
  });

  it("onIndexFailed sets error", () => {
    const setError = vi.fn();
    onIndexFailed("boom", { setError });
    expect(setError).toHaveBeenCalledWith("boom");
  });
});
