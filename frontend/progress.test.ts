import { describe, expect, it } from "vitest";
import { progressBar, progressText } from "./progress";
import type { JobProgress } from "./types";
const progress: JobProgress = { job_id: "one", sequence: 1, phase: "dit", step: 3, total_steps: 50, elapsed_seconds: 65 };
describe("generation progress", () => {
  it("shows measured DiT steps with an accessible phase-specific progress bar", () => {
    expect(progressText(progress, () => "DiT")).toBe("DiT · 3/50 · 1:05");
    expect(progressBar(progress)).toContain('max="50" value="3"');
    expect(progressBar(progress)).toContain('aria-label="DiT"');
  });
  it("keeps phases without a known total indeterminate", () => {
    const lm = { ...progress, phase: "lm" as const, total_steps: null };
    expect(progressText(lm, () => "Sprachmodell")).toBe("Sprachmodell · 3 · 1:05");
    expect(progressBar(lm)).not.toContain("<progress");
    expect(progressBar({ ...lm, phase: "vae", step: null })).not.toContain("<progress");
  });
});
