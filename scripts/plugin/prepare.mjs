import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { download, exists, hash, root, run } from "../desktop/common.mjs";
const pins = JSON.parse(
  await readFile(new URL("dependencies.json", import.meta.url), "utf8"),
);
const cache = path.join(root, ".cache");
async function checkout(repository, revision, destination) {
  const stamp = path.join(destination, ".plugin-revision");
  if (
    (await exists(stamp)) &&
    (await readFile(stamp, "utf8")).trim() === revision
  )
    return;
  // Never delete a developer's checkout. New caches use a temporary sibling.
  const temporary = `${destination}.plugin-download`;
  await rm(temporary, { recursive: true, force: true });
  await mkdir(path.dirname(destination), { recursive: true });
  await run("git", ["init", temporary]);
  await run("git", ["-C", temporary, "remote", "add", "origin", repository]);
  await run("git", [
    "-C",
    temporary,
    "fetch",
    "--depth",
    "1",
    repository,
    revision,
  ]);
  await run("git", ["-C", temporary, "checkout", "--detach", "FETCH_HEAD"]);
  if (await exists(destination)) {
    // iPlug ships placeholder SDK folders. Preserve a real checkout on mismatch.
    if (await exists(path.join(destination, ".git")))
      throw new Error(
        `Dependency checkout needs revision ${revision}: ${destination}`,
      );
    await rm(destination, { recursive: true, force: true });
  }
  await cp(temporary, destination, { recursive: true });
  await rm(temporary, { recursive: true, force: true });
  await writeFile(stamp, revision + "\n");
}
export async function preparePlugin() {
  await mkdir(cache, { recursive: true });
  const framework = path.join(cache, "iplug2");
  // Existing manually prepared caches can be registered after checking their HEAD.
  if (
    (await exists(path.join(framework, ".git"))) &&
    !(await exists(path.join(framework, ".plugin-revision")))
  ) {
    const { execFileSync } = await import("node:child_process");
    if (
      execFileSync("git", ["-C", framework, "rev-parse", "HEAD"], {
        encoding: "utf8",
      }).trim() === pins.iplug2
    )
      await writeFile(
        path.join(framework, ".plugin-revision"),
        pins.iplug2 + "\n",
      );
  }
  await checkout(
    "https://github.com/iPlug2/iPlug2.git",
    pins.iplug2,
    framework,
  );
  const sdk = path.join(framework, "Dependencies/IPlug");
  await checkout(
    "https://github.com/free-audio/clap.git",
    pins.clap,
    path.join(sdk, "CLAP_SDK"),
  );
  await checkout(
    "https://github.com/free-audio/clap-helpers.git",
    pins.clap_helpers,
    path.join(sdk, "CLAP_HELPERS"),
  );
  await checkout(
    "https://github.com/steinbergmedia/vst3sdk.git",
    pins.vst3,
    path.join(sdk, "VST3_SDK"),
  );
  await run("git", [
    "-C",
    path.join(sdk, "VST3_SDK"),
    "config",
    "remote.origin.url",
    "https://github.com/steinbergmedia/vst3sdk.git",
  ]);
  await run("git", ["-C", path.join(sdk, "VST3_SDK"), "submodule", "sync"]);
  await run("git", [
    "-C",
    path.join(sdk, "VST3_SDK"),
    "submodule",
    "update",
    "--init",
    "--depth",
    "1",
    "pluginterfaces",
    "base",
    "public.sdk",
    "cmake",
  ]);
  for (const [name, repo, revision] of [
    [
      "signalsmith-stretch",
      "https://github.com/Signalsmith-Audio/signalsmith-stretch.git",
      pins.stretch,
    ],
    [
      "signalsmith-linear",
      "https://github.com/Signalsmith-Audio/linear.git",
      pins.linear,
    ],
  ]) {
    const destination = path.join(cache, name);
    if (
      (await exists(path.join(destination, ".git"))) &&
      !(await exists(path.join(destination, ".plugin-revision")))
    ) {
      const { execFileSync } = await import("node:child_process");
      if (
        execFileSync("git", ["-C", destination, "rev-parse", "HEAD"], {
          encoding: "utf8",
        }).trim() === revision
      )
        await writeFile(
          path.join(destination, ".plugin-revision"),
          revision + "\n",
        );
    }
    await checkout(repo, revision, destination);
  }
  await download(
    pins.json_url,
    path.join(cache, "json/json.hpp"),
    pins.json_sha256,
  );
  const platform = process.platform === "darwin" ? "mac" : "win";
  const archive = path.join(cache, `graphics-${platform}.zip`);
  await download(
    `https://github.com/iPlug2/iPlug2/releases/download/v1.0.0-beta/IPLUG2_DEPS_${platform.toUpperCase()}.zip`,
    archive,
    pins[`graphics_${platform}_sha256`],
  );
  if (!(await exists(path.join(framework, "Dependencies/Build", platform)))) {
    const staging = path.join(cache, "graphics-unpacked");
    await mkdir(staging, { recursive: true });
    await run(
      process.platform === "win32" ? "tar" : "unzip",
      process.platform === "win32"
        ? ["-xf", archive, "-C", staging]
        : ["-q", archive, "-d", staging],
    );
    await cp(
      path.join(staging, `IPLUG2_DEPS_${platform.toUpperCase()}`),
      path.join(framework, "Dependencies/Build"),
      { recursive: true },
    );
    await rm(staging, { recursive: true, force: true });
  }
  const adapter = path.join(
    framework,
    "IPlug/VST3/IPlugVST3_ProcessorBase.cpp",
  );
  let source = await readFile(adapter, "utf8");
  const original =
    "if (paramQueue->getPoint(numPoints - 1,  offsetSamples, value) == kResultTrue)";
  if (source.includes(original)) {
    source = source.replace(
      original,
      "for (int32 point = 0; point < numPoints; ++point)\n        if (paramQueue->getPoint(point, offsetSamples, value) == kResultTrue)",
    );
    await writeFile(adapter, source);
  }
  if (!source.includes("paramQueue->getPoint(point, offsetSamples, value)"))
    throw new Error("Pinned VST3 automation adapter changed unexpectedly");
  await writeFile(
    path.join(cache, "plugin-dependencies.json"),
    JSON.stringify(
      {
        ...pins,
        automation_patch_sha256: await hash(
          path.join(root, "scripts/plugin/iplug-automation.patch"),
        ),
      },
      null,
      2,
    ) + "\n",
  );
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  await preparePlugin();
