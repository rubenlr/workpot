#!/usr/bin/env node
/**
 * Run Vite production build in a child process.
 * On some macOS + Node builds, libuv aborts (SIGABRT / exit 134) during teardown
 * after adapter-static even when the build succeeded; treat that as success if
 * artifacts are present.
 */
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "build");
const entryHtml = join(outDir, "index.html");

const child = spawn(
  process.execPath,
  ["--no-warnings", join(root, "node_modules/vite/bin/vite.js"), "build"],
  { cwd: root, stdio: "inherit", env: process.env },
);

child.on("close", (code, signal) => {
  if (code === 0) {
    process.exit(0);
  }

  const aborted = code === 134 || signal === "SIGABRT" || signal === "SIGTRAP";
  if (aborted && existsSync(entryHtml)) {
    process.exit(0);
  }

  process.exit(code ?? 1);
});

child.on("error", (err) => {
  console.error(err);
  process.exit(1);
});
