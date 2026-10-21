import assert from "node:assert/strict";
import { once } from "node:events";
import { spawn } from "node:child_process";
import { chmod, mkdtemp, mkdir, readFile, readlink, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";
import { exists, files, hash, platform, root, run, verifyEngine } from "./common.mjs";
import { verifyMacSandbox } from "./verify-macos-sandbox.mjs";

// Exercise the distributed files from a relocated directory, without loading any models.
const output = path.join(root, "release");
const diskImage = process.argv.includes("--dmg");
assert(!diskImage || platform === "macos", "DMG verification requires macOS");
const suffix = diskImage ? "-macos-arm64.dmg" : platform === "linux" ? "-linux-x64.AppImage" : `-${platform}-${platform === "macos" ? "arm64" : "x64"}.zip`;
const { version } = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
const filename = `Music-Generator-${version}${suffix}`;
const artifact = path.join(output, filename);
assert(await exists(artifact), `Platform bundle is missing: ${filename}`);
assert.equal(await hash(artifact), (await readFile(`${artifact}.sha256`, "utf8")).split(/\s/)[0]);
const work = await mkdtemp(path.join(tmpdir(), "Music Generator portable test "));
const mount = path.join(work, "mounted");
let mounted = false;
try {
  if (diskImage) {
    await run("hdiutil", ["verify", artifact]);
    await mkdir(mount);
    await run("hdiutil", ["attach", "-readonly", "-nobrowse", "-mountpoint", mount, artifact]);
    mounted = true;
    assert.equal(await readlink(path.join(mount, "Applications")), "/Applications");
    await run("ditto", [path.join(mount, "Music Generator.app"), path.join(work, "Music Generator.app")]);
    await run("hdiutil", ["detach", mount]); mounted = false;
  } else if (platform === "linux") {
    await chmod(artifact, 0o755);
    await run(artifact, ["--appimage-extract"], { cwd: work });
  } else if (platform === "windows") {
    await run("tar.exe", ["-xf", artifact, "-C", work]);
  } else {
    await run("ditto", ["-x", "-k", artifact, work]);
  }
  const extracted = await files(work);
  const record = extracted.find(file => path.basename(file) === "VERSION.desktop.json");
  assert(record, "Bundled engine manifest is missing");
  const engine = path.dirname(record);
  await verifyEngine(engine);
  const app = extracted.find(file => path.basename(file) === (platform === "windows" ? "Music Generator.exe" : "music-generator"));
  assert(app, "Application executable is missing");
  if (platform === "macos") await run("codesign", ["--verify", "--deep", "--strict", path.dirname(path.dirname(path.dirname(app)))]);
  if (platform === "windows") {
    assert(await exists(path.join(path.dirname(app), "WebView2/msedgewebview2.exe")), "Fixed WebView2 is missing");
    for (const dll of ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"]) {
      assert(await exists(path.join(engine, dll)), `Engine runtime ${dll} is missing`);
    }
  }
  if (platform === "macos") {
    await verifyMacSandbox(app, engine);
  } else {
  const listener = createServer();
  listener.listen(0, "127.0.0.1"); await once(listener, "listening");
  const port = listener.address().port;
  await new Promise(resolve => listener.close(resolve));
  const models = path.join(work, "metadata fixture"); await mkdir(models);
  // A tensor-free GGUF permits registry/HTTP startup only; this does not test inference.
  const metadata = Buffer.alloc(96);
  metadata.write("GGUF"); metadata.writeUInt32LE(3, 4); metadata.writeBigUInt64LE(1n, 16);
  metadata.writeBigUInt64LE(20n, 24); metadata.write("general.architecture", 32);
  metadata.writeUInt32LE(8, 52); metadata.writeBigUInt64LE(10n, 56); metadata.write("acestep-lm", 64);
  await writeFile(path.join(models, "smoke.gguf"), metadata);
  const child = spawn(app, ["--engine-supervisor", path.join(engine, platform === "windows" ? "ace-server.exe" : "ace-server"), "--host", "127.0.0.1", "--port", String(port), "--models", models], {
    cwd: engine, stdio: ["pipe", "pipe", "pipe"],
    env: { ...process.env, GGML_BACKEND: "CPU" },
  });
  let log = "";
  child.stdout.on("data", chunk => { log += chunk; });
  child.stderr.on("data", chunk => { log += chunk; });
  const exited = once(child, "exit");
  try {
    let healthy = false;
    for (let attempt = 0; attempt < 120 && child.exitCode === null; attempt++) {
      try {
        const response = await fetch(`http://127.0.0.1:${port}/health`, { signal: AbortSignal.timeout(1000) });
        if (response.ok) { console.log("Bundled engine health:", await response.text()); healthy = true; break; }
      } catch { /* Wait for the bundled server to open its port. */ }
      await new Promise(resolve => setTimeout(resolve, 250));
    }
    assert(healthy, `Relocated bundle failed to start its engine:\n${log}`);
    child.stdin.end();
    const stopped = await Promise.race([exited, new Promise((_, reject) => setTimeout(() => reject(new Error("Bundled supervisor did not stop")), 10000).unref())]);
    assert.equal(stopped[0], 0, log);
    console.log("Bundle checksum, relocated resources, CPU engine startup and supervisor shutdown passed.");
  } finally {
    child.stdin.end();
    if (child.exitCode === null) child.kill();
  }
  }
} finally {
  if (mounted) await run("hdiutil", ["detach", mount]);
  await rm(work, { recursive: true, force: true });
}
