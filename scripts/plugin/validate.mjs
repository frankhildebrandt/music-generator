import path from "node:path";
import { platform, root, run } from "../desktop/common.mjs";
const sdk = path.join(root, ".cache/iplug2/Dependencies/IPlug/VST3_SDK");
const build = path.join(root, ".cache/vst3-validator");
await run("cmake", [
  "-S",
  sdk,
  "-B",
  build,
  "-DCMAKE_BUILD_TYPE=Release",
  "-DSMTG_ENABLE_VSTGUI_SUPPORT=OFF",
  "-DSMTG_ENABLE_VST3_PLUGIN_EXAMPLES=OFF",
]);
await run("cmake", [
  "--build",
  build,
  "--config",
  "Release",
  "--target",
  "validator",
  "-j",
  "3",
]);
await run(
  path.join(
    build,
    "bin/Release",
    `validator${platform === "windows" ? ".exe" : ""}`,
  ),
  [path.join(root, "plugin/build-native/out/MusicGeneratorLoops.vst3")],
);
if (platform !== "windows")
  await run(path.join(root, "plugin/build-native/clap_host"), [
    path.join(
      root,
      "plugin/build-native/out/MusicGeneratorLoops.clap/Contents/MacOS/MusicGeneratorLoops",
    ),
  ]);
