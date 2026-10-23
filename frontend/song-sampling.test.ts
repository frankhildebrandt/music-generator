import { readFileSync } from "node:fs";
import { afterEach, expect, it, vi } from "vitest";
import { samplingForSong } from "./song-sampling";

afterEach(() => vi.unstubAllGlobals());

it("uses the upstream defaults only for a fresh generation form", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  expect(html.match(/<input name="inference_steps"[^>]*value="([^"]+)"/)?.[1]).toBe("80");
  expect(html.match(/<input name="lm_temperature"[^>]*value="([^"]+)"/)?.[1]).toBe("1.2");
  expect(html.match(/<input name="duration"[^>]*max="([^"]+)"/)?.[1]).toBe("600");
  expect(html).toContain('id="lenHint" hidden aria-live="polite"');
});

it("restores stored old, custom and new song values without mutating parameters", () => {
  for (const params of [{ inference_steps: 50, lm_temperature: 1 }, { inference_steps: 75, lm_temperature: 0.65 }, { inference_steps: 80, lm_temperature: 1.2 }]) {
    Object.freeze(params);
    expect(samplingForSong(params)).toEqual(params);
  }
});

it("retains engine defaults for legacy songs with omitted or sentinel values", () => {
  for (const params of [{}, { inference_steps: 0, lm_temperature: null }, { inference_steps: null }]) {
    expect(samplingForSong(params)).toEqual({ inference_steps: 50, lm_temperature: 0.85 });
  }
});

it("updates both tooltips in all twelve languages while retaining desktop messages", async () => {
  vi.stubGlobal("localStorage", { getItem: () => null });
  vi.stubGlobal("navigator", { language: "en" });
  vi.stubGlobal("document", { documentElement: { lang: "" }, querySelectorAll: () => [], querySelector: () => null });
  const { I18N, LANGS } = await import("./i18n.js");
  expect(Object.keys(LANGS)).toHaveLength(12);
  for (const lang of Object.keys(LANGS) as (keyof typeof I18N)[]) {
    expect(I18N[lang].iterations_tip).toContain("80");
    expect(I18N[lang].iterations_tip).toContain("50");
    expect(I18N[lang].variance_tip).toContain("1.2");
    expect(I18N[lang].variance_tip).toContain("0.85");
    expect(I18N[lang].mp3_failed).toBeTruthy();
    expect(I18N[lang].lm_dev_hint).not.toContain("OFF");
    expect(I18N[lang].length_tip).toContain("10:00");
    expect(I18N[lang].len_long).toBeTruthy();
    expect(I18N[lang].err_timeout.split("–")[1]?.trim()).toBeTruthy();
  }
});
