import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { Generation, Job, Project, Summary } from "./model";

async function call<T>(action: string, payload: unknown = {}): Promise<T> {
  try { return await invoke<T>("pro_request", { action, payload }); }
  catch (error) { throw new Error(String(error)); }
}
function urls(project: Project): Project {
  return { ...project, takes: project.takes.map((take) => ({ ...take, url: take.path ? convertFileSrc(take.path) : undefined })) };
}
export const proApi = {
  list: () => call<Summary[]>("list"),
  open: async (id: string) => urls(await call<Project>("open", { id })),
  create: async (name: string, bpm: number, key: string, duration: number) => urls(await call<Project>("create", { name, bpm, key, duration })),
  save: async (project: Project) => urls(await call<Project>("save", project)),
  history: async (id: string, redo: boolean) => urls(await call<Project>("history", { id, redo })),
  import: async (id: string, path: string, track_id: string | null = null) => urls(await call<Project>("import", { id, path, track_id })),
  jobs: (id: string) => call<Job[]>("jobs", { id }),
  generate: (id: string, request: Generation) => call<{ ids: string[] }>("generate", { id, request }),
  cancel: (id: string) => call<void>("cancel", { id }),
  retry: (id: string) => call<void>("retry", { id }),
  model: () => call<{ installed: boolean; bytes: number }>("model"),
  install: () => call<void>("install_base"),
  export: (id: string, path: string, stems: boolean) => call<void>("export", { id, path, stems }),
};
