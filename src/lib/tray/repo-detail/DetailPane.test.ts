import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import DetailPane from "./DetailPane.svelte";
import type { BranchListItemDto, RepoDto } from "$lib/types";

const hide = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ hide }),
}));

const invokeMock = vi.mocked(invoke);

const baseRepo: RepoDto = {
  path: "/tmp/testrepo",
  name: "testrepo",
  alias: null,
  branch: "main",
  ahead: null,
  behind: null,
  is_dirty: false,
  parent_dir: "~/tmp",
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
};

const sampleBranch: BranchListItemDto = {
  name: "feature",
  presence: "local_remote",
  ahead: null,
  behind: null,
};

function notesTextarea(container: HTMLElement): HTMLTextAreaElement {
  const el = container.querySelector(
    'textarea[placeholder="Add notes..."]',
  ) as HTMLTextAreaElement;
  expect(el).toBeTruthy();
  return el;
}

function tagInput(container: HTMLElement): HTMLInputElement {
  const el = container.querySelector(
    'input[placeholder="Add tag…"]',
  ) as HTMLInputElement;
  expect(el).toBeTruthy();
  return el;
}

function aliasInput(container: HTMLElement): HTMLInputElement {
  const el = container.querySelector(
    'input[placeholder="Display name…"]',
  ) as HTMLInputElement;
  expect(el).toBeTruthy();
  return el;
}

function renderPane(
  repo: RepoDto,
  opts: {
    allTags?: string[];
    onClose?: () => void;
    onMutated?: () => void;
    branchRevision?: number;
    requestTagFocus?: boolean;
    onTagFocusDone?: () => void;
  } = {},
) {
  const onClose = opts.onClose ?? vi.fn();
  const onMutated = opts.onMutated ?? vi.fn();
  const onTagFocusDone = opts.onTagFocusDone ?? vi.fn();
  const result = render(DetailPane, {
    props: {
      repo,
      allTags: opts.allTags ?? [],
      onClose,
      onMutated,
      branchRevision: opts.branchRevision ?? 0,
      requestTagFocus: opts.requestTagFocus ?? false,
      onTagFocusDone,
    },
  });
  return { ...result, onClose, onMutated, onTagFocusDone };
}

