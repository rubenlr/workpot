import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import RemoteDetailPane from "./RemoteDetailPane.svelte";
import type { RepoDto } from "$lib/types";

const repo: RepoDto = {
  path: "/Users/me/code/acme/api",
  name: "api",
  alias: null,
  branch: null,
  ahead: null,
  behind: null,
  is_dirty: null,
  parent_dir: "~/code/acme",
  last_opened_at: null,
  git_state_error: null,
  pinned: false,
  pin_order: null,
  notes: null,
  tags: [],
  branches: [],
  is_bare: false,
  convert_to: null,
  convert_block_reason: null,
  cloned: false,
  remote_url: "https://git.example.com/acme/platform/api",
  remote_full_path: "acme/platform/api",
  remote_source: "acme",
  clone_policy: "manual",
};

describe("RemoteDetailPane", () => {
  afterEach(() => cleanup());

  it("shows remote details and clones on click under manual policy", async () => {
    const onClone = vi.fn();
    const { getByText, getByRole } = render(RemoteDetailPane, {
      props: { repo, onClone, onClose: vi.fn() },
    });
    expect(getByText("acme/platform/api")).toBeTruthy();
    expect(getByText("https://git.example.com/acme/platform/api")).toBeTruthy();
    expect(getByText(/Clones to ~\/code\/acme\/api/)).toBeTruthy();
    await fireEvent.click(getByRole("button", { name: "Clone" }));
    expect(onClone).toHaveBeenCalledWith(repo);
  });

  it.each([
    ["manual", /Not cloned/],
    ["on_open", /Opening this repository clones it first/],
    ["auto", /Cloned automatically/],
  ] as const)(
    "explains the %s policy and keeps Clone available",
    (policy, hint) => {
      const { getByText, getByRole } = render(RemoteDetailPane, {
        props: {
          repo: { ...repo, clone_policy: policy },
          onClone: vi.fn(),
          onClose: vi.fn(),
        },
      });
      expect(getByText(hint)).toBeTruthy();
      expect(
        (getByRole("button", { name: "Clone" }) as HTMLButtonElement).disabled,
      ).toBe(false);
    },
  );

  it.each([null, undefined] as const)(
    "falls back to the manual hint when clone_policy is %s",
    (policy) => {
      const { getByText } = render(RemoteDetailPane, {
        props: {
          repo: { ...repo, clone_policy: policy },
          onClone: vi.fn(),
          onClose: vi.fn(),
        },
      });
      expect(getByText(/Not cloned\. Clone it here/)).toBeTruthy();
    },
  );

  it("clones to the repo path when parent_dir is empty", () => {
    const { getByText } = render(RemoteDetailPane, {
      props: {
        repo: { ...repo, parent_dir: "" },
        onClone: vi.fn(),
        onClose: vi.fn(),
      },
    });
    expect(getByText("Clones to /Users/me/code/acme/api")).toBeTruthy();
  });

  it("titles the pane with the alias when set, else the name", () => {
    const aliased = render(RemoteDetailPane, {
      props: { repo: { ...repo, alias: "Platform API" }, onClose: vi.fn() },
    });
    expect(aliased.getByRole("heading", { level: 2 }).textContent?.trim()).toBe(
      "Platform API",
    );
    aliased.unmount();

    const plain = render(RemoteDetailPane, {
      props: { repo, onClose: vi.fn() },
    });
    expect(plain.getByRole("heading", { level: 2 }).textContent?.trim()).toBe(
      "api",
    );
  });

  it("omits the remote path and url lines when they are absent", () => {
    const { queryByText, getByText } = render(RemoteDetailPane, {
      props: {
        repo: { ...repo, remote_full_path: null, remote_url: null },
        onClone: vi.fn(),
        onClose: vi.fn(),
      },
    });
    expect(queryByText("acme/platform/api")).toBeNull();
    expect(queryByText(/https:\/\/git\.example\.com/)).toBeNull();
    expect(getByText(/Clones to/)).toBeTruthy();
  });

  it("disables Clone when no onClone handler is provided", () => {
    const { getByRole } = render(RemoteDetailPane, {
      props: { repo, onClose: vi.fn() },
    });
    expect(
      (getByRole("button", { name: "Clone" }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it("disables Clone while cloning and closes via the back button", async () => {
    const onClose = vi.fn();
    const { getByRole } = render(RemoteDetailPane, {
      props: { repo, cloning: true, onClone: vi.fn(), onClose },
    });
    expect(
      (getByRole("button", { name: /Cloning/ }) as HTMLButtonElement).disabled,
    ).toBe(true);
    await fireEvent.click(getByRole("button", { name: "Close detail pane" }));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
