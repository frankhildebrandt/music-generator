import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { assertLinuxDependencies, copyCudaRuntime, linuxRpath } from "./linux-runtime.mjs";

describe("portable Linux runtime", () => {
  it("locates shared CUDA libraries from the engine, title helper and nested backends", () => {
    assert.equal(linuxRpath("/app/engine/ace-server", "/app/engine"), "$ORIGIN:$ORIGIN/cuda");
    assert.equal(linuxRpath("/app/engine/title/music-title", "/app/engine"), "$ORIGIN:$ORIGIN/..:$ORIGIN/../cuda");
    assert.equal(linuxRpath("/app/engine/cuda/libcublas.so.12", "/app/engine"), "$ORIGIN:$ORIGIN/..:$ORIGIN/../cuda");
  });
  it("rejects missing libraries, driver stubs and reliance on the builder's CUDA installation", () => {
    assert.throws(() => assertLinuxDependencies("libcublas.so.12 => not found", "/app/engine"));
    assert.throws(() => assertLinuxDependencies("libcuda.so.1 => /usr/lib/libcuda.so.1 (0x1)", "/app/engine"));
    assert.throws(() => assertLinuxDependencies("libcudart.so.12 => /usr/local/cuda/lib64/libcudart.so.12 (0x1)", "/app/engine"));
    assert.doesNotThrow(() => assertLinuxDependencies("libcublas.so.12 => /app/engine/cuda/libcublas.so.12 (0x1)\nlibc.so.6 => /lib/libc.so.6 (0x2)", "/app/engine"));
  });
  it("copies only redistributable runtime SONAMEs and their licenses", async () => {
    const work = await mkdtemp(path.join(tmpdir(), "cuda-runtime-test-"));
    try {
      const sdk = path.join(work, "sdk"), engine = path.join(work, "engine");
      await mkdir(path.join(sdk, "lib"), { recursive: true });
      await mkdir(path.join(sdk, "licenses"));
      const pin = { version: "test", runtime: { "libcudart.so.12": "cuda_cudart" } };
      await writeFile(path.join(sdk, "lib/libcudart.so.12"), "runtime");
      await writeFile(path.join(sdk, "lib/libcuda.so.1"), "driver stub");
      await writeFile(path.join(sdk, "licenses/cuda_cudart.txt"), "runtime license");
      await copyCudaRuntime(engine, sdk, pin);
      assert.equal(await readFile(path.join(engine, "cuda/libcudart.so.12"), "utf8"), "runtime");
      assert.equal(await readFile(path.join(engine, "cuda/licenses/cuda_cudart.txt"), "utf8"), "runtime license");
      await assert.rejects(readFile(path.join(engine, "cuda/libcuda.so.1")));
      await assert.rejects(copyCudaRuntime(engine, sdk, { ...pin, runtime: { "libcuda.so.1": "driver" } }));
    } finally { await rm(work, { recursive: true, force: true }); }
  });
});
