import { audible, RATE, type Project, type Take } from "./model";

interface Channel { input: GainNode; left: GainNode; right: GainNode; output: ChannelMergerNode }
export class ArrangementAudio {
  private context = new AudioContext({ sampleRate: RATE });
  private master = this.context.createGain();
  private analyser = this.context.createAnalyser();
  private buffers = new Map<string, AudioBuffer>();
  private loading = new Map<string, Promise<AudioBuffer>>();
  private channels = new Map<string, Channel>();
  private sources: AudioBufferSourceNode[] = [];
  private project: Project | null = null;
  private epoch = 0;
  private anchor = 0;
  private position = 0;
  private playing = false;
  private sequence = 0;
  private loop: [number, number] | null = null;
  private loopTimer: ReturnType<typeof setInterval> | null = null;
  private nextLoop = 0;
  private meterSamples = new Float32Array(1024);

  constructor() { this.master.connect(this.analyser); this.analyser.connect(this.context.destination); }
  get isPlaying(): boolean { return this.playing; }
  get frame(): number {
    if (!this.playing) return this.position;
    const elapsed = Math.max(0, Math.round((this.context.currentTime - this.epoch) * RATE));
    if (this.loop && elapsed >= this.loop[1] - this.anchor) return this.loop[0] + (elapsed - (this.loop[1] - this.anchor)) % (this.loop[1] - this.loop[0]);
    return Math.min(this.project?.duration ?? 0, this.anchor + elapsed);
  }
  get peak(): number {
    this.analyser.getFloatTimeDomainData(this.meterSamples);
    return this.meterSamples.reduce((peak, x) => Math.max(peak, Math.abs(x)), 0);
  }
  private async buffer(take: Take): Promise<AudioBuffer> {
    const cached = this.buffers.get(take.id);
    if (cached) return cached;
    const pending = this.loading.get(take.id);
    if (pending) return pending;
    if (!take.url) throw new Error("Audio file unavailable; reopen this project");
    const load = (async () => {
      const response = await fetch(take.url!);
      if (!response.ok) throw new Error(`Cannot read ${take.name}`);
      const buffer = await this.context.decodeAudioData(await response.arrayBuffer());
      this.buffers.set(take.id, buffer);
      return buffer;
    })();
    this.loading.set(take.id, load);
    try { return await load; } finally { this.loading.delete(take.id); }
  }
  async prepare(project: Project): Promise<void> {
    const ids = new Set(project.clips.map((clip) => clip.take_id));
    const takes = project.takes.filter((take) => ids.has(take.id));
    if (takes.reduce((frames, take) => frames + take.frames, 0) > RATE * 60 * 12) throw new Error("Audio memory budget exceeded; use shorter takes");
    for (const id of this.buffers.keys()) if (!ids.has(id)) this.buffers.delete(id);
    await Promise.all(takes.map((take) => this.buffer(take)));
  }
  update(project: Project): void {
    this.project = project;
    for (const track of project.tracks) {
      const channel = this.channels.get(track.id);
      if (!channel) continue;
      const gain = audible(track, project.tracks) ? track.gain : 0;
      channel.left.gain.setTargetAtTime(gain * Math.min(1, 1 - track.pan), this.context.currentTime, 0.008);
      channel.right.gain.setTargetAtTime(gain * Math.min(1, 1 + track.pan), this.context.currentTime, 0.008);
    }
  }
  setLoop(loop: [number, number] | null): void {
    const frame = this.frame; const playing = this.playing;
    this.loop = loop && loop[0] >= 0 && loop[1] - loop[0] >= RATE / 100 && loop[1] <= (this.project?.duration ?? Infinity) ? loop : null;
    if (playing && this.project) void this.play(this.project, frame).catch(() => this.pause());
  }
  private scheduleLoop(): void {
    if (this.loopTimer) clearInterval(this.loopTimer);
    const schedule = () => {
      if (this.loop) {
        const duration = (this.loop[1] - this.loop[0]) / RATE;
        if (this.nextLoop < this.context.currentTime - 0.1) this.nextLoop += Math.ceil((this.context.currentTime - this.nextLoop) / duration) * duration;
        while (this.nextLoop < this.context.currentTime + 0.5) {
          this.scheduleWindow(this.loop[0], this.loop[1], this.nextLoop);
          this.nextLoop += duration;
        }
      } else if (this.project && this.frame >= this.project.duration) this.pause();
    };
    schedule(); this.loopTimer = setInterval(schedule, 100);
  }
  async play(project: Project, frame = this.position): Promise<void> {
    this.pause();
    const sequence = ++this.sequence;
    await this.context.resume();
    await this.prepare(project);
    if (sequence !== this.sequence) return;
    this.project = project;
    this.channels.clear();
    for (const track of project.tracks) {
      const input = this.context.createGain(); const splitter = this.context.createChannelSplitter(2);
      const left = this.context.createGain(); const right = this.context.createGain(); const merge = this.context.createChannelMerger(2);
      input.channelCount = 2; input.channelCountMode = "explicit";
      input.connect(splitter); splitter.connect(left, 0); splitter.connect(right, 1);
      left.connect(merge, 0, 0); right.connect(merge, 0, 1); merge.connect(this.master);
      this.channels.set(track.id, { input, left, right, output: merge });
    }
    this.update(project);
    this.anchor = Math.max(0, frame >= project.duration ? 0 : frame);
    if (this.loop && (this.anchor < this.loop[0] || this.anchor >= this.loop[1])) this.anchor = this.loop[0];
    this.epoch = this.context.currentTime + 0.04;
    this.position = this.anchor;
    const end = this.loop?.[1] ?? project.duration;
    this.scheduleWindow(this.anchor, end, this.epoch);
    this.nextLoop = this.epoch + (end - this.anchor) / RATE;
    this.playing = true;
    this.scheduleLoop();
  }
  private scheduleWindow(start: number, end: number, when: number): void {
    for (const clip of this.project!.clips) {
      const clipEnd = Math.min(clip.start + clip.length, end);
      if (clipEnd <= start || clip.start >= end) continue;
      const buffer = this.buffers.get(clip.take_id); const channel = this.channels.get(clip.track_id);
      if (!buffer || !channel) continue;
      const skip = Math.max(0, start - clip.start);
      const source = this.context.createBufferSource(); source.buffer = buffer;
      const gain = this.context.createGain(); gain.gain.value = clip.gain;
      source.connect(gain); gain.connect(channel.input);
      source.start(when + Math.max(0, clip.start - start) / RATE, (clip.offset + skip) / RATE, (clipEnd - clip.start - skip) / RATE);
      this.sources.push(source);
      source.onended = () => { source.disconnect(); gain.disconnect(); this.sources = this.sources.filter((item) => item !== source); };
    }
  }
  pause(): void {
    ++this.sequence;
    this.position = this.frame;
    this.playing = false;
    if (this.loopTimer) clearInterval(this.loopTimer);
    for (const source of this.sources) { try { source.stop(); } catch { /* An ended source is already stopped. */ } source.disconnect(); }
    this.sources = [];
    for (const channel of this.channels.values()) { channel.input.disconnect(); channel.left.disconnect(); channel.right.disconnect(); channel.output.disconnect(); }
    this.channels.clear();
  }
  seek(frame: number): void { this.position = frame; }
  stop(): void { this.pause(); this.position = 0; }
  async waveform(take: Take, width = 220, offset = 0, length = take.frames): Promise<number[]> {
    const buffer = await this.buffer(take); const channel = buffer.getChannelData(0);
    return Array.from({ length: width }, (_, column) => {
      let peak = 0; const start = Math.min(channel.length, offset + Math.floor(column * length / width)); const end = Math.min(channel.length, offset + Math.floor((column + 1) * length / width));
      for (let n = start; n < end; n += Math.max(1, Math.floor((end - start) / 120))) peak = Math.max(peak, Math.abs(channel[n]));
      return peak;
    });
  }
}
