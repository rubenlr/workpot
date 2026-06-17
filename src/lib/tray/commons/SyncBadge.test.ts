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

  it("display-only spans look disabled", () => {
    const { container, queryByRole } = render(SyncBadge, {
      props: { ahead: 2, behind: 1, branch: "main" },
    });
    expect(queryByRole("button")).toBeNull();
    const chips = [...container.querySelectorAll("span")].filter((span) =>
      span.className.includes("rounded-full"),
    );
    expect(chips).toHaveLength(2);
    for (const chip of chips) {
      expect(chip.className).toContain("opacity-70");
      expect(chip.className).toContain("cursor-default");
    }
  });

  it("push syncing renders non-interactive span", async () => {
    const onPush = vi.fn();
    const { container, queryByRole } = render(SyncBadge, {
      props: {
        ahead: 2,
        behind: 0,
        branch: "main",
        syncingDirection: "push",
        onPush,
      },
    });
    expect(queryByRole("button", { name: /Push/ })).toBeNull();
    const chip = container.querySelector("span.animate-pulse");
    expect(chip).not.toBeNull();
    expect(chip!.className).toContain("cursor-default");
    expect(chip!.className).not.toMatch(/hover:/);
    await fireEvent.click(chip!);
    expect(onPush).not.toHaveBeenCalled();
  });

  it("pull syncing renders non-interactive span", async () => {
    const onPull = vi.fn();
    const { container, queryByRole } = render(SyncBadge, {
      props: {
        ahead: 0,
        behind: 3,
        branch: "main",
        syncingDirection: "pull",
        onPull,
      },
    });
    expect(queryByRole("button", { name: /Pull/ })).toBeNull();
    const chip = container.querySelector("span.animate-pulse");
    expect(chip).not.toBeNull();
    expect(chip!.className).toContain("cursor-default");
    expect(chip!.className).not.toMatch(/hover:/);
    await fireEvent.click(chip!);
    expect(onPull).not.toHaveBeenCalled();
  });
});
