// Legacy requests used the engine defaults when steps were zero/missing or
// temperature was null/missing. Reusing a song must not apply fresh-form defaults.
export function samplingForSong(params: { inference_steps?: number | null; lm_temperature?: number | null }) {
  return { inference_steps: params.inference_steps || 50, lm_temperature: params.lm_temperature ?? 0.85 };
}
