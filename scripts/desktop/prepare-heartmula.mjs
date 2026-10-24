import { appendFile, cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { architecture, download, exists, files, hash, platform, root, run, target } from "./common.mjs";

const pin = JSON.parse(await readFile(path.join(root,"models/heartmula-engine.json"),"utf8"));
export const heartFolder = path.join(root,"src-tauri/resources/engine/heartmula");
const exe = platform === "windows" ? "audiocpp_cli.exe" : "audiocpp_cli";
async function recipe() { return (await hash(fileURLToPath(import.meta.url))) + ":" + (await hash(path.join(root,"scripts/desktop/engine-sandbox.mm"))) + ":" + (await hash(path.join(root,"scripts/desktop/licenses/Vulkan-Loader.txt"))); }
export async function verifyHeartmula(folder = heartFolder, checkRecipe = true) {
  const proof = JSON.parse(await readFile(path.join(folder,"HEART_RUNTIME.json"),"utf8"));
  if (proof.revision !== pin.revision || proof.platform !== platform || proof.arch !== architecture || !proof.files[exe]) throw Error("HeartMuLa runtime target/revision mismatch");
  if (checkRecipe && proof.recipe !== await recipe()) throw Error("HeartMuLa build recipe changed");
  for (const [file, checksum] of Object.entries(proof.files)) {
    if (file.split("/").includes("..") || path.isAbsolute(file) || await hash(path.join(folder,file)) !== checksum) throw Error(`HeartMuLa resource mismatch: ${file}`);
  }
  return proof;
}
export async function recordHeartmula(folder = heartFolder) {
  const sums = {};
  for (const file of await files(folder)) if (path.basename(file) !== "HEART_RUNTIME.json") sums[path.relative(folder,file).split(path.sep).join("/")] = await hash(file);
  await writeFile(path.join(folder,"HEART_RUNTIME.json"),JSON.stringify({revision:pin.revision,platform,arch:architecture,recipe:await recipe(),files:sums},null,2)+"\n");
}
export async function prepareHeartmula() {
  try { await verifyHeartmula(); return; } catch { /* Prepare pinned complete runtime. */ }
  const cache = path.join(root,".cache/desktop/heartmula"); await mkdir(cache,{recursive:true});
  // Never reuse an extracted source tree from a different pinned revision.
  const source = path.join(cache,pin.revision,"source"); const build = path.join(cache,pin.revision,"build");
  let built;
  if (platform === "windows") {
    const archive = path.join(cache,"runtime.zip"); await download(pin.windows_url,archive,pin.windows_sha256);
    const unpacked = path.join(cache,"unpacked"); await rm(unpacked,{recursive:true,force:true}); await mkdir(unpacked,{recursive:true});
    await run("tar.exe",["-xf",archive,"-C",unpacked]);
    const binary=(await files(unpacked)).find(f=>path.basename(f)===exe); if(!binary)throw Error("HeartMuLa CLI missing"); built=path.dirname(binary);
  } else {
    const archive=path.join(cache,"source.tar.gz"); await download(pin.source_url,archive,pin.source_sha256);
    if (!await exists(path.join(source,"CMakeLists.txt"))) { await mkdir(source,{recursive:true}); await run("tar",["-xf",archive,"-C",source,"--strip-components=1"]); }
    if (platform === "macos") {
      const cmake=path.join(source,"CMakeLists.txt"); let text=await readFile(cmake,"utf8");
      if (!text.includes("desktop-sandbox.mm")) { text=text.replace("project(AudioCpp LANGUAGES C CXX)","project(AudioCpp LANGUAGES C CXX OBJC OBJCXX)"); await writeFile(cmake,text); await appendFile(cmake,'\ntarget_sources(audiocpp_cli PRIVATE desktop-sandbox.mm)\ntarget_link_libraries(audiocpp_cli PRIVATE "-framework Foundation")\n'); }
      await cp(path.join(root,"scripts/desktop/engine-sandbox.mm"),path.join(source,"desktop-sandbox.mm"));
    }
    const options=["-DCMAKE_BUILD_TYPE=Release","-DAUDIOCPP_MODEL_SET=custom","-DAUDIOCPP_MODELS=heartmula","-DAUDIOCPP_DEPLOYMENT_BUILD=ON","-DAUDIOCPP_BUILD_C_API=OFF","-DAUDIOCPP_BUILD_NATIVE_MODEL_MANAGER=OFF","-DENGINE_ENABLE_NATIVE_CPU=OFF","-DENGINE_ENABLE_CPU_ALL_VARIANTS=OFF","-DENGINE_ENABLE_OPENMP=OFF","-DGGML_OPENMP=OFF","-DENGINE_ENABLE_CUDA=OFF","-DENGINE_BUILD_TESTS=OFF","-DENGINE_BUILD_EXAMPLES=OFF","-DENGINE_BUILD_WARMBENCH=OFF",`-DENGINE_ENABLE_METAL=${platform==="macos"&&!target.cpuOnly?"ON":"OFF"}`,`-DENGINE_ENABLE_VULKAN=${platform==="linux"?"ON":"OFF"}`];
    if(platform==="macos")options.push(`-DCMAKE_OSX_ARCHITECTURES=${architecture==="arm64"?"arm64":"x86_64"}`,`-DCMAKE_OSX_DEPLOYMENT_TARGET=${target.minimumSystemVersion}`,"-DGGML_METAL_EMBED_LIBRARY=ON");
    if(architecture==="x64")options.push("-DGGML_SSE42=OFF","-DGGML_BMI2=OFF","-DGGML_AVX=OFF","-DGGML_AVX2=OFF","-DGGML_FMA=OFF","-DGGML_F16C=OFF");
    await run("cmake",["-S",source,"-B",build,...options]); await run("cmake",["--build",build,"--target","audiocpp_cli","--parallel","2"]); built=path.join(build,"bin");
  }
  await rm(heartFolder,{recursive:true,force:true}); await mkdir(heartFolder,{recursive:true});
  for(const file of await files(built)) {
    if(path.basename(file)!==exe&&!/\.(dll|dylib)$|\.so(?:\.\d+)*$|LICENSE|NOTICE/i.test(file))continue;
    const destination=/LICENSE|NOTICE/i.test(path.basename(file))?path.join(heartFolder,"licenses",path.relative(built,file)):path.join(heartFolder,path.basename(file));
    await mkdir(path.dirname(destination),{recursive:true});await cp(file,destination,{dereference:true});
  }
  if(platform!=="windows") { await cp(path.join(source,"LICENSE"),path.join(heartFolder,"LICENSE-audio.cpp")); await cp(path.join(source,"external/ggml/LICENSE"),path.join(heartFolder,"LICENSE-ggml")); }
  else {
    // Preserve the pinned package's matching CRT; fill only missing redistributables.
    const parent=path.dirname(heartFolder);for(const file of await files(parent))if(path.dirname(file)===parent&&/^(vcruntime140(?:_1)?|msvcp140(?:_[12])?|concrt140)\.dll$/i.test(path.basename(file))&&!await exists(path.join(heartFolder,path.basename(file))))await cp(file,path.join(heartFolder,path.basename(file)));
    // Vulkan is linked at process startup, even for CPU-only inference. Ship the
    // loader app-locally so a clean Windows machine needs no runtime installer.
    const runtime=path.join(cache,"vulkan-runtime.zip");await download(pin.windows_vulkan_url,runtime,pin.windows_vulkan_sha256);
    const extracted=path.join(cache,"vulkan-runtime");await rm(extracted,{recursive:true,force:true});await mkdir(extracted,{recursive:true});
    await run("tar.exe",["-xf",runtime,"-C",extracted]);
    const components=await files(extracted);
    const loader=components.find(f=>path.basename(f).toLowerCase()==="vulkan-1.dll"&&path.basename(path.dirname(f)).toLowerCase()==="x64");
    const license=components.find(f=>path.basename(f)==="VulkanRT-License.txt");
    if(!loader||!license)throw Error("Pinned Windows Vulkan loader/license missing");
    await cp(loader,path.join(heartFolder,"vulkan-1.dll"));await cp(license,path.join(heartFolder,"LICENSE-Vulkan-Runtime.txt"));
  }
  if(platform==="linux") {
    const dependencies=execFileSync("ldd",[path.join(built,exe)],{encoding:"utf8"});
    const loader=dependencies.match(/libvulkan\.so\.1\s+=>\s+(\/[^\n]+?)\s+\(/)?.[1];
    if(!loader)throw Error("HeartMuLa requires a resolvable Vulkan loader for app-local packaging");
    await cp(loader,path.join(heartFolder,"libvulkan.so.1"),{dereference:true});
    await cp(path.join(root,"scripts/desktop/licenses/Vulkan-Loader.txt"),path.join(heartFolder,"LICENSE-Vulkan-Loader.txt"));
    await run("patchelf",["--set-rpath","$ORIGIN",path.join(heartFolder,exe)]);
  }
  if(platform!=="windows")for(const file of await files(path.join(source,"external")))if(/^(LICENSE|COPYING|NOTICE)/i.test(path.basename(file))){const dest=path.join(heartFolder,"licenses",path.relative(source,file));await mkdir(path.dirname(dest),{recursive:true});await cp(file,dest);}
  if(platform==="macos")await run("codesign",["--force","--sign","-","--entitlements",path.join(root,"src-tauri/Engine.entitlements"),path.join(heartFolder,exe)]);
  await recordHeartmula(); await verifyHeartmula();
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))await prepareHeartmula();
