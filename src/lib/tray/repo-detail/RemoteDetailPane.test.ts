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

  it("explains each policy and keeps Clone available", () => {
    const expectations = {
      manual: /Not cloned/,
      on_open: /Opening this repository clones it first/,
      auto: /Cloned automatically/,
    } as const;
    for (const [policy, hint] of Object.entries(expectations)) {
      const { getByText, getByRole, unmount } = render(RemoteDetailPane, {
        props: {
          repo: { ...repo, clone_policy: policy as RepoDto["clone_policy"] },
          onClone: vi.fn(),
          onClose: vi.fn(),
        },
      });
      expect(getByText(hint)).toBeTruthy();
      expect(
        (getByRole("button", { name: "Clone" }) as HTMLButtonElement).disabled,
      ).toBe(false);
      unmount();
    }
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
