import { readFile } from "node:fs/promises";
import path from "node:path";
import {
  architecture,
  exists,
  files,
  hash,
  platform,
  root,
  run,
} from "../desktop/common.mjs";
const folder = path.resolve(
  process.argv[2] ??
    path.join(
      root,
      `release/Music-Generator-Loops-0.1.2-${platform}-${architecture}`,
    ),
);
const sums = JSON.parse(
  await readFile(path.join(folder, "SHA256.json"), "utf8"),
);
for (const [name, checksum] of Object.entries(sums)) {
  const target = path.resolve(folder, name);
  if (
    !target.startsWith(folder + path.sep) ||
    (await hash(target)) !== checksum
  )
    throw new Error(`Invalid package file: ${name}`);
}
for (const file of await files(folder)) {
  const relative = path.relative(folder, file).split(path.sep).join("/");
  if (relative !== "SHA256.json" && !Object.hasOwn(sums, relative))
    throw new Error(`Unlisted package file: ${relative}`);
}
for (const format of ["vst3", "clap", platform === "macos" ? "app" : "exe"]) {
  const bundle = path.join(folder, `MusicGeneratorLoops.${format}`);
  if (!(await exists(bundle))) throw new Error(`Missing ${format}`);
  if (platform === "macos")
    await run("codesign", ["--verify", "--deep", "--strict", bundle]);
}
const resourceDirectories = platform === "macos"
  ? ["MusicGeneratorLoops.vst3/Contents/Resources", "MusicGeneratorLoops.clap/Contents/Resources", "MusicGeneratorLoops.app/Contents/Resources"]
  : ["MusicGeneratorLoops.vst3/Contents/x86_64-win/MusicGeneratorLoops.resources", "MusicGeneratorLoops.resources"];
for (const directory of resourceDirectories) {
  const service = `music-generator-loops-service${platform === "windows" ? ".exe" : ""}`;
  for (const name of [service, "resources/engine/VERSION.desktop.json"])
    if (!(await exists(path.join(folder, directory, name))))
      throw new Error(`Missing runtime resource: ${directory}/${name}`);
}
console.log(
  `Verified ${Object.keys(sums).length} package files, both plugin formats and standalone app`,
);
