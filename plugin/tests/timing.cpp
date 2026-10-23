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
    Player player;
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
    player.event({0, 0, -1, 0});
    process(player.latencySamples() + 241);
    require(!PlayerProbe::active(player, 0), "Gate ignored note-off");
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
    std::cout << "MIDI modes, offsets, quantization, transport jumps and 100 "
                 "exact cycles passed\n";
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
