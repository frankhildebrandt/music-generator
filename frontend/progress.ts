import type { JobProgress } from "./types";

export function progressText(progress: JobProgress, translate: (key: string) => string): string {
  const elapsed = `${Math.floor(progress.elapsed_seconds / 60)}:${String(progress.elapsed_seconds % 60).padStart(2, "0")}`;
  const steps = progress.step == null ? "" : ` · ${progress.step}${progress.total_steps ? `/${progress.total_steps}` : ""}`;
  return `${translate(`desktop_progress_${progress.phase}`)}${steps} · ${elapsed}`;
}

export function progressBar(progress?: JobProgress): string {
  if (!progress?.total_steps || progress.step == null) return '<div class="mini-bar" aria-hidden="true"></div>';
  const total = Math.max(1, progress.total_steps), step = Math.max(0, Math.min(total, progress.step));
  return `<progress class="step-progress" max="${total}" value="${step}" aria-label="DiT"></progress>`;
}
