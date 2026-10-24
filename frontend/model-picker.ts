import { listen } from "@tauri-apps/api/event";
import { backend } from "./backend";
import { bytes, element } from "./dom";
import { applyI18n, t } from "./i18n.js";
import { modelName, modelOptions, profile } from "./model-options";
import type { DownloadProgress, HeartOptions, ModelCatalog, ModelSelection } from "./types";

let catalog: ModelCatalog | undefined;
let chosen: ModelSelection | undefined;
let draft: ModelSelection;
let downloading = false;
let initialized = false;
let codecGuidance = 1.25;
const dialog = () => element<HTMLDialogElement>("#modelPicker");
const picker = <T extends HTMLElement>(id: string) => element<T>(`#${id}`);

export function generationModels(): ModelSelection | undefined {
  return chosen ? { ...chosen } : undefined;
}
export function heartOptions(): HeartOptions | undefined {
  if(chosen?.provider!=="heartmula")return undefined;
  return {temperature:Number(picker<HTMLInputElement>("heartTemperature").value),cfg_scale:Number(picker<HTMLInputElement>("heartCfg").value),top_k:Number(picker<HTMLInputElement>("heartTopK").value),codec_steps:Number(picker<HTMLInputElement>("heartSteps").value),codec_guidance:codecGuidance};
}
export function restoreHeartOptions(options?: HeartOptions): void {
  codecGuidance=options?.codec_guidance??1.25;
  for(const [id,value] of Object.entries({heartTemperature:options?.temperature??1,heartCfg:options?.cfg_scale??1.5,heartTopK:options?.top_k??50,heartSteps:options?.codec_steps??10}))picker<HTMLInputElement>(id).value=String(value);
}

// Reuse the saved profile in memory without modifying the song or its sampling values.
export function restoreSongModels(selection?: ModelSelection): void {
  if (!selection) return;
  chosen = { ...selection };
  compact();
}

function compact(): void {
  picker("currentModels").textContent = chosen ? `${chosen.provider==="heartmula"?"HeartMuLa · ":""}${modelName(chosen.synthesis)}${chosen.language ? ` · ${modelName(chosen.language)}` : ""}` : t("models");
  const heart=chosen?.provider==="heartmula";
  document.body.dataset.provider=heart?"heartmula":"ace-step";
  picker("heartControls").hidden=!heart;
  for(const input of picker("heartControls").querySelectorAll<HTMLInputElement>("input")) { input.disabled=!heart;input.required=heart; }
  picker("lyricsDice").hidden=heart;
  picker("lyricsBox").title=t(heart?"heart_lyrics":"lyrics_tip");
  picker<HTMLInputElement>("fileInput").disabled=heart;
  const record=picker<HTMLButtonElement>("recBtn");record.disabled=heart&&record.getAttribute("aria-pressed")!=="true";
  record.title=t(heart?"heart_analysis":"rec_aria");
  picker("dropzone").title=t(heart?"heart_analysis":"drop_title");
  picker("dropzone").setAttribute("aria-disabled",String(heart));
  picker("dropzone").tabIndex=heart?-1:0;
}

async function refresh(): Promise<void> {
  catalog = await backend.modelCatalog();
  chosen ??= { ...catalog.selection };
  compact();
}

async function loadSelection(): Promise<void> {
  // Startup keeps its saved-setup fast path: inspect model files only when opening
  // the picker, starting an operation or reacting to an actual model change.
  const settings = await backend.settings();
  chosen = { provider: settings.model_provider || "ace-step", synthesis: settings.synthesis_model || (settings.dit === "xl" ? "acestep-v15-xl-sft-Q8_0.gguf" : "acestep-v15-sft-Q8_0.gguf"), language: settings.lm_model };
  compact();
}

