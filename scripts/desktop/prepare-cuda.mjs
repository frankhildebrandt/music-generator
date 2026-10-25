import { cp, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import pin from "../../models/cuda-linux.json" with { type: "json" };
import { architecture, download, exists, hash, platform, root, run } from "./common.mjs";

// Install only the pinned compiler/development components; never install a driver.
export async function prepareCuda() {
  if (platform !== "linux" || architecture !== "x64") throw new Error("CUDA SDK preparation requires Linux x64");
  const cache = path.join(root, ".cache/desktop/cuda", pin.version);
  const sdk = path.join(cache, "sdk"), stamp = path.join(sdk, "INSTALLED.json");
  const recipe = `${await hash(path.join(root, "models/cuda-linux.json"))}:${await hash(fileURLToPath(import.meta.url))}`;
  const required = ["bin/nvcc", "include/cuda_runtime.h", ...Object.keys(pin.runtime).map(name => `lib/${name}`), ...Object.keys(pin.components).map(name => `licenses/${name}.txt`)];
  if (await exists(stamp) && JSON.parse(await readFile(stamp, "utf8")).recipe === recipe && (await Promise.all(required.map(name => exists(path.join(sdk, name))))).every(Boolean)) return sdk;
  await rm(sdk, { recursive: true, force: true });
  await mkdir(path.join(sdk, "licenses"), { recursive: true });
  for (const [name, component] of Object.entries(pin.components)) {
    const archive = path.join(cache, `${name}.tar.xz`);
    await download(new URL(component.path, pin.base_url).href, archive, component.sha256);
    const work = await mkdtemp(path.join(tmpdir(), "music-cuda-component-"));
    try {
      await run("tar", ["-xf", archive, "-C", work, "--strip-components=1"]);
      const license = ["LICENSE.txt", "LICENSE", "EULA.txt"].map(name => path.join(work, name));
      const selected = (await Promise.all(license.map(exists))).findIndex(Boolean);
      if (selected < 0) throw new Error(`CUDA component license missing: ${name}`);
      await cp(license[selected], path.join(sdk, "licenses", `${name}.txt`));
      await cp(work, sdk, { recursive: true, verbatimSymlinks: true });
    } finally { await rm(work, { recursive: true, force: true }); }
    await rm(archive);
  }
  // NVIDIA's tar components use lib/, while CMake and nvcc also probe lib64/.
  if (!await exists(path.join(sdk, "lib64"))) await symlink("lib", path.join(sdk, "lib64"));
  for (const name of required) if (!await exists(path.join(sdk, name))) throw new Error(`Incomplete CUDA SDK: ${name}`);
  await writeFile(stamp, JSON.stringify({ version: pin.version, recipe }) + "\n");
  return sdk;
}

export async function cudaBuildOptions() {
  const sdk = await prepareCuda();
  return [`-DCUDAToolkit_ROOT=${sdk}`, `-DCMAKE_CUDA_COMPILER=${path.join(sdk, "bin/nvcc")}`];
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) console.log(await prepareCuda());
