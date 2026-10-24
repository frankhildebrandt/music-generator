#pragma once
#include "Audio.h"
#include "signalsmith-stretch.h"
#include <atomic>
#include <limits>
#include <memory>
namespace loops {
constexpr int Slots = 128, Voices = 32, Buses = 17, ParamsPerSlot = 8,
              NumParams = Slots * ParamsPerSlot;
enum Param { Active, Gain, Pan, Pitch, Highpass, Lowpass, Start, End };
enum class Mode { Gate, Toggle, OneShot };
struct Clip {
  std::shared_ptr<const Audio> audio;
  double bpm = 120, beats = 16;
  Mode mode = Mode::Gate;
  double quantize = 0;
  int choke = 0, bus = 0, group = 0;
  bool retrigger = true;
};
struct Clock {
  double bpm = 120, ppq = 0;
  bool running = false;
  double barBeats = 4;
  bool internal = false;
};
struct Event {
  int offset = 0, slot = 0, param = -1;
  double value = 0;
};
template <class T, size_t N> class Queue {
  std::array<T, N> data{};
  std::atomic<size_t> read{0}, write{0};

public:
  bool push(T value) {
    auto w = write.load(std::memory_order_relaxed);
    auto next = (w + 1) % N;
    if (next == read.load(std::memory_order_acquire))
      return false;
    data[w] = value;
    write.store(next, std::memory_order_release);
    return true;
  }
  bool pop(T &value) {
    auto r = read.load(std::memory_order_relaxed);
    if (r == write.load(std::memory_order_acquire))
      return false;
    value = data[r];
    read.store((r + 1) % N, std::memory_order_release);
    return true;
  }
  bool full() const {
    return (write.load(std::memory_order_relaxed) + 1) % N ==
           read.load(std::memory_order_acquire);
  }
};
class Player {
  friend struct PlayerProbe;
  struct Voice {
    signalsmith::stretch::SignalsmithStretch<float> stretch;
    int slot = -1;
    int releaseIn = -1, boundaryFade = 0;
    std::array<float, 2> boundaryFrom{};
    uint64_t age = 0;
    double read = 0, inputFraction = 0, played = 0, due = 0;
    float velocity = 1, envelope = 0;
    bool pending = false, releasing = false;
    std::array<float, 2> low{}, highInput{}, highOutput{}, last{};
    double gain = 1, pan = 0, pitch = 0, hp = 20, lp = 20000;
  };
  std::array<std::array<float, 2>, Buses> tails{};
  double rate = 48000, lastBpm = 120, previousPpq = 0;
  bool wasRunning = false;
  uint64_t age = 0;
  std::array<Voice, Voices> voices;
  std::array<std::array<float, 32768>, 2> preroll{};
  std::array<Clip *, Slots> clips{};
  std::array<std::atomic<double>, Slots> playheads;
  std::array<std::atomic<bool>, Slots> queued{};
  double internalPpq = 0;
  std::atomic<double> publishedPpq{0};
  std::array<std::atomic<Clip *>, Slots> pending{};
  Queue<Clip *, 512> retired;
  std::array<std::array<double, ParamsPerSlot>, Slots> parameters{};
  std::array<std::atomic<double>, NumParams> uiValues;
  std::array<Event, 8192> events{};
  size_t count = 0;
  int latency = 0;
  void note(int slot, double velocity, const Clock &clock, double ppq);
  void stop(int slot);
  void activate(Voice &, const Clock &, double ppq);
  std::array<float, 2> tick(Voice &voice, const Clock &clock, double ppq);

public:
  Player();
  ~Player();
  Player(const Player &) = delete;
  Player &operator=(const Player &) = delete;
  void prepare(double sampleRate);
  int latencySamples() const { return latency; }
  double position(int slot) const {
    return slot >= 0 && slot < Slots
               ? playheads[slot].load(std::memory_order_relaxed)
               : -1;
  }
  bool isQueued(int slot) const {
    return slot >= 0 && slot < Slots && queued[slot].load();
  }
  double transportPosition() const { return publishedPpq.load(); }
  void publish(int slot, std::unique_ptr<Clip> clip);
  void collect();
  void uiParameter(int id, double value);
  void event(Event event);
  void process(double **outputs, int channels, int frames, Clock clock);
};
Audio renderCycle(const Clip &clip,
                  const std::array<double, ParamsPerSlot> &params, double bpm,
                  double rate);
} // namespace loops
