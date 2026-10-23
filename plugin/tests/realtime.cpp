#include "Player.h"
#include <atomic>
#include <cstdlib>
#include <iostream>
#include <new>
using namespace loops;
thread_local bool auditing = false;
std::atomic<unsigned> allocations{0};
void *operator new(size_t n) {
  if (auditing)
    ++allocations;
  if (auto *p = std::malloc(n ? n : 1))
    return p;
  throw std::bad_alloc();
}
void operator delete(void *p) noexcept { std::free(p); }
void operator delete(void *p, size_t) noexcept { std::free(p); }
void *operator new[](size_t n) { return ::operator new(n); }
void operator delete[](void *p) noexcept { std::free(p); }
int main() {
  Player player;
  player.prepare(48000);
  auto audio = std::make_shared<Audio>();
  audio->samples.resize(24000);
  for (size_t i = 0; i < audio->samples.size(); ++i) {
    float x = float(std::sin(i * 0.05) * 0.1);
    audio->samples[i] = {x, x};
  }
  for (int slot = 0; slot < 33; ++slot) {
    auto clip = std::make_unique<Clip>();
    clip->audio = audio;
    clip->beats = 1;
    clip->mode = Mode::Toggle;
    clip->bus = slot % 17;
    player.publish(slot, std::move(clip));
  }
  std::array<std::array<double, 256>, 34> data{};
  std::array<double *, 34> ptr{};
  for (int ch = 0; ch < 34; ++ch)
    ptr[ch] = data[ch].data();
  auditing = true;
  for (int i = 0; i < 33; ++i)
    player.event({i * 3, i, -1, 1});
  for (int block = 0; block < 100; ++block) {
    if (block == 20)
      player.event({127, 0, Pitch, 7});
    if (block == 40) {
      player.event({42, 1, Start, 0.2});
      player.event({84, 1, End, 0.8});
    }
    player.process(ptr.data(), 34, 256,
                   {120 + block * .1, block * 256 * 120.0 / 2880000, true, 4});
  }
  auditing = false;
  if (allocations) {
    std::cerr << allocations << " allocations in real-time callbacks\n";
    return 1;
  }
  for (auto channel : data)
    for (auto value : channel)
      if (!std::isfinite(value))
        return 2;
  std::cout << "32 voices, voice stealing, automation, tempo changes: zero C++ "
               "allocations in audio processing\n";
}
