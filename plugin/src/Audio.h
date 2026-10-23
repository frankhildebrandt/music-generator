#pragma once
#include <array>
#include <cstdint>
#include <filesystem>
#include <vector>
namespace loops {
struct Audio {
  double rate = 48000;
  std::vector<std::array<float, 2>> samples;
};
Audio readWav(const std::filesystem::path &path);
void writeWav(const std::filesystem::path &path, const Audio &audio);
} // namespace loops
