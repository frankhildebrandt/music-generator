import { expect, it } from "vitest";
import { generationSeconds } from "./song-timing";

it("excludes queue waiting time for sequential variants", () => {
  const created_at = "2026-10-23T10:00:00Z";
  expect(generationSeconds({ created_at, started_at: "2026-10-23T10:10:00Z", finished_at: "2026-10-23T10:12:00Z" })).toBe(120);
  expect(generationSeconds({ created_at, started_at: "2026-10-23T10:12:00Z", finished_at: "2026-10-23T10:13:00Z" })).toBe(60);
});

it("retains the previous display for older songs without a start timestamp", () => {
  for (const started_at of [undefined, null, ""]) {
    expect(generationSeconds({ created_at: "2026-10-23T10:00:00Z", started_at, finished_at: "2026-10-23T10:12:00Z" })).toBe(720);
  }
});

it("handles unfinished, invalid and backwards timestamps without misleading durations", () => {
  const song = { created_at: "2026-10-23T10:00:00Z", finished_at: "2026-10-23T10:00:00Z" };
  expect(generationSeconds(song)).toBe(0);
  expect(generationSeconds({ ...song, finished_at: null })).toBeNull();
  expect(generationSeconds({ ...song, started_at: "invalid" })).toBeNull();
  expect(generationSeconds({ ...song, started_at: "2026-10-23T10:01:00Z" })).toBeNull();
});
