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

/** Each `[[permission]]` block: its identifier and the commands it allows. */
function parsePermissionBlocks(
  toml: string,
): { identifier: string; commands: string[] }[] {
  return toml
    .split(/^\[\[permission\]\]\s*$/m)
    .slice(1)
    .map((block) => {
      const identifier = block.match(/identifier\s*=\s*"([^"]+)"/)?.[1];
      if (!identifier) {
        throw new Error(`permission block without identifier:\n${block}`);
      }
      const allow =
        block.match(/commands\.allow\s*=\s*\[([\s\S]*?)\]/)?.[1] ?? "";
      const commands = [...allow.matchAll(/"([^"]+)"/g)].map((m) => m[1]);
      return { identifier, commands };
    });
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
  const permissionBlocks = parsePermissionBlocks(trayCommandsToml);

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

  it("references only known tray permission identifiers in default.json", () => {
    const customTrayPermissions = capabilityPermissions.filter((p) =>
      p.startsWith("allow-"),
    );
    const unknown = customTrayPermissions.filter(
      (id) => !permissionIds.has(id),
    );
    expect(
      unknown,
      `default.json references unknown tray permission ids: ${unknown}`,
    ).toEqual([]);
  });

  it("parses every permission block in tray-commands.toml", () => {
    expect(permissionBlocks.length).toBeGreaterThan(0);
    expect(permissionBlocks.map((b) => b.identifier).sort()).toEqual(
      [...permissionIds].sort(),
    );
  });

  it("grants every tray permission defined in tray-commands.toml in default.json", () => {
    const granted = new Set(capabilityPermissions);
    const ungranted = [...permissionIds].filter((id) => !granted.has(id));
    expect(
      ungranted,
      `tray-commands.toml permissions missing from default.json (IPC would be denied): ${ungranted}`,
    ).toEqual([]);
  });

  it("grants the permission owning every registered command in default.json", () => {
    const granted = new Set(capabilityPermissions);
    const denied = registered.filter((cmd) => {
      const owners = permissionBlocks.filter((b) => b.commands.includes(cmd));
      return !owners.some((b) => granted.has(b.identifier));
    });
    expect(
      denied,
      `registered commands whose permission is not granted in default.json: ${denied}`,
    ).toEqual([]);
  });

  it("registers repo convert IPC commands", () => {
    expect(registered).toContain("convert_repo");
    expect(registered).toContain("get_repo_convert_status");
  });

  it("registers repo clone and remote source IPC commands", () => {
    for (const cmd of [
      "clone_repo",
      "get_repo_clone_status",
      "get_remote_sources",
    ]) {
      expect(registered).toContain(cmd);
      expect(permitted.has(cmd)).toBe(true);
    }
  });
});
