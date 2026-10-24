import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { desktopTarget } from "./targets.mjs";

describe("native desktop engine targets", () => {
  it("builds a portable Intel CPU engine with the same deployment target as its app", () => {
    const intel = desktopTarget("macos", "x64");
    assert.equal(intel.cpuOnly, true);
    assert.equal(intel.minimumSystemVersion, "13.3");
    for (const option of ["-DGGML_METAL=OFF", "-DGGML_NATIVE=OFF", "-DGGML_CPU_ALL_VARIANTS=ON", "-DCMAKE_OSX_ARCHITECTURES=x86_64", `-DCMAKE_OSX_DEPLOYMENT_TARGET=${intel.minimumSystemVersion}`]) assert(intel.cmake.includes(option));
  });
  it("targets macOS Sonoma on Apple Silicon with Metal acceleration", () => {
    const arm = desktopTarget("macos", "arm64");
    assert.equal(arm.cpuOnly, false);
    assert(arm.cmake.includes("-DGGML_METAL=ON"));
    assert.equal(arm.minimumSystemVersion, "14.0");
    assert(arm.cmake.includes("-DCMAKE_OSX_ARCHITECTURES=arm64"));
    assert(arm.cmake.includes("-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0"));
  });
  it("rejects unsupported architectures", () => {
    assert.throws(() => desktopTarget("macos", "ia32"));
    assert.throws(() => desktopTarget("windows", "arm64"));
  });
  it("keeps the default app bundle minimum aligned with its Apple Silicon engine", async () => {
    const config = JSON.parse(await readFile(new URL("../../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
    assert.equal(config.bundle.macOS.minimumSystemVersion, desktopTarget("macos", "arm64").minimumSystemVersion);
  });
});
