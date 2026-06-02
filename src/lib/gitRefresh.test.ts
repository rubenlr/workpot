import { describe, expect, it } from "vitest";
import {
  gitRefreshErrorMessage,
  shouldClearListErrorOnRefreshLoad,
} from "./gitRefresh";
import type { GitRefreshSummary } from "./types";

function summary(partial: Partial<GitRefreshSummary>): GitRefreshSummary {
  return {
    refreshed: 0,
    errors: 0,
    any_dirty: false,
    ...partial,
  };
}

describe("gitRefreshErrorMessage", () => {
  it("returns null when refresh succeeds", () => {
    expect(gitRefreshErrorMessage(summary({ refreshed: 3 }))).toBeNull();
  });

  it("reports total failure", () => {
    expect(gitRefreshErrorMessage(summary({ refreshed: 0, errors: 2 }))).toBe(
      "Git refresh failed for all repositories.",
    );
  });

  it("returns null on partial failure (per-repo errors stay on rows)", () => {
    expect(gitRefreshErrorMessage(summary({ refreshed: 1, errors: 1 }))).toBeNull();
  });
});

describe("shouldClearListErrorOnRefreshLoad", () => {
  it("always clears so cached list shows after refresh", () => {
    expect(shouldClearListErrorOnRefreshLoad(summary({ refreshed: 1 }))).toBe(
      true,
    );
    expect(
      shouldClearListErrorOnRefreshLoad(summary({ refreshed: 1, errors: 1 })),
    ).toBe(true);
  });
});
