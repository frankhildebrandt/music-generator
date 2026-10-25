import { mkdtemp, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { run } from "./common.mjs";

// Package the already sealed app without changing its signature or requiring an installer.
export async function createMacDiskImage(app, destination) {
  const staging = await mkdtemp(path.join(tmpdir(), "music-dmg-"));
  try {
    await run("ditto", [app, path.join(staging, "Music Generator.app")]);
    await symlink("/Applications", path.join(staging, "Applications"));
    await run("hdiutil", ["create", "-volname", "Music Generator", "-srcfolder", staging, "-ov", "-format", "UDZO", destination]);
    await run("hdiutil", ["verify", destination]);
  } finally { await rm(staging, { recursive: true, force: true }); }
}
