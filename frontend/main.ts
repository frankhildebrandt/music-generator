import "./desktop-i18n";
import { applyI18n, t } from "./i18n.js";
import { backend } from "./backend";
import { initializeDesktop } from "./desktop";
import { element } from "./dom";
import { showSetup } from "./setup";
import type { Bootstrap } from "./types";

let bootstrap: Bootstrap;
async function showStudio(): Promise<void> {
  element("#setupScreen").hidden = true; element("#studio").hidden = false;
  await import("./studio.js");
  document.dispatchEvent(new Event("studio-ready"));
}

async function start(): Promise<void> {
  element("#bootRetry").hidden = true;
  element("#bootStatus").textContent = t("desktop_welcome");
  try {
    bootstrap = await backend.bootstrap();
    if (bootstrap.needs_setup) await showSetup(bootstrap, showStudio); else await showStudio();
  } catch (error) { element("#bootStatus").textContent = String(error); element("#bootRetry").hidden = false; }
}

applyI18n();
void initializeDesktop().then(start).catch((error) => { element("#bootStatus").textContent = String(error); });
element("#bootRetry").onclick = () => { void start(); };
document.addEventListener("manage-models", () => {
  void backend.settings().then((settings) => showSetup({ ...bootstrap, settings, needs_setup: false, error: null }, showStudio));
});
