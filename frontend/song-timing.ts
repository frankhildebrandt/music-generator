export function generationSeconds(song: { created_at: string; started_at?: string | null; finished_at: string | null }): number | null {
  const end = song.finished_at ? Date.parse(song.finished_at) : NaN;
  const seconds = (end - Date.parse(song.started_at || song.created_at)) / 1000;
  return Number.isFinite(seconds) && seconds >= 0 ? seconds : null;
}
