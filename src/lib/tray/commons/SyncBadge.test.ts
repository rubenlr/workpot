import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import SyncBadge from "./SyncBadge.svelte";

describe("SyncBadge", () => {
  afterEach(() => {
    cleanup();
  });

  it("clicking push chip fires onPush", async () => {
    const onPush = vi.fn();
    const { getByRole } = render(SyncBadge, {
      props: {
        ahead: 2,
        behind: 0,
        branch: "main",
        onPush,
      },
    });
    await fireEvent.click(
      getByRole("button", { name: /Push 2 commits on main/ }),
    );
    expect(onPush).toHaveBeenCalledOnce();
  });

  it("clicking pull chip fires onPull", async () => {
    const onPull = vi.fn();
    const { getByRole } = render(SyncBadge, {
      props: {
        ahead: 0,
        behind: 3,
        branch: "feature/x",
        onPull,
      },
    });
    await fireEvent.click(
      getByRole("button", { name: /Pull 3 commits on feature\/x/ }),
    );
    expect(onPull).toHaveBeenCalledOnce();
  });

  it("disabled prevents push click", async () => {
    const onPush = vi.fn();
    const { getByRole } = render(SyncBadge, {
      props: {
        ahead: 1,
        behind: 0,
        branch: "main",
        disabled: true,
        onPush,
      },
    });
    const btn = getByRole("button", { name: /Push 1 commit on main/ });
    expect(btn).toHaveProperty("disabled", true);
    await fireEvent.click(btn);
    expect(onPush).not.toHaveBeenCalled();
  });

  it("display only when no handlers", () => {
    const { queryByRole } = render(SyncBadge, {
      props: { ahead: 2, behind: 1, branch: "main" },
    });
    expect(queryByRole("button")).toBeNull();
  });
});
