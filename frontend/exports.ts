import { save } from "@tauri-apps/plugin-dialog";
import { backend } from "./backend";
import type { Song } from "./types";
import { t } from "./i18n.js";

function safeName(name: string): string { return name.replace(/[\\/:*?"<>|]/g, "_").slice(0, 80) || "Song"; }

export async function exportSong(song: Song, lyrics = false): Promise<void> {
  const format = lyrics ? null : (await backend.settings()).export_format;
  const ext = lyrics ? "txt" : format === "mp3" ? "mp3" : song.file?.split(".").pop() || "wav";
  const path = await save({ defaultPath: `${safeName(song.title || song.caption)}.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (path) await performExport(lyrics ? "lyrics" : "audio", path, song.id);
}

export async function exportFolder(folder: string): Promise<void> {
  const path = await save({ defaultPath: "Music Generator.zip", filters: [{ name: "ZIP", extensions: ["zip"] }] });
  if (path) await performExport("zip", path, null, folder);
}

async function performExport(kind: "audio" | "lyrics" | "zip", path: string, id: string | null = null, folder: string | null = null): Promise<void> {
  try { await backend.export(kind, path, id, folder); }
  catch (error) { if (error instanceof Error && error.message === "mp3_failed") throw new Error(t("mp3_failed")); throw error; }
}
