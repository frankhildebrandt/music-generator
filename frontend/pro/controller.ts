import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { askName } from "../dialogs";
import { backend } from "../backend";
import { proApi } from "./api";
import { ArrangementAudio } from "./audio";
import { bindGestures } from "./gestures";
import { newTrack, RATE, splitClip, trimClip, type Clip, type Generation, type Project } from "./model";
import { esc, jobs, ruler, shell, text, workspace, type ViewState } from "./view";

export async function initializePro(host: HTMLElement): Promise<{ pause: () => void }> {
  const audio = new ArrangementAudio();
  const state: ViewState = { project: null, projects: [], track: null, clip: null, jobs: [], zoom: 32, start: 0, end: RATE * 16, installed: false, status: "" };
  const drafts = new Map<string, { prompt: string; seed: string; steps: string; variants: string }>();
  let snap = true; let loop = false; let tail = Promise.resolve(); let timer = 0; let loadingModel = false; let pendingRender = false; let refreshing = false;
  host.innerHTML = shell();
  const field = <T extends HTMLElement>(selector: string) => host.querySelector<T>(selector)!;
  const input = (id: string) => field<HTMLInputElement>(`#${id}`);
  function message(value: string): void { state.status = value; field("#proMessage").textContent = value; }
  function run(operation: () => Promise<unknown>): void { tail = tail.then(operation).then(() => {}).catch((error) => { message(String(error)); }); }
  function rememberDraft(): void {
    const track = host.querySelector<HTMLElement>("#proPrompt")?.dataset.draftTrack;
    if (track) drafts.set(track, { prompt: input("proPrompt").value, seed: input("proSeed").value, steps: input("proSteps").value, variants: input("proVariants").value });
  }
  async function edit(change: (project: Project) => void, redraw = true): Promise<void> {
    if (!state.project) return;
    const candidate = structuredClone(state.project); change(candidate);
    const arrangementChanged = JSON.stringify(candidate.clips) !== JSON.stringify(state.project.clips) || candidate.duration !== state.project.duration;
    const playing = audio.isPlaying; const position = audio.frame;
    state.project = await proApi.save(candidate);
    state.projects = state.projects.map((project) => project.id === state.project!.id ? { ...project, name: state.project!.name } : project);
    audio.update(state.project);
    if (redraw) render(); else syncMetadata();
    if (playing && arrangementChanged) await audio.play(state.project, position);
  }
  async function reopen(id: string): Promise<void> {
    audio.stop(); loop = false; audio.setLoop(null); state.project = await proApi.open(id); state.jobs = await proApi.jobs(id);
    state.track = state.project.tracks[0]?.id ?? null; state.clip = null; state.start = 0; state.end = state.project.duration;
    localStorage.setItem("pro-project", id); state.projects = await proApi.list(); render();
  }
  function render(): void {
    pendingRender = false;
    rememberDraft();
    const previousScroll = host.querySelector(".pro-scroll"); const left = previousScroll?.scrollLeft ?? 0; const top = previousScroll?.scrollTop ?? 0;
    const selector = field<HTMLSelectElement>("#proProject");
    selector.innerHTML = state.projects.map((p) => `<option value="${p.id}" ${p.id === state.project?.id ? "selected" : ""}>${esc(p.name)}</option>`).join("");
    field("#proWorkspace").innerHTML = workspace(state);
    const scroll = host.querySelector(".pro-scroll"); if (scroll) { scroll.scrollLeft = left; scroll.scrollTop = top; }
    field("#proModel").textContent = state.installed ? text("● Base-Modell bereit · lokal", "● Base model ready · local") : text("Spurgenerierung benötigt das Base-Modell. Import und Mixing sind bereits verfügbar.", "Stem generation requires the Base model. Import and mixing are already available.");
    field<HTMLButtonElement>('[data-action="install"]').hidden = state.installed || loadingModel;
    field<HTMLButtonElement>('[data-action="cancel-download"]').hidden = !loadingModel;
    const draft = state.track ? drafts.get(state.track) : null;
    if (draft && host.querySelector("#proPrompt")) { input("proPrompt").value = draft.prompt; input("proSeed").value = draft.seed; input("proSteps").value = draft.steps; input("proVariants").value = draft.variants; }
    else if (host.querySelector("#proPrompt")) input("proPrompt").value = state.project?.tracks.find((t) => t.id === state.track)?.prompt ?? "";
    if (host.querySelector("#proSnap")) { input("proSnap").checked = snap; input("proLoop").checked = loop; }
    paintWaveforms();
  }
  function syncMetadata(): void {
    const p = state.project; if (!p) return;
    const saved = host.querySelector(".pro-saved"); if (saved) saved.textContent = `${text("Automatisch gespeichert", "Autosaved")} · r${p.revision}`;
    const option = field<HTMLSelectElement>("#proProject").selectedOptions[0]; if (option) option.textContent = p.name;
    const canvas = host.querySelector<HTMLElement>(".pro-canvas");
    if (canvas) { canvas.style.setProperty("--timeline-width", `${Math.max(700, p.duration / RATE * state.zoom)}px`); canvas.style.setProperty("--beat-width", `${60 / p.bpm * state.zoom}px`); }
    const grid = host.querySelector(".pro-ruler"); if (grid) grid.innerHTML = ruler(p, state.zoom);
    for (const track of p.tracks) {
      const row = host.querySelector(`[data-track="${track.id}"]`);
      const name = row?.querySelector(".pro-track-name"); if (name) name.innerHTML = `<i></i>${esc(track.name)}`;
      const instrument = row?.querySelector(".pro-track-switches span"); if (instrument) instrument.textContent = track.instrument;
    }
  }
  function paintWaveforms(): void {
    const project = state.project; if (!project) return;
    void audio.prepare(project).catch((error) => message(String(error)));
    for (const clip of project.clips) {
      const take = project.takes.find((t) => t.id === clip.take_id); const svg = host.querySelector<SVGElement>(`[data-wave="${clip.id}"]`);
      if (!take || !svg) continue;
      void audio.waveform(take, 220, clip.offset, clip.length).then((values) => {
        if (!svg.isConnected) return;
        svg.innerHTML = `<path d="${values.map((peak, x) => `M${x} ${30 - Math.min(1, peak) * 26}v${Math.max(1, Math.min(1, peak) * 52)}`).join(" ")}"/>`;
      }).catch(() => { svg.innerHTML = '<text x="4" y="30">Audio unavailable</text>'; });
    }
  }
  function select(clip: Clip): void { rememberDraft(); state.clip = clip.id; state.track = clip.track_id; state.start = clip.start; state.end = clip.start + clip.length; render(); }
  bindGestures(host, { project: () => state.project, zoom: () => state.zoom, snap: () => snap, select, commit: (change) => run(() => edit(change)) });

  async function generate(range: boolean): Promise<void> {
    const p = state.project; if (!p || !state.track) return;
    rememberDraft(); const draft = drafts.get(state.track)!;
    const clip = p.clips.find((c) => c.id === state.clip);
    if (range && (!clip || state.start < clip.start || state.end > clip.start + clip.length)) throw new Error(text("Wähle einen Bereich innerhalb eines Clips.", "Select a range inside one clip."));
    const request: Generation = { track_id: state.track, clip_id: range ? clip!.id : null, prompt: draft.prompt, seed: Number(draft.seed), steps: Number(draft.steps), variants: Number(draft.variants), start: range ? state.start : 0, end: range ? state.end : p.duration };
    await proApi.generate(p.id, request); state.jobs = await proApi.jobs(p.id); render(); message(text("Generierung eingereiht. Der aktuelle Take bleibt aktiv.", "Generation queued. The current take stays active."));
  }
  async function activate(id: string): Promise<void> {
    const p = state.project; const take = p?.takes.find((t) => t.id === id); if (!p || !take) return;
    const generation = take.request.generation as Generation | undefined;
    await edit((project) => {
      if (generation?.clip_id) {
        const clip = project.clips.find((c) => c.id === generation.clip_id);
        if (!clip || clip.track_id !== take.track_id || clip.offset + clip.length > take.frames) throw new Error(text("Der ursprüngliche Clip wurde verändert. Stelle ihn mit Undo wieder her.", "The original clip has changed. Restore it with Undo."));
        clip.take_id = take.id;
      } else {
        project.clips = project.clips.filter((c) => c.track_id !== take.track_id);
        project.clips.push({ id: crypto.randomUUID(), track_id: take.track_id, take_id: take.id, start: 0, offset: 0, length: Math.min(take.frames, project.duration), gain: 1 });
      }
    });
  }

  host.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    const control = target?.closest<HTMLButtonElement>("[data-action]");
    if (!control || control.disabled) return;
    const action = control.dataset.action!; const id = control.dataset.id;
    // WebKit needs resume() in the user gesture, before entering the async edit queue.
    if (action === "play") {
      if (state.project) {
        if (audio.isPlaying) audio.pause();
        else void audio.play(state.project).catch((error) => message(String(error)));
      }
      return;
    }
    if (action === "stop") { audio.stop(); return; }
    if (action === "install") { void install(); return; }
    if (action === "cancel-download") { void backend.cancelSetup(); return; }
    run(async () => {
      if (action === "new") { const name = await askName(text("Projektname", "Project name"), "New session"); if (name?.trim()) await reopen((await proApi.create(name, 120, "A minor", RATE * 16)).id); return; }
      const p = state.project; if (!p) return;
      if (action === "seek") { const rect = control.getBoundingClientRect(); const frame = Math.min(p.duration, Math.max(0, (event.clientX - rect.left) / state.zoom * RATE)); if (audio.isPlaying) await audio.play(p, frame); else audio.seek(frame); }
      if (action === "undo" || action === "redo") { state.project = await proApi.history(p.id, action === "redo"); audio.pause(); render(); }
      if (action === "select-track") { rememberDraft(); state.track = id!; state.clip = null; render(); }
      if (action === "add-track") await edit((project) => { const track = newTrack(field<HTMLSelectElement>("#proInstrument").value, project.tracks.length); project.tracks.push(track); rememberDraft(); state.track = track.id; state.clip = null; });
      if (action === "remove-track") await edit((project) => { project.tracks = project.tracks.filter((t) => t.id !== state.track); project.clips = project.clips.filter((c) => c.track_id !== state.track); state.track = project.tracks[0]?.id ?? null; state.clip = null; });
      if (action === "import") { const path = await open({ filters: [{ name: "WAV", extensions: ["wav"] }] }); if (typeof path === "string") { audio.pause(); state.project = await proApi.import(p.id, path); state.track = state.project.tracks.at(-1)?.id ?? null; render(); } }
      if (action === "library") await importLibrary();
      if (action === "export" || action === "stems") { const stems = action === "stems"; const path = await save({ defaultPath: `${p.name}.${stems ? "zip" : "wav"}`, filters: [{ name: stems ? "ZIP" : "WAV", extensions: [stems ? "zip" : "wav"] }] }); if (path) { await proApi.export(p.id, path, stems); message(text("Export gespeichert.", "Export saved.")); } }
      if (action === "generate-track" || action === "generate-range") await generate(action === "generate-range");
      if (action === "take" && id) await activate(id);
      if (action === "cancel" && id) await proApi.cancel(id);
      if (action === "retry" && id) await proApi.retry(id);
      const clip = p.clips.find((c) => c.id === state.clip);
      if (clip && ["split", "trim", "duplicate", "delete-clip"].includes(action)) await edit((project) => {
        const index = project.clips.findIndex((c) => c.id === clip.id);
        if (action === "split") project.clips.splice(index, 1, ...splitClip(clip, audio.frame));
        if (action === "trim") project.clips[index] = trimClip(clip, state.start, state.end);
        if (action === "duplicate") { const copy = { ...clip, id: crypto.randomUUID(), start: clip.start + clip.length }; if (copy.start + copy.length > project.duration) throw new Error(text("Verlängere zuerst das Projekt.", "Extend the project first.")); project.clips.push(copy); }
        if (action === "delete-clip") { project.clips.splice(index, 1); state.clip = null; }
      });
    });
  });

  host.addEventListener("input", (event) => {
    const element = event.target as HTMLInputElement; const track = state.project?.tracks.find((t) => t.id === element.closest<HTMLElement>("[data-track]")?.dataset.track);
    if (track && (element.dataset.mix === "gain" || element.dataset.mix === "pan")) { track[element.dataset.mix] = Number(element.value); audio.update(state.project!); }
  });
  host.addEventListener("change", (event) => {
    const element = event.target as HTMLInputElement;
    if (element.id === "proRangeStart" || element.id === "proRangeEnd") { state.start = Math.round(Number(input("proRangeStart").value) * RATE); state.end = Math.round(Number(input("proRangeEnd").value) * RATE); audio.setLoop(loop && state.end > state.start ? [state.start, state.end] : null); return; }
    if (element.id === "proLoop") { loop = element.checked; audio.setLoop(loop && state.end > state.start ? [state.start, state.end] : null); return; }
    if (element.id === "proSnap") { snap = element.checked; return; }
    if (element.id === "proZoom") { state.zoom = Number(element.value); render(); return; }
    if (element.id === "proProject") { run(() => reopen(element.value)); return; }
    if (element.id === "proPrompt") { const value = element.value; const id = element.dataset.draftTrack; run(() => edit((p) => { const track = p.tracks.find((t) => t.id === id); if (track) track.prompt = value; }, false)); return; }
    const value = element.value; const trackId = element.closest<HTMLElement>("[data-track]")?.dataset.track;
    if (element.dataset.project) run(() => edit((p) => { const key = element.dataset.project!; if (key === "name" || key === "key") p[key] = value; else if (key === "bpm" || key === "beats_per_bar") p[key] = Number(value); else if (key === "duration") p.duration = Math.round(Number(value) * RATE); }, false));
    if (element.dataset.trackField) run(() => edit((p) => { const track = p.tracks.find((t) => t.id === state.track); if (track) { if (element.dataset.trackField === "name") track.name = value; else track.instrument = value; } }, false));
    if (trackId && (element.dataset.mix === "gain" || element.dataset.mix === "pan")) run(() => edit((p) => { const track = p.tracks.find((t) => t.id === trackId); if (track) track[element.dataset.mix as "gain" | "pan"] = Number(value); }, false));
  });
  host.addEventListener("click", (event) => {
    const control = (event.target as Element).closest<HTMLElement>("button[data-mix]");
    const trackId = control?.closest<HTMLElement>("[data-track]")?.dataset.track; const key = control?.dataset.mix;
    if (trackId && (key === "mute" || key === "solo" || key === "context")) run(() => edit((p) => { const track = p.tracks.find((t) => t.id === trackId); if (track) track[key] = !track[key]; }));
  });

  async function install(): Promise<void> {
    if (loadingModel) return; loadingModel = true; document.body.dataset.settingUp = "true"; render();
    try { await proApi.install(); state.installed = (await proApi.model()).installed; message(text("Base-Modell bereit.", "Base model ready.")); }
    catch (error) { message(String(error)); }
    finally { loadingModel = false; delete document.body.dataset.settingUp; render(); }
  }
  async function importLibrary(): Promise<void> {
    const songs = (await backend.songs()).filter((song) => song.status === "done" && song.path);
    const dialog = document.createElement("dialog"); dialog.className = "pro-library";
    dialog.innerHTML = `<h2>${text("Aus Bibliothek übernehmen", "Import from library")}</h2><p>${text("Ein fertiger Song wird als Audiospur übernommen. Er wird dabei nicht in Instrumente getrennt.", "A finished song is imported as an audio track. This does not separate its instruments.")}</p>${songs.map((song) => `<button class="sek" data-song="${song.id}">${esc(song.title || song.caption)}</button>`).join("") || `<p>${text("Keine fertigen Songs vorhanden.", "No finished songs available.")}</p>`}<form method="dialog"><button class="sek">${text("Schließen", "Close")}</button></form>`;
    dialog.addEventListener("click", (event) => { const id = (event.target as Element).closest<HTMLElement>("[data-song]")?.dataset.song; const song = songs.find((s) => s.id === id); if (song?.path) { dialog.close(); run(async () => { state.project = await proApi.import(state.project!.id, song.path!); render(); }); } });
    dialog.addEventListener("close", () => dialog.remove()); document.body.append(dialog); dialog.showModal();
  }
  async function refresh(): Promise<void> {
    if (!state.project || host.hidden) return;
    state.jobs = await proApi.jobs(state.project.id);
    const updated = await proApi.open(state.project.id);
    if (updated.takes.length !== state.project.takes.length) { state.project.takes = updated.takes; pendingRender = true; }
    const editing = host.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select");
    if (pendingRender && !editing && !host.dataset.dragging) render();
    const el = host.querySelector("#proJobs"); if (el) el.innerHTML = jobs(state.jobs, state.track ?? undefined);
  }
  await listen("pro-changed", () => { clearTimeout(timer); timer = window.setTimeout(() => run(refresh), 150); });
  await listen<{ downloaded: number; total: number; phase: string }>("pro-download", ({ payload }) => message(`${payload.phase} · ${Math.round(payload.downloaded / Math.max(payload.total, 1) * 100)}%`));
  window.setInterval(() => { if (!host.hidden && !refreshing) { refreshing = true; run(async () => { try { await refresh(); } finally { refreshing = false; } }); } }, 3000);
  document.addEventListener("langchange", () => render());
  document.addEventListener("keydown", (event) => {
    if (host.hidden || (event.target as HTMLElement).closest('input,textarea,select,button,dialog')) return;
    if (event.code === "Space") { event.preventDefault(); if (audio.isPlaying) audio.pause(); else if (state.project) void audio.play(state.project).catch((error) => message(String(error))); }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") { event.preventDefault(); run(async () => { if (state.project) { audio.pause(); state.project = await proApi.history(state.project.id, event.shiftKey); render(); } }); }
    const clip = state.project?.clips.find((c) => c.id === (event.target as HTMLElement).dataset.clip);
    if (clip && event.code === "Enter") select(clip);
  });
  window.setInterval(() => {
    if (host.hidden) return;
    const time = host.querySelector("#proTime"); const play = host.querySelector("#proPlay"); const head = host.querySelector<HTMLElement>("#proPlayhead"); const meter = host.querySelector<HTMLElement>("#proMeter");
    const frame = audio.frame; const seconds = Math.max(0, frame / RATE);
    if (time) time.textContent = `${Math.floor(seconds / 60)}:${(seconds % 60).toFixed(2).padStart(5, "0")}`;
    const playLabel = audio.isPlaying ? "❚❚" : "▶";
    if (play && play.textContent !== playLabel) play.textContent = playLabel;
    if (head) head.style.left = `calc(230px + ${frame / RATE * state.zoom}px)`;
    if (meter) { const peak = audio.peak; meter.textContent = peak > 0.00001 ? `${(20 * Math.log10(peak)).toFixed(1)} dB` : "−∞ dB"; meter.classList.toggle("clipping", peak >= 1); }
  }, 40);
  state.installed = (await proApi.model()).installed; state.projects = await proApi.list();
  const remembered = localStorage.getItem("pro-project"); const id = state.projects.find((p) => p.id === remembered)?.id ?? state.projects[0]?.id;
  if (id) await reopen(id); else render();
  return { pause: () => audio.pause() };
}
