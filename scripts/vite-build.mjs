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

/** libuv teardown noise on some macOS + Node builds after a successful Vite build. */
const LIBUV_TEARDOWN_RE =
  /Assertion failed: \(errno == EINTR\), function uv__io_poll/;

const child = spawn(
  process.execPath,
  ["--no-warnings", join(root, "node_modules/vite/bin/vite.js"), "build"],
  { cwd: root, stdio: ["inherit", "inherit", "pipe"], env: process.env },
);

let stderr = "";
child.stderr?.on("data", (chunk) => {
  const text = chunk.toString();
  stderr += text;
  if (!LIBUV_TEARDOWN_RE.test(text)) {
    process.stderr.write(chunk);
  }
});

child.on("close", (code, signal) => {
  const buildOk = existsSync(entryHtml);

  if (code === 0) {
    process.exit(0);
  }

  const aborted = code === 134 || signal === "SIGABRT" || signal === "SIGTRAP";
  if (aborted && buildOk) {
    process.exit(0);
  }

  if (buildOk && LIBUV_TEARDOWN_RE.test(stderr)) {
    process.exit(0);
  }

  process.exit(code ?? 1);
});

child.on("error", (err) => {
  console.error(err);
  process.exit(1);
});
