#include "Player.h"
#include <algorithm>
#include <cmath>
#include <stdexcept>
namespace loops {
namespace {
constexpr double Pi = 3.141592653589793;
double valid(double n, double fallback) {
  return std::isfinite(n) ? n : fallback;
}
} // namespace
Player::Player() {
  for (auto &position : playheads)
    position.store(-1);
  for (auto &p : pending)
    p.store(nullptr);
  for (auto &v : uiValues)
    v.store(std::numeric_limits<double>::quiet_NaN());
  for (auto &p : parameters)
    p = {0, 1, 0, 0, 20, 20000, 0, 1};
}
Player::~Player() {
  for (auto p : clips)
    delete p;
  for (auto &p : pending)
    delete p.exchange(nullptr);
  collect();
}
void Player::prepare(double sampleRate) {
  for (auto &position : playheads)
    position.store(-1);
  rate = std::clamp(valid(sampleRate, 48000), 8000.0, 192000.0);
  for (auto &v : voices) {
    v.slot = -1;
    v.stretch.presetCheaper(2, float(rate), true);
  }
  latency = voices[0].stretch.outputLatency();
  count = 0;
  internalPpq = 0;
  for (auto &value : queued)
    value.store(false);
}
void Player::publish(int slot, std::unique_ptr<Clip> clip) {
  if (slot < 0 || slot >= Slots)
    throw std::runtime_error("Invalid slot");
  delete pending[slot].exchange(clip.release(), std::memory_order_acq_rel);
}
void Player::collect() {
  Clip *clip;
  while (retired.pop(clip))
    delete clip;
}
void Player::uiParameter(int id, double value) {
  if (id >= 0 && id < NumParams && std::isfinite(value))
    uiValues[id].store(value, std::memory_order_release);
}
void Player::event(Event e) {
  if (count < events.size() && e.slot >= 0 && e.slot < Slots &&
      std::isfinite(e.value)) {
    e.offset = std::max(0, e.offset);
    events[count++] = e;
  }
}
void Player::stop(int slot) {
  for (auto &v : voices)
    if (v.slot == slot) {
      v.pending = false;
      v.releaseIn = v.envelope == 0 ? 0 : latency;
    }
}
void Player::note(int slot, double velocity, const Clock &clock, double ppq) {
  const auto *clip = clips[slot];
  if (!clip || !clip->audio || clip->audio->samples.empty())
    return;
  if (velocity <= 0) {
    if (clip->mode == Mode::Gate)
      stop(slot);
    return;
  }
  if (clip->mode == Mode::Toggle) {
    for (auto &v : voices)
      if (v.slot == slot && !v.releasing) {
        stop(slot);
        return;
      }
  }
  if (!clip->retrigger)
    for (const auto &v : voices)
      if (v.slot == slot && !v.releasing)
        return;
  if (clip->group > 0)
    for (auto &v : voices)
      if (v.pending && v.slot >= 0 && clips[v.slot] &&
          clips[v.slot]->group == clip->group)
        v.slot = -1;
  Voice *target = nullptr;
  for (auto &v : voices)
    if (v.slot == slot) {
      target = &v;
      break;
    }
  if (!target)
    for (auto &v : voices)
      if (v.slot < 0) {
        target = &v;
        break;
      }
  if (!target)
    target = &*std::min_element(
        voices.begin(), voices.end(),
        [](const Voice &a, const Voice &b) { return a.age < b.age; });
  for (auto &v : voices)
    if (v.slot >= 0 && clips[v.slot] && clip->choke > 0 &&
        clips[v.slot]->choke == clip->choke) {
      v.pending = false;
      v.releaseIn = v.envelope == 0 ? 0 : latency;
    }
  auto &v = *target;
  if (v.slot >= 0 && clips[v.slot]) {
    const int bus = std::clamp(clips[v.slot]->bus, 0, 16);
    for (int ch = 0; ch < 2; ++ch)
      tails[bus][ch] += v.last[ch];
  }
  v.stretch.reset();
  v.slot = slot;
  v.age = ++age;
  v.read = 0;
  v.played = 0;
  v.inputFraction = 0;
  v.velocity = float(std::clamp(velocity, 0.0, 1.0));
  v.envelope = 0;
  v.releasing = false;
  v.releaseIn = -1;
  v.boundaryFade = 0;
  v.low = {};
  v.highInput = {};
  v.highOutput = {};
  v.due = clock.running && clip->quantize > 0
              ? std::ceil((ppq - 1e-9) / clip->quantize) * clip->quantize
              : ppq;
  v.pending = v.due > ppq + 1e-9;
  const auto &p = parameters[slot];
  v.gain = p[Gain];
  v.pan = p[Pan];
  v.pitch = p[Pitch];
  v.hp = p[Highpass];
  v.lp = p[Lowpass];
  if (!v.pending)
    activate(v, clock, ppq);
}
void Player::activate(Voice &v, const Clock &clock, double) {
  const auto *clip = clips[v.slot];
  const auto &p = parameters[v.slot];
  if (clip->group > 0) {
    double phase = 0;
    for (auto &other : voices)
      if (&other != &v && other.slot >= 0 && !other.pending &&
          clips[other.slot] && clips[other.slot]->group == clip->group &&
          !other.releasing && other.releaseIn < 0) {
        phase = other.played;
        stop(other.slot);
      }
    v.played = phase;
    const double heard =
        std::max(0.0, phase - latency * clock.bpm / (60 * rate));
    v.read = std::fmod(heard, clip->beats) / clip->beats *
             clip->audio->samples.size() * std::max(0.00001, p[End] - p[Start]);
  }
  // Feed wrapped pre-roll so the stretcher's processing time starts at the
  // note.
  const int pre = v.stretch.inputLatency();
  std::array<std::array<float, 32768>, 2> input{};
  float *pointers[2] = {input[0].data(), input[1].data()};
  const auto &audio = *clip->audio;
  const double length = double(audio.samples.size());
  for (int i = 0; i < std::min(pre, 32768); ++i) {
    double pos = std::fmod(
        std::fmod(v.read + p[Start] * length + (i - pre) * audio.rate / rate,
                  length) +
            length,
        length);
    size_t n = size_t(pos);
    for (int ch = 0; ch < 2; ++ch)
      input[ch][i] = audio.samples[n][ch];
  }
  v.stretch.seek(pointers, std::min(pre, 32768), 1.0);
}
std::array<float, 2> Player::tick(Voice &v, const Clock &clock, double ppq) {
  if (v.slot < 0)
    return {};
  auto *clip = clips[v.slot];
  if (!clip || !clip->audio) {
    v.slot = -1;
    return {};
  }
  if (v.pending) {
    if (ppq + 1e-9 < v.due)
      return {};
    v.pending = false;
    activate(v, clock, ppq);
  }
  auto &audio = *clip->audio;
  auto &p = parameters[v.slot];
  const double size = double(audio.samples.size());
  const double start = std::clamp(p[Start], 0.0, 0.99999) * size;
  const double end =
      std::max(start + 1, std::clamp(p[End], 0.00001, 1.0) * size);
  const double length = std::min(end, size) - start;
  const double wanted = length / audio.rate / (clip->beats * 60.0 / clock.bpm);
  const double ratio = std::clamp(wanted, 0.000001, 16.0);
  const double step = wanted / ratio;
  v.inputFraction += ratio;
  int n = std::min(16, int(v.inputFraction));
  v.inputFraction -= n;
  std::array<std::array<float, 16>, 2> in{};
  float outL = 0, outR = 0;
  float *inputs[2] = {in[0].data(), in[1].data()};
  float *outputs[2] = {&outL, &outR};
  for (int i = 0; i < n; ++i) {
    const double pos = start + std::fmod(v.read, length);
    size_t a = size_t(pos), b = a + 1;
    if (b >= size_t(end))
      b = size_t(start);
    const float f = float(pos - a);
    for (int ch = 0; ch < 2; ++ch)
      in[ch][i] = audio.samples[a][ch] +
                  (audio.samples[b][ch] - audio.samples[a][ch]) * f;
    v.read += audio.rate / rate * step;
  }
  v.pitch += (std::clamp(p[Pitch], -24.0, 24.0) - v.pitch) * 0.002;
  v.stretch.setTransposeSemitones(float(v.pitch - 12 * std::log2(step)));
  v.stretch.process(inputs, n, outputs, 1);
  v.played += clock.bpm / (60 * rate);
  if (clip->mode == Mode::OneShot &&
      v.played >= clip->beats + latency * clock.bpm / (60 * rate))
    v.releasing = true;
  if (v.releaseIn >= 0 && v.releaseIn-- == 0)
    v.releasing = true;
  const float envelopeStep = float(1.0 / (rate * 0.005));
  v.envelope = std::clamp(
      v.envelope + (v.releasing ? -envelopeStep : envelopeStep), 0.0f, 1.0f);
  v.gain += (std::clamp(p[Gain], 0.0, 2.0) - v.gain) * 0.002;
  v.pan += (std::clamp(p[Pan], -1.0, 1.0) - v.pan) * 0.002;
  v.hp += (std::clamp(p[Highpass], 10.0, 20000.0) - v.hp) * 0.002;
  v.lp += (std::clamp(p[Lowpass], 20.0, 22000.0) - v.lp) * 0.002;
  const float lp =
      float(1 - std::exp(-2 * Pi * std::min(v.lp, rate * 0.45) / rate));
  const float hp =
      float(std::exp(-2 * Pi * std::min(v.hp, rate * 0.45) / rate));
  std::array<float, 2> output{outL, outR};
  for (int ch = 0; ch < 2; ++ch) {
    v.low[ch] += lp * (output[ch] - v.low[ch]);
    const float high = hp * (v.highOutput[ch] + v.low[ch] - v.highInput[ch]);
    v.highInput[ch] = v.low[ch];
    v.highOutput[ch] = high;
    const float pan =
        float(ch == 0 ? std::min(1.0, 1 - v.pan) : std::min(1.0, 1 + v.pan));
    output[ch] = high * float(v.gain) * pan * v.velocity * v.envelope;
    if (v.boundaryFade > 0) {
      const float mix = float(v.boundaryFade / (rate * 0.005));
      output[ch] = output[ch] * (1 - mix) + v.boundaryFrom[ch] * mix;
    }
    v.last[ch] = output[ch];
  }
  if (v.boundaryFade > 0)
    --v.boundaryFade;
  if (v.releasing && v.envelope <= 0)
    v.slot = -1;
  return output;
}
void Player::process(double **outputs, int channels, int frames, Clock clock) {
  clock.bpm = std::clamp(valid(clock.bpm, lastBpm), 30.0, 300.0);
  lastBpm = clock.bpm;
  clock.ppq = valid(clock.ppq, 0);
  if (clock.internal) {
    clock.ppq = internalPpq;
    clock.running = true;
    internalPpq += frames * clock.bpm / (60 * rate);
  }
  publishedPpq.store(clock.ppq);
  if (wasRunning &&
      (!clock.running || std::abs(clock.ppq - previousPpq) > 0.1)) {
    for (auto &v : voices) {
      v.pending = false;
      if (!clock.running)
        v.releasing = true;
      else if (v.slot >= 0) {
        const auto *c = clips[v.slot];
        if (c) {
          const auto &p = parameters[v.slot];
          const double length = c->audio
                                    ? double(c->audio->samples.size()) *
                                          std::max(0.00001, p[End] - p[Start])
                                    : 1;
          v.read =
              std::fmod(std::max(0.0, clock.ppq), c->beats) / c->beats * length;
          v.played = c->group > 0
                         ? std::max(0.0, clock.ppq)
                         : std::fmod(std::max(0.0, clock.ppq), c->beats);
          v.stretch.reset();
        }
      }
    }
  }
  wasRunning = clock.running;
  previousPpq = clock.ppq + frames * clock.bpm / (60 * rate);
  for (int s = 0; s < Slots; ++s)
    if (!retired.full()) {
      if (auto *replacement =
              pending[s].exchange(nullptr, std::memory_order_acq_rel)) {
        auto *old = clips[s];
        clips[s] = replacement;
        if (old)
          retired.push(old);
      }
    }
  for (int id = 0; id < NumParams; ++id) {
    double value = uiValues[id].exchange(
        std::numeric_limits<double>::quiet_NaN(), std::memory_order_acq_rel);
    if (std::isfinite(value))
      event({0, id / ParamsPerSlot, id % ParamsPerSlot, value});
  }
  // Stable insertion sort uses fixed storage and preserves equal-offset event
  // order.
  for (size_t i = 1; i < count; ++i) {
    Event e = events[i];
    size_t j = i;
    while (j > 0 && events[j - 1].offset > e.offset) {
      events[j] = events[j - 1];
      --j;
    }
    events[j] = e;
  }
  size_t next = 0;
  for (int i = 0; i < frames; ++i) {
    double ppq = clock.ppq + i * clock.bpm / (60 * rate);
    while (next < count && events[next].offset <= i) {
      auto e = events[next++];
      if (e.param < 0)
        note(e.slot, e.value, clock, ppq);
      else {
        if (e.param >= ParamsPerSlot)
          continue;
        if (e.param == Start || e.param == End)
          for (auto &voice : voices)
            if (voice.slot == e.slot) {
              voice.boundaryFrom = voice.last;
              voice.boundaryFade = int(rate * 0.005);
            }
        parameters[e.slot][e.param] = e.value;
        if (e.param == Active) {
          if (e.value > 0.5) {
            bool active = false;
            for (const auto &v : voices)
              if (v.slot == e.slot && !v.releasing)
                active = true;
            if (!active)
              note(e.slot, 1, clock, ppq);
          } else
            stop(e.slot);
        }
      }
    }
    for (int ch = 0; ch < channels; ++ch)
      outputs[ch][i] = 0;
    for (int bus = 0; bus < Buses; ++bus)
      for (int ch = 0; ch < 2; ++ch) {
        if (bus * 2 + ch < channels)
          outputs[bus * 2 + ch][i] += tails[bus][ch];
        tails[bus][ch] *= 0.995f;
        if (std::abs(tails[bus][ch]) < 1e-12f)
          tails[bus][ch] = 0;
      }
    for (auto &v : voices) {
      int slot = v.slot;
      if (slot < 0)
        continue;
      const int bus = std::clamp(clips[slot] ? clips[slot]->bus : 0, 0, 16);
      auto sample = tick(v, clock, ppq);
      for (int ch = 0; ch < 2; ++ch) {
        const int output = bus * 2 + ch;
        if (output < channels)
          outputs[output][i] += sample[ch];
      }
    }
  }
  std::array<double, Slots> positions;
  positions.fill(-1);
  for (const auto &voice : voices)
    if (voice.slot >= 0 && !voice.pending && clips[voice.slot]) {
      const auto *clip = clips[voice.slot];
      const double heard =
          std::max(0.0, voice.played - latency * clock.bpm / (60 * rate));
      positions[voice.slot] = std::fmod(heard, clip->beats) / clip->beats;
    }
  for (int slot = 0; slot < Slots; ++slot) {
    playheads[slot].store(positions[slot], std::memory_order_relaxed);
    bool waiting = false;
    for (const auto &voice : voices)
      if (voice.slot == slot && voice.pending)
        waiting = true;
    queued[slot].store(waiting, std::memory_order_relaxed);
  }
  // Carry events only when a host splits a block after supplying its events.
  size_t remain = 0;
  for (; next < count; ++next) {
    auto e = events[next];
    e.offset -= frames;
    events[remain++] = e;
  }
  count = remain;
}
Audio renderCycle(const Clip &clip,
                  const std::array<double, ParamsPerSlot> &params, double bpm,
                  double rate) {
  Player player;
  player.prepare(rate);
  auto copy = std::make_unique<Clip>(clip);
  copy->bus = 0;
  copy->mode = Mode::Toggle;
  copy->quantize = 0;
  copy->group = 0;
  player.publish(0, std::move(copy));
  for (int p = 1; p < ParamsPerSlot; ++p)
    player.uiParameter(p, params[p]);
  const size_t frames = size_t(std::llround(clip.beats * 60.0 / bpm * rate));
  if (frames < 1 || frames > size_t(rate * 600))
    throw std::runtime_error("Invalid export length");
  const size_t warm = frames + size_t(player.latencySamples());
  std::vector<double> left(frames + warm), right(frames + warm);
  double *outputs[2] = {left.data(), right.data()};
  player.event({0, 0, -1, 1});
  player.process(outputs, 2, int(frames + warm), {bpm, 0, false, 4});
  Audio audio;
  audio.rate = rate;
  audio.samples.resize(frames);
  for (size_t n = 0; n < frames; ++n)
    audio.samples[n] = {float(left[n + warm]), float(right[n + warm])};
  return audio;
}
} // namespace loops
