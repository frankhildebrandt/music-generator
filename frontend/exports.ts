import { save } from "@tauri-apps/plugin-dialog";
import { backend } from "./backend";
import type { Song } from "./types";

function safeName(name: string): string { return name.replace(/[\\/:*?"<>|]/g, "_").slice(0, 80) || "Song"; }

export async function exportSong(song: Song, lyrics = false): Promise<void> {
  const ext = lyrics ? "txt" : song.file?.split(".").pop() || "wav";
  const path = await save({ defaultPath: `${safeName(song.title || song.caption)}.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (path) await backend.export(lyrics ? "lyrics" : "audio", path, song.id);
}

export async function exportFolder(folder: string): Promise<void> {
  const path = await save({ defaultPath: "Music Generator.zip", filters: [{ name: "ZIP", extensions: ["zip"] }] });
  if (path) await backend.export("zip", path, null, folder);
}
