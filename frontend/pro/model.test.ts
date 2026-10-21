import { describe, expect, it } from "vitest";
import { audible, splitClip, trimClip, type Clip } from "./model";

describe("arrangement editing", () => {
  it("keeps mute authoritative when multiple tracks are soloed", () => {
    const tracks = [{ id: "a", mute: false, solo: true }, { id: "b", mute: true, solo: true }, { id: "c", mute: false, solo: false }];
    expect(tracks.filter((track) => audible(track, tracks)).map((track) => track.id)).toEqual(["a"]);
  });
  it("splits a trimmed clip without changing its source alignment", () => {
    const clip: Clip = { id: "clip", track_id: "bass", take_id: "take", start: 48000, offset: 24000, length: 96000, gain: 1 };
    const [left, right] = splitClip(clip, 96000);
    expect([left.start, left.offset, left.length, right.start, right.offset, right.length]).toEqual([48000, 24000, 48000, 96000, 72000, 48000]);
  });
  it("trims the left edge without slipping the remaining music", () => {
    const clip: Clip = { id: "clip", track_id: "bass", take_id: "take", start: 48000, offset: 24000, length: 96000, gain: 1 };
    expect(trimClip(clip, 72000, 120000)).toMatchObject({ start: 72000, offset: 48000, length: 48000 });
  });
});
