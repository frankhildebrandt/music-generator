import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { desktopTarget } from "./targets.mjs";

describe("native desktop engine targets", () => {
  it("builds a portable Intel CPU engine with the same deployment target as its app", () => {
    const intel = desktopTarget("macos", "x64");
    assert.equal(intel.cpuOnly, true);
    for (const option of ["-DGGML_METAL=OFF", "-DGGML_NATIVE=OFF", "-DGGML_CPU_ALL_VARIANTS=ON", "-DCMAKE_OSX_ARCHITECTURES=x86_64", `-DCMAKE_OSX_DEPLOYMENT_TARGET=${intel.minimumSystemVersion}`]) assert(intel.cmake.includes(option));
    const arm = desktopTarget("macos", "arm64");
    assert.equal(arm.cpuOnly, false);
    assert(arm.cmake.includes("-DGGML_METAL=ON"));
    assert.equal(arm.minimumSystemVersion, "26.0");
    assert.throws(() => desktopTarget("macos", "ia32"));
    assert.throws(() => desktopTarget("windows", "arm64"));
  });
});
