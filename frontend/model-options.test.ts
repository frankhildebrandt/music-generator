import { expect, it } from "vitest";
import source from "../models/catalog.json";
import title from "../models/title-model.json";
import { modelOptions, profile } from "./model-options";
import type { ModelCatalog, ModelSelection } from "./types";

function catalog(): ModelCatalog {
  return { ...source, title_model: {...title.model,verified:false,present:false,partial_bytes:0}, models: source.models.map(m => ({ ...m, present: false, verified: false, partial_bytes: 0 })), selection: selection(), hardware: { ram_bytes: 8 * 1024 ** 3, gpu: "CPU", vram_bytes: null, recommended_lm: "0.6B", vae_chunk: 256, platform: "macos", cpu_only: true }, free_bytes: 50 * 1024 ** 3, models_dir: "/models", busy: false } as ModelCatalog;
}
function selection(): ModelSelection { return { provider: "ace-step", synthesis: "acestep-v15-sft-Q8_0.gguf", language: "acestep-5Hz-lm-0.6B-Q8_0.gguf" }; }

it("filters the full profile by memory without hiding the current saved choice", () => {
  const c = catalog();
  const s = selection();
  const options = modelOptions(c, s, "synthesis", true);
  expect(options.length).toBeLessThan(52);
  expect(options.some(m => m.file === s.synthesis)).toBe(true);
  expect(options.some(m => m.file === "acestep-v15-xl-sft-BF16.gguf")).toBe(false);
  s.synthesis = "acestep-v15-xl-sft-BF16.gguf";
  expect(modelOptions(c, s, "synthesis", true).some(m => m.file === s.synthesis)).toBe(true);
  expect(profile(c, s).compatible).toBe(false);
  expect(modelOptions(c, s, "synthesis", false)).toHaveLength(52);
  expect(modelOptions(c, s, "language", false)).toHaveLength(8);
});

it("includes base dependencies, resumed bytes and verification in download readiness", () => {
  const c = catalog();
  const s = selection();
  const initial = profile(c, s);
  expect(initial.models).toHaveLength(4);
  initial.models[0].partial_bytes = 123;
  expect(profile(c, s).download).toBe(initial.download - 123);
  for (const m of initial.models) m.present = true;
  expect(profile(c, s).download).toBe(0);
  expect(profile(c, s).verified).toBe(false);
  for (const m of initial.models) m.verified = true;
  expect(profile(c, s).verified).toBe(true);
});

it("rejects removed providers and keeps only ACE-Step assets selectable", () => {
  const c = catalog();
  const removed = { provider: "heartmula", synthesis: "heartmula-q8_0.gguf", language: "" };
  expect(c.providers.map(provider => provider.id)).toEqual(["ace-step"]);
  expect(profile(c, removed).valid).toBe(false);
  expect(modelOptions(c, removed, "synthesis", false)).toHaveLength(0);
  expect(profile(c, { ...selection(), synthesis: "../model.gguf" }).valid).toBe(false);
});

it("provides turbo's eight steps without changing the caller's selection", () => {
  const c = catalog();
  const s = Object.freeze({ ...selection(), synthesis: "acestep-v15-turbo-shift3-Q6_K.gguf" });
  expect(profile(c, s).steps).toBe(8);
  expect(profile(c, selection()).steps).toBe(80);
});
