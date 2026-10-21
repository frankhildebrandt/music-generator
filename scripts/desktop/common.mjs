import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { spawn } from "node:child_process";

export const root = fileURLToPath(new URL("../../", import.meta.url));
export const manifest = JSON.parse(await readFile(path.join(root, "models/manifest.json"), "utf8"));
export const platform = process.platform === "darwin" ? "macos" : process.platform === "win32" ? "windows" : "linux";
export const architecture = process.arch;

export function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: "inherit", ...options });
    child.on("error", reject);
    child.on("exit", (code) => code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`)));
  });
}

export async function hash(file) {
  const digest = createHash("sha256");
  for await (const chunk of createReadStream(file)) digest.update(chunk);
  return digest.digest("hex");
}

export async function exists(file) { return stat(file).then(() => true, () => false); }

export async function files(folder) {
  const result = [];
  for (const entry of await readdir(folder, { withFileTypes: true })) {
    const file = path.join(folder, entry.name);
    if (entry.isDirectory()) result.push(...await files(file)); else if (entry.isFile() || (entry.isSymbolicLink() && (await stat(file)).isFile())) result.push(file);
  }
  return result;
}

export async function download(url, file, checksum) {
  if (await exists(file) && (!checksum || await hash(file) === checksum)) return;
  await mkdir(path.dirname(file), { recursive: true });
  console.log(`Downloading ${new URL(url).pathname.split("/").pop()}…`);
  await run(process.platform === "win32" ? "curl.exe" : "curl", ["--fail", "--location", "--retry", "3", "--output", file, url]);
  if (checksum && await hash(file) !== checksum) throw new Error(`Checksum mismatch: ${file}`);
}

export async function recordHashes(folder, metadata) {
  const sums = {};
  for (const file of await files(folder)) if (path.basename(file) !== "VERSION.desktop.json") sums[path.relative(folder, file).split(path.sep).join("/")] = await hash(file);
  await writeFile(path.join(folder, "VERSION.desktop.json"), JSON.stringify({ ...metadata, files: sums }, null, 2) + "\n");
}

export async function verifyEngine(folder) {
  const provenance = JSON.parse(await readFile(path.join(folder, "VERSION.desktop.json"), "utf8"));
  if (provenance.platform !== platform || provenance.arch !== architecture || provenance.revision !== manifest.engines[platform].revision) throw new Error("Engine belongs to a different platform or revision");
  if (platform === "macos" && provenance.sandbox_patch_sha256 !== await hash(path.join(root,"scripts/desktop/engine-sandbox.mm"))) throw new Error("Engine sandbox bridge needs rebuilding");
  if (platform === "macos" && provenance.recipe_sha256 !== await hash(path.join(root,"scripts/desktop/prepare-engine.mjs"))) throw new Error("Engine build recipe changed");
  const executable = platform === "windows" ? "ace-server.exe" : "ace-server";
  if (!provenance.files[executable]) throw new Error("Engine executable is missing from checksums");
  for (const [file, checksum] of Object.entries(provenance.files)) {
    if (file.split("/").includes("..") || path.isAbsolute(file) || await hash(path.join(folder, file)) !== checksum) throw new Error(`Engine resource failed verification: ${file}`);
  }
  return provenance;
}