function render(): void {
  if (!catalog) return;
  const provider = picker<HTMLSelectElement>("modelProvider");
  provider.replaceChildren(...catalog.providers.map(p => new Option(`${p.name}${p.status === "planned" ? ` · ${t("picker_planned")}` : ""}`, p.id)));
  provider.value = draft.provider;
  const planned = !catalog.providers.some(p => p.id === draft.provider && p.status === "available");
  const heart=draft.provider==="heartmula";
  picker("modelProviderHint").hidden = !planned && !heart;
  picker("modelProviderHint").textContent=t(heart?"heart_hint":"picker_provider_hint");
  const only = picker<HTMLInputElement>("modelsCompatible").checked;
  for (const role of ["synthesis", "language"] as const) {
    const select = picker<HTMLSelectElement>(role === "synthesis" ? "modelSynthesis" : "modelLanguage");
    select.replaceChildren();
    if(heart&&role==="language") { select.add(new Option(t("heart_integrated"),""));select.disabled=true;continue; }
    const groups = new Map<string, HTMLOptGroupElement>();
    for (const model of modelOptions(catalog, draft, role, only)) {
      let group = groups.get(model.family);
      if (!group) { group = document.createElement("optgroup"); group.label = modelName(model.family); groups.set(model.family, group); select.append(group); }
      const fits = profile(catalog, { ...draft, [role]: model.file }).compatible;
      const status = model.verified ? t("picker_installed") : model.present ? t("picker_verify") : t("picker_missing");
      group.append(new Option(`${modelName(model.file)} · ${bytes(model.estimated_bytes)} · ${status}${fits ? "" : ` · ${t("picker_low_ram")}`}`, model.file));
    }
    select.value = draft[role];
    select.disabled = planned || downloading;
  }
  const state = profile(catalog, draft);
  picker("modelHardware").textContent = `${catalog.hardware.gpu} · RAM ${bytes(catalog.hardware.ram_bytes)}${catalog.hardware.vram_bytes ? ` · VRAM ${bytes(catalog.hardware.vram_bytes)}` : ""}`;
  picker("modelEstimate").textContent = planned ? "" : `${t("picker_memory")}: ${bytes(state.memory)} · ${t("picker_download")}: ${bytes(state.download)} · ${t("desktop_space")}: ${catalog.free_bytes === null ? "—" : bytes(catalog.free_bytes)}`;
  picker("modelFolder").textContent = catalog.models_dir;
  picker("modelMemoryHint").hidden = planned;
  picker("modelRamWarning").hidden = planned || state.compatible;
  picker("modelCpuHint").hidden = planned || !catalog.hardware.cpu_only;
  picker("modelDependencies").textContent = heart?t("heart_integrated"):planned ? "" : `${t("picker_dependencies")}: ${state.models.filter(m => m.role === "dependency").map(m => `${m.file} (${m.verified ? t("picker_installed") : m.present ? t("picker_verify") : t("picker_missing")})`).join(" · ")}`;
  const apply = picker<HTMLButtonElement>("modelApply");
  apply.textContent = state.verified ? t("picker_use") : state.download ? t("picker_download_use") : t("picker_verify_use");
  const existing = draft.provider === catalog.selection.provider && draft.synthesis === catalog.selection.synthesis && draft.language === catalog.selection.language;
  apply.disabled = downloading || !state.valid || (!state.compatible && !existing) || (catalog.free_bytes !== null && state.download > 0 && state.download + 512 * 1024 ** 2 > catalog.free_bytes);
  picker<HTMLSelectElement>("modelProvider").disabled = downloading;
  picker<HTMLButtonElement>("modelHardwareTest").disabled = downloading;
  picker<HTMLInputElement>("modelsCompatible").disabled = downloading;
  picker<HTMLButtonElement>("modelCancel").hidden = !downloading;
  picker<HTMLButtonElement>("modelRecommended").disabled = downloading || !state.valid;
  picker("modelStepHint").textContent = state.steps ? `${t(heart?"heart_steps":"iterations")}: ${state.steps}` : "";
}

async function open(): Promise<void> {
  picker("modelError").textContent = "";
  try {
    if (!downloading) { await refresh(); draft = { ...chosen! }; render(); }
    if (!dialog().open) dialog().showModal();
  } catch (error) {
    picker("modelError").textContent = String(error);
    if (!dialog().open) dialog().showModal();
  }
}