describe("DetailPane", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue([]);
    hide.mockClear();
  });

  afterEach(() => {
    cleanup();
  });

  it("header_shows_alias_when_set", () => {
    const { getByRole } = renderPane({
      ...baseRepo,
      name: "folder",
      alias: "myalias",
    });
    expect(getByRole("heading", { level: 2 }).textContent).toBe("myalias");
  });

  it("pin_toggle_shows_pinned_icon_and_aria_pressed", () => {
    const pinned = renderPane({ ...baseRepo, pinned: true });
    const pinnedBtn = pinned.getByRole("button", { name: "Unpin" });
    expect(pinnedBtn.getAttribute("aria-pressed")).toBe("true");
    expect(pinnedBtn.className).toContain("text-primary-accent");

    cleanup();

    const unpinned = renderPane({ ...baseRepo, pinned: false });
    const unpinnedBtn = unpinned.getByRole("button", { name: "Pin" });
    expect(unpinnedBtn.getAttribute("aria-pressed")).toBe("false");
    expect(unpinnedBtn.className).toContain("text-inverse-on-surface-variant");
  });

  it("tag_input_has_os_correction_disabled", () => {
    const { container } = renderPane(baseRepo);
    const input = tagInput(container);
    expect(input.getAttribute("autocomplete")).toBe("off");
    expect(input.getAttribute("autocapitalize")).toBe("off");
    expect(input.getAttribute("autocorrect")).toBe("off");
    expect(input.getAttribute("spellcheck")).toBe("false");
  });

  it("notes_textarea_has_os_correction_disabled", () => {
    const { container } = renderPane(baseRepo);
    const notes = notesTextarea(container);
    expect(notes.getAttribute("autocomplete")).toBe("off");
    expect(notes.getAttribute("autocapitalize")).toBe("off");
    expect(notes.getAttribute("spellcheck")).toBe("false");
  });

  it("tag_suggestions_exclude_tags_already_on_repo", async () => {
    const { container } = renderPane(
      { ...baseRepo, tags: ["backend"] },
      { allTags: ["backend", "frontend", "rust"] },
    );
    const input = tagInput(container);
    await fireEvent.input(input, { target: { value: "fr" } });
    await waitFor(() => {
      expect(container.querySelector('[role="option"]')).toBeTruthy();
    });
    const options = [...container.querySelectorAll('[role="option"]')].map(
      (el) => el.textContent,
    );
    expect(options).toContain("#frontend");
    expect(options).not.toContain("#backend");
  });

  it("notes_resyncs_from_repo_on_path_change", async () => {
    const { container, rerender, onClose, onMutated } = renderPane({
      ...baseRepo,
      notes: "A",
    });
    expect(notesTextarea(container).value).toBe("A");

    await rerender({
      repo: { ...baseRepo, path: "/tmp/other", notes: "B" },
      allTags: [],
      onClose,
      onMutated,
      branchRevision: 0,
      requestTagFocus: false,
      onTagFocusDone: vi.fn(),
    });
    expect(notesTextarea(container).value).toBe("B");
  });

  it("notes_not_overwritten_while_textarea_focused", async () => {
    const { container, rerender, onClose, onMutated } = renderPane({
      ...baseRepo,
      notes: "original",
    });
    const notes = notesTextarea(container);
    notes.focus();
    await fireEvent.input(notes, { target: { value: "edited locally" } });

    await rerender({
      repo: { ...baseRepo, notes: "server updated" },
      allTags: [],
      onClose,
      onMutated,
      branchRevision: 0,
      requestTagFocus: false,
      onTagFocusDone: vi.fn(),
    });
    expect(notesTextarea(container).value).toBe("edited locally");
  });

  it("notes_resyncs_after_blur_when_repo_changed", async () => {
    const { container, rerender, onClose, onMutated } = renderPane({
      ...baseRepo,
      notes: "A",
    });
    const notes = notesTextarea(container);
    notes.focus();
    await fireEvent.input(notes, { target: { value: "edited" } });
    await fireEvent.blur(notes);
    const away = document.createElement("button");
    document.body.appendChild(away);
    away.focus();

    await rerender({
      repo: { ...baseRepo, path: "/tmp/changed", notes: "C" },
      allTags: [],
      onClose,
      onMutated,
      branchRevision: 0,
      requestTagFocus: false,
      onTagFocusDone: vi.fn(),
    });
    expect(notesTextarea(container).value).toBe("C");
  });

  it("branch_list_loads_on_mount_and_reloads_on_branchRevision", async () => {
    const branches: BranchListItemDto[] = [sampleBranch];
    invokeMock.mockResolvedValue(branches);

    const { rerender, onClose, onMutated } = renderPane(baseRepo);
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("list_branches", {
        repoPath: baseRepo.path,
      });
    });
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await rerender({
      repo: baseRepo,
      allTags: [],
      onClose,
      onMutated,
      branchRevision: 1,
      requestTagFocus: false,
      onTagFocusDone: vi.fn(),
    });
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledTimes(2);
    });
  });

  it("branch_load_ignores_stale_response_after_repo_change", async () => {
    let resolveSlow!: (value: BranchListItemDto[]) => void;
    const slowBranches: BranchListItemDto[] = [
      { name: "stale", presence: "local_remote", ahead: null, behind: null },
    ];
    const fastBranches: BranchListItemDto[] = [
      {
        name: "fast-main",
        presence: "local_remote",
        ahead: null,
        behind: null,
      },
    ];
    const slowRepo = { ...baseRepo, path: "/tmp/slow" };
    const fastRepo = { ...baseRepo, path: "/tmp/fast" };

    invokeMock.mockImplementation((cmd, args) => {
      if (cmd !== "list_branches") {
        return Promise.resolve([]);
      }
      const repoPath = (args as { repoPath: string }).repoPath;
      if (repoPath === slowRepo.path) {
        return new Promise<BranchListItemDto[]>((resolve) => {
          resolveSlow = resolve;
        });
      }
      if (repoPath === fastRepo.path) {
        return Promise.resolve(fastBranches);
      }
      return Promise.resolve([]);
    });

    const { getByRole, rerender, onClose, onMutated } = renderPane(slowRepo);
    await rerender({
      repo: fastRepo,
      allTags: [],
      onClose,
      onMutated,
      branchRevision: 0,
      requestTagFocus: false,
      onTagFocusDone: vi.fn(),
    });

    await waitFor(() => {
      expect(
        getByRole("button", { name: "Activate branch fast-main" }),
      ).toBeTruthy();
    });

    resolveSlow(slowBranches);
    await waitFor(() => {
      expect(
        getByRole("button", { name: "Activate branch fast-main" }),
      ).toBeTruthy();
    });
    expect(() =>
      getByRole("button", { name: "Activate branch stale" }),
    ).toThrow();
  });

  it("branch_load_error_shows_branchError", async () => {
    invokeMock.mockRejectedValue(new Error("branch list failed"));
    const { getByText } = renderPane(baseRepo);
    await waitFor(() => {
      expect(getByText("Error: branch list failed")).toBeTruthy();
    });
  });

  it("tag_add_valid_invokes_and_clears_input", async () => {
    const onMutated = vi.fn();
    const { container } = renderPane(baseRepo, { onMutated });
    const input = tagInput(container);
    await fireEvent.input(input, { target: { value: "newtag" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("add_tag", {
        repoPath: baseRepo.path,
        tag: "newtag",
      });
    });
    expect(onMutated).toHaveBeenCalled();
    expect(tagInput(container).value).toBe("");
  });

  it("tag_add_duplicate_shows_tagError_without_invoke", async () => {
    const { container, getByText } = renderPane({
      ...baseRepo,
      tags: ["existing"],
    });
    const input = tagInput(container);
    await fireEvent.input(input, { target: { value: "existing" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(getByText("Tag already on this repo")).toBeTruthy();
    });
    expect(invokeMock).not.toHaveBeenCalledWith("add_tag", expect.anything());
  });

  it("tag_add_leading_hash_shows_client_error", async () => {
    const { container, getByText } = renderPane(baseRepo);
    const input = tagInput(container);
    await fireEvent.input(input, { target: { value: "#bad" } });
    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(getByText("Tag cannot start with #")).toBeTruthy();
    });
    expect(invokeMock).not.toHaveBeenCalledWith("add_tag", expect.anything());
  });

  it("checkout_failure_shows_checkoutError", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "list_branches") return [sampleBranch];
      if (cmd === "checkout_repo_branch") {
        throw new Error("checkout denied");
      }
      return undefined;
    });

    const { getByRole, getByText } = renderPane(baseRepo);
    await waitFor(() => {
      expect(
        getByRole("button", { name: "Activate branch feature" }),
      ).toBeTruthy();
    });

    await fireEvent.click(
      getByRole("button", { name: "Activate branch feature" }),
    );

    await waitFor(() => {
      expect(getByText("Error: checkout denied")).toBeTruthy();
    });
  });

  it("pin_toggle_invokes_set_pin_and_onMutated", async () => {
    const onMutated = vi.fn();
    const { getByRole } = renderPane(
      { ...baseRepo, pinned: false },
      { onMutated },
    );
    await fireEvent.click(getByRole("button", { name: "Pin" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_pin", {
        repoPath: baseRepo.path,
        pinned: true,
      });
    });
    expect(onMutated).toHaveBeenCalled();
  });

  it("notes_blur_skips_invoke_when_unchanged", async () => {
    const { container } = renderPane({ ...baseRepo, notes: "hello" });
    const notes = notesTextarea(container);
    await fireEvent.blur(notes);
    await waitFor(() => {
      expect(invokeMock).not.toHaveBeenCalledWith(
        "set_notes",
        expect.anything(),
      );
    });
  });

  it("requestTagFocus_focuses_tag_input_and_calls_onTagFocusDone", async () => {
    const onTagFocusDone = vi.fn();
    const focusSpy = vi.spyOn(HTMLInputElement.prototype, "focus");
    renderPane(baseRepo, { requestTagFocus: true, onTagFocusDone });

    await waitFor(() => {
      expect(focusSpy).toHaveBeenCalled();
      expect(onTagFocusDone).toHaveBeenCalled();
    });
    focusSpy.mockRestore();
  });

  it("displays_local_path_with_home_replaced_by_tilde", () => {
    const { getByText } = renderPane({
      ...baseRepo,
      path: "/Users/me/c/myrepo",
      parent_dir: "~/c",
    });
    expect(getByText("~/c/myrepo")).toBeTruthy();
  });

  it("invokes open_in_finder when finder badge is clicked", async () => {
    const { getByRole } = renderPane(baseRepo);
    const finderBtn = getByRole("button", { name: "finder" });
    await fireEvent.click(finderBtn);
    expect(invokeMock).toHaveBeenCalledWith("open_in_finder", {
      path: baseRepo.path,
    });
  });

  it("alias_input_shows_current_alias", () => {
    const { container } = renderPane({ ...baseRepo, alias: "my-alias" });
    expect(aliasInput(container).value).toBe("my-alias");
  });

  it("alias_blur_invokes_set_alias_and_onMutated", async () => {
    const onMutated = vi.fn();
    const { container } = renderPane(
      { ...baseRepo, alias: null },
      { onMutated },
    );
    const input = aliasInput(container);
    await fireEvent.input(input, { target: { value: "new-name" } });
    await fireEvent.blur(input);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_alias", {
        repoPath: baseRepo.path,
        alias: "new-name",
      });
    });
    expect(onMutated).toHaveBeenCalled();
  });

  it("alias_blur_skips_invoke_when_unchanged", async () => {
    const { container } = renderPane({ ...baseRepo, alias: "same" });
    const input = aliasInput(container);
    await fireEvent.blur(input);
    await waitFor(() => {
      expect(invokeMock).not.toHaveBeenCalledWith(
        "set_alias",
        expect.anything(),
      );
    });
  });
});
