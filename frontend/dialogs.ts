import { message } from "@tauri-apps/plugin-dialog";
import { t } from "./i18n.js";

export function notify(text: string): void { void message(text, { title: "Music Generator", kind: "error" }); }

export function askName(label: string, initial = ""): Promise<string | null> {
  return new Promise((resolve) => {
    const dialog = document.createElement("dialog");
    const form = document.createElement("form"); form.method = "dialog";
    const field = document.createElement("label"); field.className = "field"; field.textContent = label;
    const input = document.createElement("input"); input.value = initial; input.maxLength = 60; field.append(input);
    const actions = document.createElement("div"); actions.className = "submit";
    const cancel = document.createElement("button"); cancel.className = "sek"; cancel.value = "cancel"; cancel.textContent = t("cancel");
    const save = document.createElement("button"); save.className = "aktion"; save.value = "save"; save.textContent = t("save");
    actions.append(cancel, save); form.append(field, actions); dialog.append(form); document.body.append(dialog);
    dialog.addEventListener("close", () => { resolve(dialog.returnValue === "save" ? input.value : null); dialog.remove(); }, { once: true });
    dialog.showModal(); input.focus(); input.select();
  });
}
