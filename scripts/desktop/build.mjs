import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { prepareEngine, engineFolder } from "./prepare-engine.mjs";
import { download, exists, hash, platform, root, run, verifyEngine } from "./common.mjs";

await prepareEngine(); await verifyEngine(engineFolder);
const output = path.join(root, "release"); await mkdir(output, { recursive: true });
const pkg = JSON.parse(await (await import("node:fs/promises")).readFile(path.join(root,"package.json"),"utf8"));
const name = `Music-Generator-${pkg.version}`;
const cli = path.join(root,"node_modules/@tauri-apps/cli/tauri.js");
const target = path.join(root,"src-tauri/target/release");
let artifact;
if (platform === "macos") {
  await run(process.execPath, [cli,"build","--bundles","app"]);
  const app = path.join(target,"bundle/macos/Music Generator.app");
  if (!process.env.APPLE_SIGNING_IDENTITY) {
    // Apple Silicon still requires a valid ad-hoc bundle seal for an unsigned test build.
    await run("codesign",["--force","--deep","--sign","-","--entitlements",path.join(root,"src-tauri/Entitlements.plist"),app]);
  }
  await run("codesign",["--verify","--deep","--strict",app]);
  if (process.env.APPLE_SIGNING_IDENTITY && process.env.APPLE_ID && process.env.APPLE_PASSWORD && process.env.APPLE_TEAM_ID) {
    console.log("Tauri signs and notarizes using the configured Apple credentials.");
  }
  artifact = path.join(output,`${name}-macos-arm64.zip`);
  await run("ditto",["-c","-k","--sequesterRsrc","--keepParent",app,artifact]);
} else if (platform === "windows") {
  const runtimeDir = path.join(root,"src-tauri/resources/WebView2");
  if (!await exists(runtimeDir)) {
    const pinned = JSON.parse(await readFile(path.join(root,"models/webview2.json"),"utf8"));
    const cab = path.join(root,".cache/desktop/WebView2.cab");
    await download(pinned.url,cab,pinned.sha256);
    const unpack = path.join(root,".cache/desktop/webview-unpack"); await rm(unpack,{recursive:true,force:true}); await mkdir(unpack,{recursive:true});
    await run("expand.exe",[cab,"-F:*",unpack]);
    const dirs = await readdir(unpack); const extracted = dirs.find(n=>n.startsWith("Microsoft.WebView2.FixedVersionRuntime."));
    if (!extracted) throw new Error("Invalid WebView2 cabinet");
    await cp(path.join(unpack,extracted),runtimeDir,{recursive:true});
  }
  if (!await exists(path.join(runtimeDir,"msedgewebview2.exe"))) throw new Error("WebView2 runtime is incomplete");
  await run(process.execPath,[cli,"build","--no-bundle"]);
  const portable = path.join(output,`${name}-windows-x64`); await rm(portable,{recursive:true,force:true}); await mkdir(portable,{recursive:true});
  await cp(path.join(target,"music-generator.exe"),path.join(portable,"Music Generator.exe"));
  await cp(engineFolder,path.join(portable,"engine"),{recursive:true});
  await cp(runtimeDir,path.join(portable,"WebView2"),{recursive:true});
  await cp(path.join(root,"LICENSE"),path.join(portable,"LICENSE"));
  await cp(path.join(root,"THIRD_PARTY_NOTICES.md"),path.join(portable,"THIRD_PARTY_NOTICES.md"));
  await cp(path.join(root,"docs/desktop.md"),path.join(portable,"README.md"));
  artifact = `${portable}.zip`;
  // ZIP is produced without an MSI, NSIS executable, or runtime installer.
  await run("tar.exe",["-a","-cf",artifact,"-C",output,path.basename(portable)]);
  await rm(portable,{recursive:true,force:true});
} else {
  await run(process.execPath,[cli,"build","--bundles","appimage"]);
  const folder = path.join(target,"bundle/appimage"); const file = (await readdir(folder)).find(n=>n.endsWith(".AppImage"));
  if (!file) throw new Error("AppImage was not generated");
  artifact = path.join(output,`${name}-linux-x64.AppImage`); await cp(path.join(folder,file),artifact);
}
await writeFile(`${artifact}.sha256`,`${await hash(artifact)}  ${path.basename(artifact)}\n`);
console.log(`Ready: ${artifact}`);
