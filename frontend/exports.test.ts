import { beforeEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ settings: vi.fn(), export: vi.fn(), save: vi.fn() }));
vi.mock("./backend", () => ({ backend: mocks }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: mocks.save }));
vi.mock("./i18n.js", () => ({ t: (key: string) => `translated:${key}` }));
import { exportFolder, exportSong } from "./exports";
import type { Song } from "./types";
const song = { id: "one", title: "Rain", caption: "piano", file: "one.wav" } as Song;
beforeEach(() => { vi.resetAllMocks(); mocks.settings.mockResolvedValue({ export_format: "mp3" }); mocks.save.mockResolvedValue("/exports/Rain.mp3"); });
it("uses the selected MP3 format in the native save dialog", async () => {
  await exportSong(song);
  expect(mocks.save).toHaveBeenCalledWith({ defaultPath: "Rain.mp3", filters: [{ name: "MP3", extensions: ["mp3"] }] });
  expect(mocks.export).toHaveBeenCalledWith("audio", "/exports/Rain.mp3", "one", null);
});
it("keeps lyrics as text regardless of the audio setting", async () => {
  await exportSong(song, true);
  expect(mocks.settings).not.toHaveBeenCalled();
  expect(mocks.save.mock.calls[0][0].defaultPath).toBe("Rain.txt");
});
it("preserves folder selection and translates conversion errors", async () => {
  mocks.export.mockRejectedValue(new Error("mp3_failed"));
  await expect(exportFolder("favorites")).rejects.toThrow("translated:mp3_failed");
  expect(mocks.export).toHaveBeenCalledWith("zip", "/exports/Rain.mp3", null, "favorites");
});
