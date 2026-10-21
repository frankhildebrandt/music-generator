export const RATE = 48000;
export const INSTRUMENTS = ["drums", "bass", "keyboard", "guitar", "synth", "strings", "percussion", "vocals", "backing_vocals", "fx", "brass", "woodwinds"] as const;
export interface Track { id: string; name: string; instrument: string; prompt: string; gain: number; pan: number; mute: boolean; solo: boolean; context: boolean; color: string }
export interface Clip { id: string; track_id: string; take_id: string; start: number; offset: number; length: number; gain: number }
export interface Take { id: string; track_id: string; name: string; file: string; frames: number; created_at: string; request: Record<string, unknown>; source_revision: number; path?: string; url?: string }
export interface Project { id: string; name: string; bpm: number; key: string; beats_per_bar: number; duration: number; revision: number; tracks: Track[]; clips: Clip[]; takes: Take[] }
export interface Summary { id: string; name: string; revision: number; updated_at: string }
export interface Job { id: string; project_id: string; status: string; message: string; take_id: string | null; request: Generation; source_revision: number; created_at: string }
export interface Generation { track_id: string; clip_id: string | null; prompt: string; seed: number; steps: number; variants: number; start: number; end: number }

export function audible(track: Pick<Track, "mute" | "solo">, tracks: Pick<Track, "solo">[]): boolean {
  return !track.mute && (!tracks.some((item) => item.solo) || track.solo);
}
export function splitClip(clip: Clip, frame: number): [Clip, Clip] {
  if (frame <= clip.start || frame >= clip.start + clip.length) throw new Error("Split inside the clip");
  const firstLength = frame - clip.start;
  return [{ ...clip, length: firstLength }, { ...clip, id: crypto.randomUUID(), start: frame, offset: clip.offset + firstLength, length: clip.length - firstLength }];
}
export function trimClip(clip: Clip, start: number, end: number): Clip {
  if (start < clip.start || end > clip.start + clip.length || end <= start) throw new Error("Trim inside the clip");
  return { ...clip, start, offset: clip.offset + start - clip.start, length: end - start };
}
export function snap(frame: number, bpm: number, enabled = true): number {
  const step = RATE * 60 / bpm;
  return Math.max(0, Math.round(enabled ? Math.round(frame / step) * step : frame));
}
export function newTrack(instrument: string, index: number): Track {
  return { id: crypto.randomUUID(), name: instrument, instrument, prompt: "", gain: 0.7, pan: 0, mute: false, solo: false, context: true, color: ["#dca766", "#70c3b0", "#9994e8", "#d586a8", "#78b6d9"][index % 5] };
}