async function apply(): Promise<void> {
  downloading = true;
  document.body.dataset.modelDownload = "true";
  picker("modelError").textContent = "";
  picker("modelProgressArea").hidden = false;
  const bar = picker<HTMLProgressElement>("modelProgress");
  bar.hidden = false; bar.removeAttribute("value");
  picker("modelProgressText").textContent = t("desktop_verifying");
  render();
  try {
    chosen = await backend.useModels({ ...draft });
    bar.max = 1; bar.value = 1;
    picker("modelProgressText").textContent = t("picker_selected");
  } catch (error) { picker("modelError").textContent = String(error); }
  finally {
    downloading = false;
    document.body.dataset.modelDownload = "false";
    try { await refresh(); } catch (error) { picker("modelError").textContent = String(error); }
    render();
  }
}

export async function initializeModelPicker(): Promise<void> {
  if (initialized) { await loadSelection(); return; }
  initialized = true;
  picker<HTMLButtonElement>("openModels").onclick = () => { void open(); };
  picker<HTMLButtonElement>("modelClose").onclick = () => dialog().close();
  picker<HTMLButtonElement>("modelApply").onclick = () => { void apply(); };
  picker<HTMLButtonElement>("modelCancel").onclick = () => { void backend.cancelModelDownload().catch(error => { picker("modelError").textContent = String(error); }); };
  picker<HTMLButtonElement>("modelHardwareTest").onclick = () => { if (!downloading) { dialog().close(); document.dispatchEvent(new Event("manage-models")); } };
  picker<HTMLSelectElement>("modelProvider").onchange = () => {
    const id=picker<HTMLSelectElement>("modelProvider").value;
    const provider=catalog!.providers.find(p=>p.id===id)!;
    draft=chosen?.provider===id?{...chosen}:catalog!.profiles?.[id]?{...catalog!.profiles[id]}:{provider:id,synthesis:provider.default_synthesis||"",language:provider.default_language||""};render();
  };
  for (const role of ["synthesis", "language"] as const) {
    picker<HTMLSelectElement>(role === "synthesis" ? "modelSynthesis" : "modelLanguage").onchange = (event) => { draft[role] = (event.target as HTMLSelectElement).value; render(); };
  }
  picker<HTMLInputElement>("modelsCompatible").onchange = render;
  picker<HTMLButtonElement>("modelRecommended").onclick = () => {
    if (!catalog) return;
    const steps = profile(catalog, draft).steps;
    if (steps && chosen?.synthesis === draft.synthesis && chosen.provider === draft.provider) {
      const input = chosen.provider==="heartmula"?picker<HTMLInputElement>("heartSteps"):element<HTMLInputElement>('#genForm [name="inference_steps"]');
      input.value = String(steps); input.dispatchEvent(new Event("input", { bubbles: true }));
      picker("modelProgressText").textContent = `${t("iterations")}: ${steps}`;
      picker("modelProgressArea").hidden = false;
      picker("modelProgress").hidden = true;
    } else { picker("modelError").textContent = t("picker_select_first"); }
  };
  document.addEventListener("langchange", () => { compact(); if (catalog && draft) render(); });
  document.addEventListener("open-model-picker", () => { void open(); });
  await listen<DownloadProgress>("model-download-progress", ({ payload: p }) => {
    const bar = picker<HTMLProgressElement>("modelProgress");
    bar.max = Math.max(1, p.total); bar.value = p.downloaded;
    const speed = new Intl.NumberFormat(document.documentElement.lang, { maximumFractionDigits: 1 }).format(p.bytes_per_second / 1024 ** 2);
    picker("modelProgressText").textContent = `${p.file} · ${t(p.phase === "verifying" ? "desktop_verifying" : "desktop_downloading")} ${bytes(p.downloaded)} / ${bytes(p.total)}${p.bytes_per_second ? ` · ${speed} MiB/s` : ""}`;
  });
  await listen("models-changed", () => { if (!downloading) void (dialog().open ? refresh() : loadSelection()).catch(() => {}); });
  applyI18n();
  await loadSelection();
}
