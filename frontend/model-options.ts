import type { CatalogModel, ModelCatalog, ModelSelection } from "./types";

export function profileModels(catalog: ModelCatalog, selection: ModelSelection): CatalogModel[] {
  return catalog.models.filter(m => m.provider === selection.provider &&
    (m.role === "dependency" || m.file === selection.synthesis || m.file === selection.language));
}

export function profile(catalog: ModelCatalog, selection: ModelSelection) {
  const models = profileModels(catalog, selection);
  const provider = catalog.providers.find(p=>p.id===selection.provider);
  const valid = catalog.providers.some(p => p.id === selection.provider && p.status === "available") &&
    models.some(m => m.role === "synthesis" && m.file === selection.synthesis) &&
    (provider?.integrated_language_model ? selection.language === "" : models.some(m => m.role === "language" && m.file === selection.language));
  const memory = models.reduce((n, m) => n + m.estimated_bytes, 0) +
    (provider?.memory_reserve_bytes ?? (selection.synthesis.includes("-xl-") ? 4 : 3) * 1024 ** 3);
  return {
    models, valid, memory, compatible: valid && memory <= catalog.hardware.ram_bytes,
    verified: valid && models.every(m => m.verified),
    download: models.filter(m => !m.present).reduce((n, m) => n + Math.max(0, m.estimated_bytes - m.partial_bytes), 0),
    steps: models.find(m => m.role === "synthesis")?.recommended_steps,
  };
}

export function modelName(file: string): string {
  return file.replace(/\.gguf$/, "").replace(/^acestep-v15-/, "").replace(/^acestep-5Hz-lm-/, "LM ");
}

export function modelOptions(catalog: ModelCatalog, selection: ModelSelection, role: "synthesis" | "language", compatibleOnly: boolean): CatalogModel[] {
  const field = role === "synthesis" ? "synthesis" : "language";
  return catalog.models.filter(m => m.provider === selection.provider && m.role === role &&
    (!compatibleOnly || m.file === selection[field] || profile(catalog, { ...selection, [field]: m.file }).compatible));
}
