import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const tauriRoot = dirname(fileURLToPath(import.meta.url));

function readTauriFile(relativePath: string): string {
  return readFileSync(join(tauriRoot, relativePath), "utf8");
}

function parseRegisteredCommands(libRs: string): string[] {
  const match = libRs.match(
    /\.invoke_handler\(tauri::generate_handler!\[([\s\S]*?)\]\)/,
  );
  if (!match) {
    throw new Error("generate_handler block not found in lib.rs");
  }
  return [...match[1].matchAll(/commands::(\w+)/g)].map((m) => m[1]);
}

function parsePermittedCommands(toml: string): Set<string> {
  const allowed = new Set<string>();
  for (const match of toml.matchAll(/commands\.allow\s*=\s*\[([\s\S]*?)\]/g)) {
    for (const cmd of match[1].matchAll(/"([^"]+)"/g)) {
      allowed.add(cmd[1]);
    }
  }
  return allowed;
}

function parsePermissionIdentifiers(toml: string): Set<string> {
  const ids = new Set<string>();
  for (const match of toml.matchAll(/identifier\s*=\s*"([^"]+)"/g)) {
    ids.add(match[1]);
  }
  return ids;
}

function parseCapabilityPermissions(defaultJson: string): string[] {
  const parsed = JSON.parse(defaultJson) as { permissions: string[] };
  return parsed.permissions;
}

describe("tray command capability parity", () => {
  const libRs = readTauriFile("src/lib.rs");
  const trayCommandsToml = readTauriFile("permissions/tray-commands.toml");
  const defaultJson = readTauriFile("capabilities/default.json");

  const registered = parseRegisteredCommands(libRs);
  const permitted = parsePermittedCommands(trayCommandsToml);
  const permissionIds = parsePermissionIdentifiers(trayCommandsToml);
  const capabilityPermissions = parseCapabilityPermissions(defaultJson);

  it("registers at least one tray command", () => {
    expect(registered.length).toBeGreaterThan(0);
  });

  it("permits every command registered in generate_handler", () => {
    const missing = registered.filter((cmd) => !permitted.has(cmd));
    expect(
      missing,
      `missing tray-commands.toml allow entries: ${missing}`,
    ).toEqual([]);
  });

  it("grants every custom tray permission identifier in default.json", () => {
    const customTrayPermissions = capabilityPermissions.filter((p) =>
      p.startsWith("allow-"),
    );
    const missing = customTrayPermissions.filter(
      (id) => !permissionIds.has(id),
    );
    expect(
      missing,
      `default.json references unknown tray permission ids: ${missing}`,
    ).toEqual([]);
  });

  it("registers repo convert IPC commands", () => {
    expect(registered).toContain("convert_repo");
    expect(registered).toContain("get_repo_convert_status");
  });
});
