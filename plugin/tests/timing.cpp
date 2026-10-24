#include "Player.h"
#include <cmath>
#include <iostream>
#include <stdexcept>
namespace loops {
struct PlayerProbe {
  static bool active(const Player &player, int slot) {
    for (const auto &voice : player.voices)
      if (voice.slot == slot && !voice.releasing)
        return true;
    return false;
  }
  static double played(const Player &player, int slot) {
    for (const auto &voice : player.voices)
      if (voice.slot == slot)
        return voice.played;
    return -1;
  }
  static double sourcePosition(const Player &player, int slot) {
    for (const auto &voice : player.voices)
      if (voice.slot == slot)
        return voice.read;
    return -1;
  }
};
} // namespace loops
using namespace loops;
void require(bool value, const char *message) {
  if (!value)
    throw std::runtime_error(message);
}
int main() {
  try {
    auto playerStorage = std::make_unique<Player>();
    auto &player = *playerStorage;
    player.prepare(48000);
    auto audio = std::make_shared<Audio>();
    audio->samples.resize(2400, {0.2f, 0.2f});
    for (int slot = 0; slot < 3; ++slot) {
      auto clip = std::make_unique<Clip>();
      clip->audio = audio;
      clip->beats = 0.1;
      clip->mode = Mode(slot);
      player.publish(slot, std::move(clip));
    }
    std::array<std::array<double, 256>, 2> buffers{};
    double *outputs[] = {buffers[0].data(), buffers[1].data()};
    auto process = [&](int frames) {
      while (frames > 0) {
        const int n = std::min(frames, 256);
        player.process(outputs, 2, n, {120, 0, false, 4});
        frames -= n;
      }
    };
    player.event({37, 0, -1, 1});
    process(256);
    require(std::abs(PlayerProbe::sourcePosition(player, 0) - 219) < 1e-8,
            "MIDI offset ignored");
    process(2400 * 100 - 219);
    require(std::abs(PlayerProbe::sourcePosition(player, 0) - 240000) < 1e-6,
            "One hundred cycles accumulated source drift");
    const auto position = player.position(0);
    require(position >= 0 && position < 1, "Active playhead outside cycle");
    process(600);
    const auto advanced = player.position(0);
    require(std::abs(std::fmod(advanced - position + 1, 1) - 0.25) < 0.001,
            "Playhead does not follow loop phase");
    player.event({0, 0, -1, 0});
    process(player.latencySamples() + 241);
    require(!PlayerProbe::active(player, 0), "Gate ignored note-off");
    require(player.position(0) == -1, "Stopped playhead remains visible");
    player.event({0, 1, -1, 1});
    process(256);
    player.event({0, 1, -1, 0});
    process(256);
    require(PlayerProbe::active(player, 1), "Toggle stopped on note-off");
    player.event({0, 1, -1, 1});
    process(player.latencySamples() + 241);
    require(!PlayerProbe::active(player, 1), "Toggle failed to stop");
    player.event({0, 2, -1, 1});
    process(2400 + player.latencySamples() + 241);
    require(!PlayerProbe::active(player, 2), "One-shot exceeded one cycle");
    auto quantized = std::make_unique<Clip>();
    quantized->audio = audio;
    quantized->beats = 0.1;
    quantized->quantize = 1;
    player.publish(3, std::move(quantized));
    player.event({0, 3, -1, 1});
    player.process(outputs, 2, 256, {120, 0.5, true, 4});
    require(PlayerProbe::sourcePosition(player, 3) == 0,
            "Quantization started early");
    player.process(outputs, 2, 256, {150, 1, true, 4});
    require(PlayerProbe::sourcePosition(player, 3) > 0,
            "Transport jump did not resume");
    auto groupedStorage = std::make_unique<Player>();
    auto &grouped = *groupedStorage;
    grouped.prepare(48000);
    auto groupAudio = std::make_shared<Audio>();
    groupAudio->samples.resize(48000, {.2f, .1f});
    for (int n = 0; n < 3; ++n) {
      auto c = std::make_unique<Clip>();
      c->audio = groupAudio;
      c->beats = n == 1 ? .5 : 2;
      c->group = n == 2 ? 2 : 1;
      c->mode = Mode::Toggle;
      c->quantize = 1;
      grouped.publish(n, std::move(c));
    }
    auto advance = [&](int frames) {
      while (frames > 0) {
        const int n = std::min(256, frames);
        grouped.process(outputs, 2, n, {120, 0, false, 4, true});
        frames -= n;
      }
    };
    grouped.event({0, 0, -1, 1});
    advance(12000);
    grouped.event({0, 1, -1, 1});
    advance(256);
    require(grouped.isQueued(1), "Group target not shown as queued");
    require(PlayerProbe::active(grouped, 0),
            "Group source stopped before boundary");
    advance(11744);
    const double before = PlayerProbe::played(grouped, 0);
    advance(1);
    require(!grouped.isQueued(1),
            "Group target did not start at exact boundary");
    require(
        std::abs(PlayerProbe::played(grouped, 1) - before) < .0001,
        "Group inherited request-time instead of switch-time musical position");
    const double latencyBeats = grouped.latencySamples() * 120.0 / (60 * 48000);
    const double expectedRead =
        std::fmod(std::max(0.0, before - latencyBeats), .5) / .5 * 48000;
    require(std::abs(PlayerProbe::sourcePosition(grouped, 1) - expectedRead) <
                10,
            "Short group target did not wrap its musical phase");
    advance(grouped.latencySamples() + 300);
    require(!PlayerProbe::active(grouped, 0),
            "Exclusive group source kept playing");
    grouped.event({0, 2, -1, 1});
    advance(24000);
    require(PlayerProbe::active(grouped, 1) && PlayerProbe::active(grouped, 2),
            "Different sound groups cannot play together");
    auto immediateStorage = std::make_unique<Player>();
    auto &immediate = *immediateStorage;
    immediate.prepare(48000);
    for (int n = 0; n < 3; ++n) {
      auto c = std::make_unique<Clip>();
      c->audio = groupAudio;
      c->beats = n == 0 ? 2 : .5;
      c->group = 1;
      c->mode = Mode::Toggle;
      immediate.publish(n, std::move(c));
    }
    immediate.event({0, 0, -1, 1});
    for (int n = 0; n < 20; ++n)
      immediate.process(outputs, 2, 256, {120, 0, false, 4});
    const double immediatePhase = PlayerProbe::played(immediate, 0);
    immediate.event({37, 1, -1, 1});
    immediate.process(outputs, 2, 256, {120, 0, false, 4});
    require(std::abs(PlayerProbe::played(immediate, 1) - immediatePhase -
                     256 * 120.0 / (60 * 48000)) < .0001,
            "Immediate switch lost current group position / sample offset");
    auto jumpedStorage = std::make_unique<Player>();
    auto &jumped = *jumpedStorage;
    jumped.prepare(48000);
    for (int n = 0; n < 2; ++n) {
      auto c = std::make_unique<Clip>();
      c->audio = groupAudio;
      c->beats = n ? 32 : 16;
      c->group = 1;
      c->mode = Mode::Toggle;
      jumped.publish(n, std::move(c));
    }
    jumped.event({0, 0, -1, 1});
    jumped.process(outputs, 2, 256, {120, 0, true, 4});
    jumped.process(outputs, 2, 256, {120, 20, true, 4});
    jumped.event({0, 1, -1, 1});
    jumped.process(outputs, 2, 256, {120, 20 + 256.0 / 24000, true, 4});
    require(
        PlayerProbe::played(jumped, 1) > 20,
        "Host jump lost unwrapped group position when moving to a longer loop");
    std::cout << "MIDI modes, offsets, quantization, transport jumps and 100 "
                 "exact cycles passed\n";
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
