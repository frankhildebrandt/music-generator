import { RATE, snap, type Clip, type Project } from "./model";

export interface GestureHost {
  project: () => Project | null; zoom: () => number; snap: () => boolean;
  select: (clip: Clip) => void; commit: (edit: (project: Project) => void) => void;
}
export function bindGestures(host: HTMLElement, state: GestureHost): void {
  host.addEventListener("pointerdown", (event) => {
    const target = event.target instanceof Element ? event.target : null;
    const element = target?.closest<HTMLElement>("[data-clip]");
    const project = state.project();
    const clip = project?.clips.find((c) => c.id === element?.dataset.clip);
    if (!element || !clip || !project || event.button !== 0) return;
    event.preventDefault();
    host.dataset.dragging = "true";
    const original = { ...clip }; const origin = event.clientX; const edge = target?.closest<HTMLElement>("[data-edge]")?.dataset.edge;
    const take = project.takes.find((item) => item.id === original.take_id);
    element.setPointerCapture(event.pointerId);
    let edited = { ...original }; let moved = false;
    const move = (e: PointerEvent) => {
      if (Math.abs(e.clientX - origin) < 3 && !moved) return;
      moved = true;
      const frame = snap(original.start + (e.clientX - origin) / state.zoom() * RATE, project.bpm, state.snap());
      if (edge === "left") {
        const start = Math.max(0, original.start - original.offset, Math.min(frame, original.start + original.length - 480));
        edited = { ...original, start, offset: original.offset + start - original.start, length: original.length - (start - original.start) };
      } else if (edge === "right") {
        const edgeFrame = snap(original.start + original.length + (e.clientX - origin) / state.zoom() * RATE, project.bpm, state.snap());
        const end = Math.max(original.start + 480, Math.min(edgeFrame, project.duration, original.start + (take?.frames ?? original.offset + original.length) - original.offset));
        edited = { ...original, length: end - original.start };
      } else edited = { ...original, start: Math.min(frame, project.duration - original.length) };
      element.style.left = `${edited.start / RATE * state.zoom()}px`; element.style.width = `${edited.length / RATE * state.zoom()}px`;
    };
    const finish = () => {
      delete host.dataset.dragging;
      element.removeEventListener("pointermove", move); element.removeEventListener("pointerup", finish); element.removeEventListener("pointercancel", cancel);
      if (moved) state.commit((p) => { const index = p.clips.findIndex((c) => c.id === original.id); if (index >= 0) p.clips[index] = edited; });
      else state.select(original);
    };
    const cancel = () => { moved = false; element.style.left = `${original.start / RATE * state.zoom()}px`; element.style.width = `${original.length / RATE * state.zoom()}px`; finish(); };
    element.addEventListener("pointermove", move); element.addEventListener("pointerup", finish); element.addEventListener("pointercancel", cancel);
  });
}
