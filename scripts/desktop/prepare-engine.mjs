import { appendFile, cp, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { architecture, download, exists, files, hash, manifest, platform, recordHashes, root, run, verifyEngine } from "./common.mjs";

export const engineFolder = path.join(root, "src-tauri/resources/engine");

export async function prepareEngine() {
  if ((platform === "macos" && architecture !== "arm64") || (platform !== "macos" && architecture !== "x64")) throw new Error("Unsupported desktop architecture");
  try { await verifyEngine(engineFolder); console.log("Engine resources verified."); return; } catch { /* Rebuild incomplete or changed resources. */ }
  const source = manifest.engines[platform];
  const cache = path.join(root, ".cache/desktop"); await mkdir(cache, { recursive: true });
  const work = await mkdtemp(path.join(tmpdir(), "music-engine-"));
  try {
    let built;
    if (platform === "linux" || platform === "macos") {
      const sourceDir = path.join(work, "source");
      await run("git", ["clone", "--no-checkout", source.repository, sourceDir]);
      await run("git", ["checkout", source.source_revision || source.revision], { cwd: sourceDir });
      await run("git", ["submodule", "update", "--init", "--recursive"], { cwd: sourceDir });
      built = path.join(sourceDir, "build");
      if (platform === "macos") {
        await cp(path.join(root,"scripts/desktop/engine-sandbox.mm"), path.join(sourceDir,"desktop-sandbox.mm"));
        const cmake = path.join(sourceDir,"CMakeLists.txt");
        await writeFile(cmake,(await readFile(cmake,"utf8")).replace("LANGUAGES C CXX)","LANGUAGES C CXX OBJC OBJCXX)"));
        await appendFile(cmake, '\ntarget_sources(ace-server PRIVATE desktop-sandbox.mm)\ntarget_link_libraries(ace-server PRIVATE "-framework Foundation")\n');
      }
      const options = platform === "macos" ? ["-DGGML_METAL=ON", "-DGGML_BACKEND_DL=ON", "-DGGML_NATIVE=OFF", "-DCMAKE_OSX_DEPLOYMENT_TARGET=26.0", "-DCMAKE_BUILD_RPATH=@loader_path", "-DCMAKE_INSTALL_RPATH=@loader_path"] : ["-DGGML_VULKAN=ON", "-DGGML_BACKEND_DL=ON", "-DGGML_CPU_ALL_VARIANTS=ON", "-DCMAKE_BUILD_RPATH=$ORIGIN", "-DCMAKE_INSTALL_RPATH=$ORIGIN"];
      await run("cmake", ["-S", sourceDir, "-B", built, "-DCMAKE_BUILD_TYPE=Release", ...options]);
      await run("cmake", ["--build", built, "--config", "Release", "-j", "2"]);
      for (const file of await files(sourceDir)) if (/^(LICENSE|COPYING)/i.test(path.basename(file))) await cp(file, path.join(built, `LICENSE-${path.relative(sourceDir,file).replaceAll(path.sep,"-")}`));
    } else {
      const archive = path.join(cache, `${platform}-${source.revision}.zip`);
      await download(source.url, archive, source.sha256);
      const unpacked = path.join(work, "unpacked"); await mkdir(unpacked);
      await run(platform === "windows" ? "tar.exe" : "unzip", platform === "windows" ? ["-xf", archive, "-C", unpacked] : ["-q", archive, "-d", unpacked]);
      const server = (await files(unpacked)).find((f) => path.basename(f) === (platform === "windows" ? "ace-server.exe" : "ace-server"));
      if (!server) throw new Error("Downloaded archive has no engine executable");
      built = path.dirname(server);
    }
    await rm(engineFolder, { recursive: true, force: true }); await mkdir(engineFolder, { recursive: true });
    // Preserve the complete runtime directory, including dynamically loaded GPU backends.
    for (const file of await files(built)) {
      const relative = path.relative(built, file);
      if (relative.split(path.sep).some((p) => p === "CMakeFiles" || p === "_deps") || /\.(o|a|obj|pdb|lib)$/.test(file)) continue;
      const dest = path.join(engineFolder, relative); await mkdir(path.dirname(dest), { recursive: true }); await cp(file, dest, { dereference: true });
      if (platform === "linux" && /\.so(?:\.\d+)*$/.test(file) && path.dirname(dest) !== engineFolder) await cp(file, path.join(engineFolder,path.basename(file)), { dereference: true });
    }
    if (platform === "macos") await relocateMac();
    if (platform === "linux") {
      for (const file of await files(engineFolder)) {
        if (path.basename(file).startsWith("ace-") || /\.so(?:\.\d+)*$/.test(file)) await run("patchelf",["--set-rpath","$ORIGIN",file]);
      }
    }
    if (platform === "windows") await copyWindowsCrt();
    await recordHashes(engineFolder, { platform, arch: architecture, revision: source.revision, source: source.url || source.repository, source_revision: source.source_revision || source.revision, archive_sha256: source.sha256 || null, sandbox_patch_sha256: platform === "macos" ? await hash(path.join(root,"scripts/desktop/engine-sandbox.mm")) : null, recipe_sha256: await hash(path.join(root,"scripts/desktop/prepare-engine.mjs")) });
    await verifyEngine(engineFolder);
    console.log(`Prepared ${platform} engine ${source.revision}.`);
  } finally { await rm(work, { recursive: true, force: true }); }
}

async function copyWindowsCrt() {
  const vswhere = path.join(process.env["ProgramFiles(x86)"] || "C:/Program Files (x86)","Microsoft Visual Studio/Installer/vswhere.exe");
  const installation = execFileSync(vswhere,["-latest","-property","installationPath"],{encoding:"utf8"}).trim();
  const redistRoot = path.join(installation,"VC/Redist/MSVC");
  const versions=(await readdir(redistRoot)).filter(n=>/^\d/.test(n)).sort((a,b)=>b.localeCompare(a,undefined,{numeric:true}));
  const candidates=await files(path.join(redistRoot,versions[0],"x64"));
  for (const file of candidates) if (/^(vcruntime140(?:_1)?|msvcp140(?:_[12])?|concrt140)\.dll$/i.test(path.basename(file))) await cp(file,path.join(engineFolder,path.basename(file)));
}

async function relocateMac() {
  const all = await files(engineFolder);
  for (const file of all) {
    const bytes = await readFile(file);
    if (bytes.length < 4 || ![0xfeedfacf,0xcffaedfe,0xcafebabe].includes(bytes.readUInt32BE(0))) continue;
    const headers = execFileSync("otool", ["-l", file], { encoding: "utf8" });
    for (const match of headers.matchAll(/cmd LC_RPATH\s+cmdsize \d+\s+path (.+?) \(offset/g)) {
      if (match[1].startsWith("/")) await run("install_name_tool", ["-delete_rpath", match[1], file]);
    }
    if (!headers.includes("path @loader_path (offset")) await run("install_name_tool", ["-add_rpath", "@loader_path", file]);
    const dependencies = execFileSync("otool", ["-L", file], { encoding: "utf8" });
    for (const line of dependencies.split("\n").slice(1)) {
      const dependency = line.trim().split(" (compatibility")[0];
      if (dependency.startsWith("/") && !dependency.startsWith("/usr/lib/") && !dependency.startsWith("/System/") && await exists(path.join(engineFolder, path.basename(dependency)))) await run("install_name_tool", ["-change", dependency, `@loader_path/${path.basename(dependency)}`, file]);
    }
    const inheritance = path.basename(file) === "ace-server" ? ["--entitlements",path.join(root,"src-tauri/Engine.entitlements")] : [];
    await run("codesign", ["--force", "--sign", "-", ...inheritance, file]);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await prepareEngine();
