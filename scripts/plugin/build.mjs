import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { preparePlugin } from "./prepare.mjs";
import { prepareEngine, engineFolder } from "../desktop/prepare-engine.mjs";
import {
  architecture,
  exists,
  files,
  hash,
  platform,
  root,
  run,
  target,
  verifyEngine,
} from "../desktop/common.mjs";
if (!["macos", "windows"].includes(platform))
  throw new Error("Plugin packaging supports macOS and Windows");
if (!process.argv.includes("--skip-deps")) await preparePlugin();
await prepareEngine();
await verifyEngine(engineFolder);
const build = path.join(root, "plugin/build-native");
const environment = {
  ...process.env,
  ...(platform === "macos"
    ? { MACOSX_DEPLOYMENT_TARGET: target.minimumSystemVersion }
    : {}),
};
await run(
  "cargo",
  [
    "build",
    "--release",
    "--locked",
    "--manifest-path",
    "crates/loops-service/Cargo.toml",
  ],
  { env: environment },
);
await run("cmake", [
  "-S",
  "plugin",
  "-B",
  build,
  "-DCMAKE_BUILD_TYPE=Release",
  "-DIPLUG_DEPLOY_PLUGINS=OFF",
  ...(platform === "macos"
    ? [
        `-DCMAKE_OSX_DEPLOYMENT_TARGET=${target.minimumSystemVersion}`,
        `-DCMAKE_OSX_ARCHITECTURES=${architecture === "arm64" ? "arm64" : "x86_64"}`,
      ]
    : []),
]);
await run("cmake", ["--build", build, "--config", "Release", "-j", "3"]);
await run("ctest", [
  "--test-dir",
  build,
  "-C",
  "Release",
  "--output-on-failure",
]);
const packageName = `Music-Generator-Loops-0.1.0-${platform}-${architecture}`;
const folder = path.join(root, "release", packageName);
await rm(folder, { recursive: true, force: true });
await mkdir(folder, { recursive: true });
for (const format of ["vst3", "clap"]) {
  const name = `MusicGeneratorLoops.${format}`;
  await cp(path.join(build, "out", name), path.join(folder, name), {
    recursive: true,
    dereference: true,
  });
}
const resourceDirectories =
  platform === "macos"
    ? [
        "MusicGeneratorLoops.vst3/Contents/Resources",
        "MusicGeneratorLoops.clap/Contents/Resources",
      ]
    : [
        "MusicGeneratorLoops.vst3/Contents/x86_64-win/MusicGeneratorLoops.resources",
        "MusicGeneratorLoops.resources",
      ];
const serviceName = `music-generator-loops-service${platform === "windows" ? ".exe" : ""}`;
for (const directory of resourceDirectories) {
  const destination = path.join(folder, directory);
  await mkdir(path.join(destination, "resources"), { recursive: true });
  await cp(
    path.join(root, "crates/loops-service/target/release", serviceName),
    path.join(destination, serviceName),
  );
  const engine = path.join(destination, "resources/engine");
  await cp(engineFolder, engine, { recursive: true, dereference: true });
  if (platform === "windows") {
    for (const file of await files(engine))
      if (/^(vcruntime140(?:_1)?|msvcp140(?:_[12])?|concrt140)\.dll$/i.test(path.basename(file)))
        await cp(file, path.join(destination, path.basename(file)));
  }
  if (platform === "macos")
    for (const file of await files(engine)) {
      const bytes = await readFile(file);
      if (
        bytes.length < 4 ||
        ![0xfeedfacf, 0xcffaedfe, 0xcafebabe].includes(bytes.readUInt32BE(0))
      )
        continue;
      // The DAW service is unsandboxed: desktop inheritance entitlements cannot apply here.
      await run("codesign", ["--force", "--sign", "-", file]);
    }
}
if (platform === "macos")
  for (const format of ["vst3", "clap"]) {
    const bundle = path.join(folder, `MusicGeneratorLoops.${format}`);
    await run("codesign", ["--force", "--deep", "--sign", "-", bundle]);
    await run("codesign", ["--verify", "--deep", "--strict", bundle]);
  }
