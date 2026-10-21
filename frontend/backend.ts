import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { Bootstrap, Filter, Folder, Generation, Health, Queue, Settings, SetupRequest, Song, Update } from "./types";

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try { return await invoke<T>(name, args); }
  catch (error) { throw new Error(typeof error === "string" ? error : String(error)); }
}
const audioUrl = (song: Song): Song => ({ ...song, url: song.path ? convertFileSrc(song.path) : null });

export const backend = {
  rememberFolder: (path: string) => command<void>("remember_folder", { path }),
  bootstrap: () => command<Bootstrap>("bootstrap"),
  setup: (request: SetupRequest) => command<{ ok: boolean; mode: string }>("setup_models", { request }),
  estimate: (request: SetupRequest) => command<{ missing_bytes: number; free_bytes: number }>("setup_estimate", { request }),
  cancelSetup: () => command<void>("cancel_setup"),
  importLibrary: (root: string) => command<{ songs: number; models_dir: string }>("import_library", { root }),
  songs: async (filter: Filter = {}) => (await command<Song[]>("list_songs", { filter })).map(audioUrl),
  song: async (id: string) => audioUrl(await command<Song>("get_song", { id })),
  generate: (request: Generation) => command<{ ids: string[] }>("generate", { request }),
  patch: (id: string, patch: { title?: string; favorite?: boolean; folder_id?: string | null }) => command<Song>("patch_song", { id, patch }),
  remove: (id: string) => command<{ cancelled: boolean; deleted: boolean }>("delete_song", { id }),
  retry: (id: string) => command<Song>("retry_song", { id }),
  folders: () => command<{ folders: Folder[]; all: number; none: number }>("list_folders"),
  saveFolder: (name: string, id: string | null = null) => command<Folder>("save_folder", { id, name }),
  deleteFolder: (id: string) => command<void>("delete_folder", { id }),
  queue: () => command<Queue>("queue"),
  settings: () => command<Settings>("get_settings"),
  saveSettings: (values: Pick<Settings, "timeout_minutes" | "songs_dir" | "dit">) => command<Settings>("save_settings", { values }),
  health: () => command<Health>("health"),
  analyze: async (file: File) => {
    if (file.size > 60 * 1024 * 1024) throw new Error("Audio files must be smaller than 60 MB");
    const filename = /\.(wav|mp3|flac|m4a|ogg)$/i.test(file.name) ? file.name : `${file.name}.wav`;
    return command<Record<string, unknown>>("analyze", { data: Array.from(new Uint8Array(await file.arrayBuffer())), filename });
  },
  export: (kind: "audio" | "lyrics" | "zip", destination: string, id: string | null = null, folder: string | null = null) => command<void>("export_file", { kind, destination, id, folder }),
  checkUpdate: () => command<Update>("check_update"),
  exit: () => command<void>("exit_app"),
};
