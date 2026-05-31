import { describe, expect, it } from "vitest";
import {
  clientTagAddError,
  shouldSaveNotes,
  tagAlreadyOnRepo,
} from "./orgClient";

describe("clientTagAddError", () => {
  it("returns null for empty or whitespace-only input", () => {
    expect(clientTagAddError("")).toBeNull();
    expect(clientTagAddError("   ")).toBeNull();
  });

  it("rejects tags starting with #", () => {
    expect(clientTagAddError("#bad")).toBe("Tag cannot start with #");
    expect(clientTagAddError("  #bad")).toBe("Tag cannot start with #");
  });

  it("accepts normal tags", () => {
    expect(clientTagAddError("backend")).toBeNull();
    expect(clientTagAddError("  rust  ")).toBeNull();
  });
});

describe("tagAlreadyOnRepo", () => {
  it("returns false for empty tag", () => {
    expect(tagAlreadyOnRepo("", ["a"])).toBe(false);
    expect(tagAlreadyOnRepo("   ", ["a"])).toBe(false);
  });

  it("detects existing tag case-sensitively", () => {
    expect(tagAlreadyOnRepo("Rust", ["Rust", "go"])).toBe(true);
    expect(tagAlreadyOnRepo("rust", ["Rust"])).toBe(false);
  });

  it("returns false when tag is new", () => {
    expect(tagAlreadyOnRepo("new", ["old"])).toBe(false);
  });
});

describe("shouldSaveNotes", () => {
  it("skips save when unchanged after trim", () => {
    expect(shouldSaveNotes("hello", "hello")).toBe(false);
    expect(shouldSaveNotes("hello   ", "hello")).toBe(false);
  });

  it("saves when text changed", () => {
    expect(shouldSaveNotes("hello world", "hello")).toBe(true);
  });

  it("saves when clearing notes", () => {
    expect(shouldSaveNotes("   ", "had notes")).toBe(true);
  });

  it("saves when adding first note", () => {
    expect(shouldSaveNotes("new", null)).toBe(true);
  });
});