await cp(
  path.join(root, "docs/loop-plugin.md"),
  path.join(folder, "README.md"),
);
await cp(
  path.join(root, "docs/loop-plugin-verification.md"),
  path.join(folder, "loop-plugin-verification.md"),
);
await cp(
  path.join(root, "THIRD_PARTY_NOTICES.md"),
  path.join(folder, "THIRD_PARTY_NOTICES.md"),
);
await cp(path.join(root, "LICENSE"), path.join(folder, "LICENSE"));
await mkdir(path.join(folder, "licenses"));
const notices = [
  ["iplug2/Dependencies/IGraphics/NanoVG/LICENSE.txt", "NanoVG.txt"],
  ["iplug2/Dependencies/IGraphics/NanoSVG/LICENSE.txt", "NanoSVG.txt"],
  ["iplug2/Dependencies/IGraphics/MetalNanoVG/LICENSE", "MetalNanoVG.txt"],
  ["iplug2/Dependencies/IGraphics/yoga/LICENSE", "Yoga.txt"],
  ["iplug2/WDL/libpng/LICENSE", "libpng.txt"],
  ["iplug2/Dependencies/IPlug/CLAP_HELPERS/LICENSE", "CLAP-helpers.txt"],
  ["iplug2/LICENSE.txt", "iPlug2.txt"],
  ["iplug2/Dependencies/IPlug/VST3_SDK/LICENSE.txt", "VST3.txt"],
  ["iplug2/Dependencies/IPlug/CLAP_SDK/LICENSE", "CLAP.txt"],
  ["signalsmith-stretch/LICENSE.txt", "Signalsmith-Stretch.txt"],
  ["signalsmith-linear/LICENSE.txt", "Signalsmith-Linear.txt"],
];
for (const [source, name] of notices)
  if (await exists(path.join(root, ".cache", source)))
    await cp(
      path.join(root, ".cache", source),
      path.join(folder, "licenses", name),
    );
await cp(
  path.join(root, "plugin/resources/JSON-LICENSE.txt"),
  path.join(folder, "licenses/JSON.txt"),
);
await cp(
  path.join(root, "plugin/resources/Roboto-LICENSE.txt"),
  path.join(folder, "licenses/Roboto.txt"),
);
await writeFile(
  path.join(folder, "BUILD.json"),
  JSON.stringify(
    {
      manufacturer: "Alphatester x KRGSH",
      version: "0.1.0",
      platform,
      architecture,
      minimumSystemVersion: target.minimumSystemVersion ?? "Windows 10",
      source: execFileSync("git", ["rev-parse", "HEAD"], {
        encoding: "utf8",
      }).trim(),
      working_tree_changes:
        execFileSync("git", ["status", "--porcelain"], {
          encoding: "utf8",
        }).trim().length > 0,
      engine: await verifyEngine(engineFolder),
      dependencies: JSON.parse(
        await readFile(
          path.join(root, "scripts/plugin/dependencies.json"),
          "utf8",
        ),
      ),
    },
    null,
    2,
  ) + "\n",
);
const checksums = {};
for (const file of await files(folder))
  checksums[path.relative(folder, file).split(path.sep).join("/")] =
    await hash(file);
await writeFile(
  path.join(folder, "SHA256.json"),
  JSON.stringify(checksums, null, 2) + "\n",
);
const archive = folder + ".zip";
await rm(archive, { force: true });
if (platform === "macos")
  await run("ditto", [
    "-c",
    "-k",
    "--sequesterRsrc",
    "--keepParent",
    folder,
    archive,
  ]);
else await run("7z", ["a", "-tzip", archive, folder]);
await writeFile(
  archive + ".sha256",
  `${await hash(archive)}  ${path.basename(archive)}\n`,
);
console.log(`Plugin package: ${archive}`);
