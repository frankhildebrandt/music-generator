export interface Melody { range: number; rate: number; drift: number; legato: number }
function measureMelody(mono: Float32Array, rate: number): Melody | null {
  const step = Math.max(1, Math.round(rate / 16000)), sr = rate / step;      // auf etwa 16 kHz herunter
  const x = new Float32Array(Math.floor(mono.length / step));
  for (let i = 0; i < x.length; i++) { let a = 0; for (let k = 0; k < step; k++) a += mono[i * step + k]; x[i] = a / step; }
  const W = 1024, H = 256, tauMin = Math.floor(sr / 2500), tauMax = Math.ceil(sr / 70);   // 70 Hz (tiefe Stimme) bis 2,5 kHz (Pfeifen)
  let peak = 0; for (const v of x) peak = Math.max(peak, Math.abs(v));
  const frames: { t: number; rms: number; pitch: number | null; loud: boolean }[] = []; const d = new Float32Array(tauMax + 1);
  for (let start = 0; start + W + tauMax < x.length; start += H) {
    let e = 0; for (let i = 0; i < W; i++) e += x[start + i] ** 2;
    const rms = Math.sqrt(e / W);
    let pitch: number | null = null;
    if (rms > peak * 0.04) {
      let run = 0;
      for (let tau = 1; tau <= tauMax; tau++) {
        let sum = 0; for (let i = 0; i < W; i++) { const df = x[start + i] - x[start + i + tau]; sum += df * df; }
        run += sum; d[tau] = run ? (sum * tau) / run : 1;            // kumulierte mittlere normierte Differenz
      }
      for (let tau = tauMin; tau < tauMax; tau++) {
        if (d[tau] < 0.15) { while (tau + 1 < tauMax && d[tau + 1] < d[tau]) tau++; pitch = 69 + 12 * Math.log2(sr / tau / 440); break; }
      }
    }
    frames.push({ t: start / sr, rms, pitch, loud: rms > peak * 0.04 });
  }
  const voiced = frames.filter((f): f is typeof f & {pitch: number} => f.pitch !== null);
  if (voiced.length < 10) return null;
  const ps = voiced.map((f) => f.pitch).sort((a, b) => a - b), q = (p: number) => ps[Math.floor(p * (ps.length - 1))];
  // Tonwechsel: Sprung um mehr als einen Dreiviertelton gegenüber der laufenden Note
  let notes = 1, cur = voiced[0].pitch;
  for (const f of voiced) if (Math.abs(f.pitch - cur) > 0.75) { notes++; cur = f.pitch; }
  const n = voiced.length, mt = voiced.reduce((a, f) => a + f.t, 0) / n, mp = voiced.reduce((a, f) => a + f.pitch, 0) / n;
  const slope = voiced.reduce((a, f) => a + (f.t - mt) * (f.pitch - mp), 0) / Math.max(1e-9, voiced.reduce((a, f) => a + (f.t - mt) ** 2, 0));
  const dur = frames.length * H / sr;
  // gebunden = Anteil der klingenden Stellen mit fester Tonhöhe (Pausen zählen nicht, Konsonanten und Atem schon)
  return { range: q(0.95) - q(0.05), rate: notes / dur, drift: slope * dur, legato: voiced.length / Math.max(1, frames.filter((f) => f.loud).length) };
}
const MOODS = ["magical", "mysterious", "whimsical", "dreamy", "melancholic", "nostalgic", "playful", "gentle", "tender", "hopeful", "haunting",
  "warm", "calm", "peaceful", "uplifting", "dark", "sad", "joyful", "epic", "triumphant", "eerie", "romantic", "serene", "wistful",
  "cheerful", "energetic", "intense", "bittersweet", "soothing", "somber", "lighthearted"];
function describeIdea(m: Melody, caption: string, key: string) {
  const minor = /minor/i.test(key || "");
  const pace = m.rate < 2 ? "slow" : m.rate < 3.5 ? "moderately paced" : "lively";
  const shape = m.drift < -2 ? "descending" : m.drift > 2 ? "rising" : "gently circling";
  const width = m.range < 5 ? "narrow" : m.range > 9 ? "wide" : "medium-range";
  const smooth = m.legato > 0.6;   // Summen hat Atem- und Konsonantenlücken, 0,6 ist schon gebunden
  const like = m.rate < 2 && smooth ? (minor || m.drift < -2 ? "like a gentle lullaby or a film ballad theme" : "like a tender lullaby or a folk ballad")
    : m.rate < 3.5 ? (smooth ? "like a heartfelt ballad melody" : "like a simple folk tune")
    : smooth ? "like a soaring pop melody" : minor ? "like a driving, rhythmic theme" : "like a cheerful dance tune";
  const low = (caption || "").toLowerCase();
  const moods = MOODS.filter((w) => new RegExp(`\\b${w}\\b`).test(low)).slice(0, 2);
  return `${pace[0].toUpperCase() + pace.slice(1)} melody with a ${width}, ${shape} line and ${smooth ? "smooth, flowing" : "detached, rhythmic"} phrasing, ${like}` +
    (moods.length ? `, ${moods.join(" and ")}` : "");
}


async function toWav(blob: Blob) {
  const ctx = new AudioContext({ sampleRate: 48000 });
  const buf = await ctx.decodeAudioData(await blob.arrayBuffer());
  ctx.close?.();
  const n = buf.length, ch = buf.numberOfChannels, mono = new Float32Array(n);
  for (let c = 0; c < ch; c++) { const d = buf.getChannelData(c); for (let i = 0; i < n; i++) mono[i] += d[i] / ch; }
  let peak = 0;
  for (let i = 0; i < n; i++) peak = Math.max(peak, Math.abs(mono[i]));
  const gain = peak > 0 ? Math.min(0.89 / peak, 20) : 1;
  const out = new DataView(new ArrayBuffer(44 + n * 2));
  const str = (o: number, v: string) => { for (let i = 0; i < v.length; i++) out.setUint8(o + i, v.charCodeAt(i)); };
  str(0, "RIFF"); out.setUint32(4, 36 + n * 2, true); str(8, "WAVE"); str(12, "fmt "); out.setUint32(16, 16, true);
  out.setUint16(20, 1, true); out.setUint16(22, 1, true); out.setUint32(24, buf.sampleRate, true);
  out.setUint32(28, buf.sampleRate * 2, true); out.setUint16(32, 2, true); out.setUint16(34, 16, true);
  str(36, "data"); out.setUint32(40, n * 2, true);
  for (let i = 0; i < n; i++) { const v = Math.max(-1, Math.min(1, mono[i] * gain)); out.setInt16(44 + i * 2, v < 0 ? v * 32768 : v * 32767, true); }
  return { wav: out.buffer, mono, rate: buf.sampleRate };
}

export { measureMelody, describeIdea, toWav };
