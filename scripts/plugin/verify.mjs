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
      `release/Music-Generator-Loops-0.1.0-${platform}-${architecture}`,
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
for (const format of ["vst3", "clap"]) {
  const bundle = path.join(folder, `MusicGeneratorLoops.${format}`);
  if (!(await exists(bundle))) throw new Error(`Missing ${format}`);
  if (platform === "macos")
    await run("codesign", ["--verify", "--deep", "--strict", bundle]);
}
console.log(
  `Verified ${Object.keys(sums).length} package files and both plugin formats`,
);
