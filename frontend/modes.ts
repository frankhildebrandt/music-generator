import { initializePro } from "./pro/controller";
import { text } from "./pro/view";

let initialized = false;
export function initializeModes(): void {
  if (initialized) return;
  initialized = true;
  const nav = document.createElement("nav"); nav.className = "studio-modes"; nav.setAttribute("aria-label", "Studio mode");
  nav.innerHTML = '<button type="button" data-mode="easy" aria-pressed="true">Easy</button><button type="button" data-mode="pro" aria-pressed="false">Pro</button>';
  document.querySelector(".titlebar-name")!.after(nav);
  const host = document.querySelector<HTMLElement>("#proStudio")!;
  let pro: Awaited<ReturnType<typeof initializePro>> | null = null;
  let loading: Promise<void> | null = null;
  const show = async (mode: string) => {
    document.body.dataset.mode = mode;
    document.dispatchEvent(new CustomEvent("studio-mode", { detail: mode }));
    pro?.pause();
    document.querySelector<HTMLElement>("#studio")!.hidden = mode === "pro";
    host.hidden = mode !== "pro";
    for (const button of nav.querySelectorAll("button")) button.setAttribute("aria-pressed", String(button.dataset.mode === mode));
    localStorage.setItem("studio-mode", mode);
    if (mode === "pro" && !pro) {
      if (!loading) {
        host.textContent = text("Pro Studio wird geöffnet …", "Opening Pro Studio …");
        loading = initializePro(host).then((instance) => { pro = instance; }).catch((error) => { host.textContent = String(error); }).finally(() => { loading = null; });
      }
      await loading;
    }
  };
  nav.addEventListener("click", (event) => { const button = (event.target as Element).closest<HTMLButtonElement>("button[data-mode]"); if (button) void show(button.dataset.mode!); });
  void show(localStorage.getItem("studio-mode") === "pro" ? "pro" : "easy");
}
