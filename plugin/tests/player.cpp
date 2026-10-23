#include "Bank.h"
#include <cmath>
#include <iostream>
#include <stdexcept>
using namespace loops;
void require(bool value, const char *error) {
  if (!value)
    throw std::runtime_error(error);
}
std::shared_ptr<Audio> tone() {
  auto a = std::make_shared<Audio>();
  a->samples.resize(24000);
  for (size_t i = 0; i < a->samples.size(); ++i) {
    float v = float(std::sin(i * 2 * 3.141592653589793 * 440 / 48000) * 0.3);
    a->samples[i] = {v, v};
  }
  return a;
}
int main() {
  try {
    Bank bank;
    bank[36].original = tone();
    bank[36].edited = bank[36].original;
    bank[36].beats = 1;
    bank[36].name = "Bass";
    auto bytes = saveBank(bank);
    auto restored = loadBank(bytes.data(), bytes.size());
    require(restored[36].edited->samples == bank[36].edited->samples,
            "Embedded audio changed");
    require(restored[36].name == "Bass", "Metadata lost");
    bool rejected = false;
    try {
      loadBank(bytes.data(), bytes.size() - 1);
    } catch (...) {
      rejected = true;
    }
    require(rejected, "Truncated bank accepted");
    auto clip = bank[36].clip();
    auto a = renderCycle(clip, bank[36].params, 120, 48000);
    require(a.samples.size() == 24000, "Incorrect cycle length");
    double energy = 0;
    for (auto s : a.samples)
      energy += s[0] * s[0];
    require(energy > 100, "Player is silent");
    auto faster = renderCycle(clip, bank[36].params, 150, 44100);
    require(faster.samples.size() == 17640,
            "Tempo conversion length incorrect");
    for (double tempo : {120.0, 137.0}) {
      auto continuous = std::make_shared<Audio>();
      continuous->rate = 8000;
      const auto frames = size_t(std::llround(32 * 60.0 / tempo * 8000));
      continuous->samples.resize(frames);
      for (size_t n = 0; n < frames; ++n) {
        const float value =
            float(0.3 * std::sin(2 * 3.141592653589793 * 200 * n / 8000));
        continuous->samples[n] = {value, value};
      }
      auto eightBars = clip;
      eightBars.audio = continuous;
      eightBars.beats = 32;
      auto cycle = renderCycle(eightBars, bank[36].params, tempo, 8000);
      require(cycle.samples.size() == frames,
              "Eight-bar export length incorrect");
      for (size_t start = 0; start + 160 <= frames; start += 160) {
        double energy = 0;
        for (size_t n = start; n < start + 160; ++n)
          energy += cycle.samples[n][0] * cycle.samples[n][0];
        require(energy / 160 > 0.001, "Silence inserted in eight-bar cycle");
      }
    }
    writeWav("test-loop.wav", a);
    auto wav = readWav("test-loop.wav");
    require(wav.samples == a.samples, "WAV changed samples");
    std::filesystem::remove("test-loop.wav");
    std::cout << "Bank portability, validation, player, tempo and WAV "
                 "contracts passed\n";
  } catch (const std::exception &e) {
    std::cerr << e.what() << '\n';
    return 1;
  }
}
