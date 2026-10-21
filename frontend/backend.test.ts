import { beforeEach, describe, expect, it, vi } from "vitest";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke, convertFileSrc: (path: string) => `asset:${path}` }));
import { backend } from "./backend";

describe("desktop contracts", () => {
  beforeEach(() => { invoke.mockReset(); });
  it("uses commands and converts song paths to local asset URLs", async () => {
    invoke.mockResolvedValue([{ id: "one", path: "/songs/one.wav" }]);
    const songs = await backend.songs({ q: "piano", favorites: true });
    expect(invoke).toHaveBeenCalledWith("list_songs", { filter: { q: "piano", favorites: true } });
    expect(songs[0].url).toBe("asset:/songs/one.wav");
  });
  it("preserves cancellation and generation payloads", async () => {
    invoke.mockResolvedValue({ ids: ["one"] });
    await backend.generate({ prompt: "piano", variants: 2, seed: 42 });
    expect(invoke).toHaveBeenCalledWith("generate", { request: { prompt: "piano", variants: 2, seed: 42 } });
    await backend.cancelSetup(); expect(invoke).toHaveBeenLastCalledWith("cancel_setup", undefined);
  });
  it("turns Rust errors into displayable errors", async () => {
    invoke.mockRejectedValue("Disk full");
    let failure: unknown;
    try { await backend.settings(); } catch (error) { failure = error; }
    expect(failure).toBeInstanceOf(Error);
    expect((failure as Error).message).toBe("Disk full");
  });
});
