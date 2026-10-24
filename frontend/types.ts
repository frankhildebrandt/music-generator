export interface Settings {
  models_dir: string; songs_dir: string; lm_model: string; dit: "standard" | "xl";
  model_provider: string; synthesis_model: string;
  lm_device: "auto" | "cpu"; export_format: "wav" | "mp3"; mp3_available?: boolean;
  cpu_only?: boolean;
  poll_interval: number; timeout_minutes: number; mode: "single" | "split";
  vae_chunk: number | null; tested_fingerprint: string; setup_complete: boolean;
  songs_dir_default?: string; dit_xl_available?: boolean;
}
export interface Generation {
  model_selection?: ModelSelection;
  prompt?: string; style?: string; caption?: string; lyrics?: string; instrumental?: boolean;
  keep_caption?: boolean; title?: string; duration?: number; bpm?: number; keyscale?: string;
  timesignature?: string; vocal_language?: string; seed?: number; variants?: number;
  inference_steps?: number; lm_temperature?: number; group_id?: string; folder_id?: string | null;
}
export interface Song {
  id: string; group_id: string; created_at: string; finished_at: string | null;
  started_at?: string | null;
  status: "queued" | "running" | "done" | "error" | "cancelled"; message: string | null;
  title: string | null; caption: string; lyrics: string | null; seed: number;
  params: Generation; result_meta: Record<string, unknown> | null; file: string | null;
  favorite: boolean; folder_id: string | null; letter: string; path: string | null; url?: string | null;
}
export interface Folder { id: string; name: string; count: number }
export interface Filter { q?: string; favorites?: boolean; group?: string; folder?: string }
export interface Queue { running: { id: string; caption: string; message: string | null }[]; queued: number; active: boolean; library: { count: number; bytes: number } }
export interface Health { ok: boolean; error?: string; props?: { models?: Record<string, string[]> } }
export interface Update { available: boolean; current: string; latest?: string; url?: string; notes?: string }
export interface Model { file: string; estimated_bytes: number; sha256: string; kind: string }
export interface Bootstrap {
  needs_setup: boolean; settings: Settings; version: string; data_dir: string; error: string | null;
  hardware: { ram_bytes: number; gpu: string; vram_bytes: number | null; recommended_lm: string; vae_chunk: number | null; platform: string; cpu_only: boolean };
  manifest: { version: number; base_url: string; models: Model[] };
}
export interface SetupRequest { lm: string; xl: boolean; models_dir: string; songs_dir: string }
export interface DownloadProgress { file: string; phase: "downloading" | "verifying"; downloaded: number; total: number; bytes_per_second: number }
export interface ModelSelection { provider: string; synthesis: string; language: string }
export interface CatalogModel {
  file: string; provider: string; role: "synthesis" | "language" | "dependency"; family: string;
  precision: string; estimated_bytes: number; sha256: string; recommended_steps: number | null;
  verified: boolean; present: boolean; partial_bytes: number;
}
export interface ModelCatalog {
  version: number; revision: string; models: CatalogModel[];
  providers: { id: string; name: string; status: "available" | "planned"; adapter: string | null; source: string }[];
  selection: ModelSelection; hardware: Bootstrap["hardware"]; free_bytes: number | null;
  models_dir: string; busy: boolean;
}
