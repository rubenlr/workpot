import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";
import DetailPane from "./DetailPane.svelte";
import type { RepoDto } from "$lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

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
};

function renderPane(
  repo: RepoDto,
  opts: {
    allTags?: string[];
    onClose?: () => void;
    onMutated?: () => void;
  } = {},
) {
  const onClose = opts.onClose ?? vi.fn();
  const onMutated = opts.onMutated ?? vi.fn();
  const result = render(DetailPane, {
    props: {
      repo,
      allTags: opts.allTags ?? [],
      onClose,
      onMutated,
    },
  });
  return { ...result, onClose, onMutated };
}

describe("DetailPane", () => {
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
    expect(pinnedBtn.className).toContain("text-primary");

    cleanup();

    const unpinned = renderPane({ ...baseRepo, pinned: false });
    const unpinnedBtn = unpinned.getByRole("button", { name: "Pin" });
    expect(unpinnedBtn.getAttribute("aria-pressed")).toBe("false");
    expect(unpinnedBtn.className).toContain("text-inverse-on-surface-variant");
  });

  it("tag_input_has_os_correction_disabled", () => {
    const { container } = renderPane(baseRepo);
    const input = container.querySelector(
      'input[placeholder="Add tag…"]',
    ) as HTMLInputElement;
    expect(input).toBeTruthy();
    expect(input.getAttribute("autocomplete")).toBe("off");
    expect(input.getAttribute("autocapitalize")).toBe("off");
    expect(input.getAttribute("autocorrect")).toBe("off");
    expect(input.getAttribute("spellcheck")).toBe("false");
  });

  it("notes_textarea_has_os_correction_disabled", () => {
    const { container } = renderPane(baseRepo);
    const notes = container.querySelector(
      'textarea[placeholder="Add notes..."]',
    ) as HTMLTextAreaElement;
    expect(notes).toBeTruthy();
    expect(notes.getAttribute("autocomplete")).toBe("off");
    expect(notes.getAttribute("autocapitalize")).toBe("off");
    expect(notes.getAttribute("spellcheck")).toBe("false");
  });

  it("tag_suggestions_exclude_tags_already_on_repo", async () => {
    const { container } = renderPane(
      { ...baseRepo, tags: ["backend"] },
      { allTags: ["backend", "frontend", "rust"] },
    );
    const input = container.querySelector(
      'input[placeholder="Add tag…"]',
    ) as HTMLInputElement;
    expect(input).toBeTruthy();
    await fireEvent.input(input, { target: { value: "fr" } });
    await waitFor(() => {
      expect(container.querySelector('button[role="option"]')).toBeTruthy();
    });
    const options = [
      ...container.querySelectorAll('button[role="option"]'),
    ].map((el) => el.textContent);
    expect(options).toContain("#frontend");
    expect(options).not.toContain("#backend");
  });
});
