import { it } from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { desktopTarget } from "./targets.mjs";
import { engineRecipeHash, manifest, recordHashes, verifyEngine } from "./common.mjs";

it("rejects a cached engine from the other Linux variant or an older build recipe", async () => {
  const work = await mkdtemp(path.join(tmpdir(), "linux-provenance-test-"));
  try {
    const vulkan = desktopTarget("linux", "x64"), cuda = desktopTarget("linux", "x64", "cuda");
    const metadata = { platform: "linux", arch: "x64", revision: manifest.engines.linux.revision, backend: "vulkan", recipe_sha256: await engineRecipeHash(vulkan) };
    await writeFile(path.join(work, "ace-server"), "test engine");
    await recordHashes(work, metadata);
    await verifyEngine(work, vulkan);
    await assert.rejects(verifyEngine(work, cuda), /Linux backend changed/);
    assert.notEqual(await engineRecipeHash(cuda), metadata.recipe_sha256);
    await recordHashes(work, { ...metadata, recipe_sha256: "stale" });
    await assert.rejects(verifyEngine(work, vulkan), /build recipe changed/);
    await recordHashes(work, { ...metadata, backend: "cuda", recipe_sha256: await engineRecipeHash(cuda) });
    await verifyEngine(work, cuda);
    await assert.rejects(verifyEngine(work, vulkan), /Linux backend changed/);
    await writeFile(path.join(work, "unrecorded.so"), "extra library");
    await assert.rejects(verifyEngine(work, cuda), /inventory changed/);
    await rm(path.join(work, "unrecorded.so"));
    await mkdir(path.join(work, "heartmula"));
    await writeFile(path.join(work, "heartmula/HEART_RUNTIME.json"), "retired provider");
    await recordHashes(work, { ...metadata, backend: "cuda", recipe_sha256: await engineRecipeHash(cuda) });
    await assert.rejects(verifyEngine(work, cuda), /Retired provider resources/);
  } finally { await rm(work, { recursive: true, force: true }); }
});
