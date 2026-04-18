import test from "node:test";
import assert from "node:assert/strict";

import { missingBinaryMessage, resolveHostSupport } from "../bin/platform-support.js";

test("resolveHostSupport accepts each packaged native target", () => {
  assert.deepEqual(resolveHostSupport("darwin", "arm64"), {
    supported: true,
    hostKey: "darwin/arm64",
    targetTriple: "aarch64-apple-darwin",
    binaryName: "legolas"
  });
  assert.deepEqual(resolveHostSupport("darwin", "x64"), {
    supported: true,
    hostKey: "darwin/x64",
    targetTriple: "x86_64-apple-darwin",
    binaryName: "legolas"
  });
  assert.deepEqual(resolveHostSupport("linux", "x64", { linuxGlibc: true }), {
    supported: true,
    hostKey: "linux/x64",
    targetTriple: "x86_64-unknown-linux-gnu",
    binaryName: "legolas"
  });
  assert.deepEqual(resolveHostSupport("win32", "x64"), {
    supported: true,
    hostKey: "win32/x64",
    targetTriple: "x86_64-pc-windows-msvc",
    binaryName: "legolas.exe"
  });
});

test("resolveHostSupport rejects unsupported host combinations", () => {
  assert.deepEqual(resolveHostSupport("linux", "arm64", { linuxGlibc: true }), {
    supported: false,
    hostKey: "linux/arm64",
    targetTriple: null,
    binaryName: null
  });
  assert.deepEqual(resolveHostSupport("win32", "arm64"), {
    supported: false,
    hostKey: "win32/arm64",
    targetTriple: null,
    binaryName: null
  });
  assert.deepEqual(resolveHostSupport("freebsd", "x64"), {
    supported: false,
    hostKey: "freebsd/x64",
    targetTriple: null,
    binaryName: null
  });
  assert.deepEqual(resolveHostSupport("linux", "x64", { linuxGlibc: false }), {
    supported: false,
    hostKey: "linux/x64",
    targetTriple: null,
    binaryName: null
  });
});

test("missingBinaryMessage preserves the launcher contract", () => {
  assert.equal(
    missingBinaryMessage("linux/arm64"),
    "legolas: no packaged Rust binary for linux/arm64"
  );
});
