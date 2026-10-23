#include "Audio.h"
#include <cmath>
#include <cstring>
#include <fstream>
#include <stdexcept>
namespace loops {
Audio readWav(const std::filesystem::path &path) {
  std::ifstream file(path, std::ios::binary);
  if (!file)
    throw std::runtime_error("Cannot open WAV");
  file.seekg(0, std::ios::end);
  const auto size = file.tellg();
  if (size < 44 || size > 256 * 1024 * 1024)
    throw std::runtime_error("Invalid WAV size");
  file.seekg(0);
  std::vector<uint8_t> b(static_cast<size_t>(size));
  file.read(reinterpret_cast<char *>(b.data()), size);
  auto u16 = [&](size_t p) {
    return unsigned(b[p]) | (unsigned(b[p + 1]) << 8);
  };
  auto u32 = [&](size_t p) {
    return uint32_t(u16(p)) | (uint32_t(u16(p + 2)) << 16);
  };
  if (memcmp(b.data(), "RIFF", 4) || memcmp(b.data() + 8, "WAVE", 4))
    throw std::runtime_error("Invalid WAV header");
  unsigned format = 0, channels = 0, bits = 0, rate = 0;
  size_t data = 0, bytes = 0;
  for (size_t p = 12; p + 8 <= b.size();) {
    const size_t n = u32(p + 4), start = p + 8;
    if (n > b.size() - start)
      throw std::runtime_error("Truncated WAV");
    if (!memcmp(b.data() + p, "fmt ", 4) && n >= 16) {
      format = u16(start);
      channels = u16(start + 2);
      rate = u32(start + 4);
      bits = u16(start + 14);
    }
    if (!memcmp(b.data() + p, "data", 4)) {
      data = start;
      bytes = n;
    }
    p = start + n + (n % 2);
  }
  if (format != 3 || channels != 2 || bits != 32 || rate < 8000 ||
      rate > 192000 || !data || bytes % 8 || !bytes)
    throw std::runtime_error("Expected canonical stereo float32 WAV");
  Audio a;
  a.rate = rate;
  a.samples.resize(bytes / 8);
  memcpy(a.samples.data(), b.data() + data, bytes);
  for (const auto &s : a.samples)
    for (float v : s)
      if (!std::isfinite(v) || std::abs(v) > 16)
        throw std::runtime_error("Invalid audio sample");
  return a;
}
void writeWav(const std::filesystem::path &path, const Audio &a) {
  if (a.samples.empty() || a.samples.size() > 192000 * 600)
    throw std::runtime_error("Invalid export length");
  std::ofstream f(path, std::ios::binary);
  if (!f)
    throw std::runtime_error("Cannot create WAV");
  auto u16 = [&](uint16_t v) {
    f.put(v & 255);
    f.put(v >> 8);
  };
  auto u32 = [&](uint32_t v) {
    u16(v & 65535);
    u16(v >> 16);
  };
  const auto n = uint32_t(a.samples.size() * 8);
  f.write("RIFF", 4);
  u32(36 + n);
  f.write("WAVEfmt ", 8);
  u32(16);
  u16(3);
  u16(2);
  u32(uint32_t(a.rate));
  u32(uint32_t(a.rate) * 8);
  u16(8);
  u16(32);
  f.write("data", 4);
  u32(n);
  f.write(reinterpret_cast<const char *>(a.samples.data()), n);
  if (!f)
    throw std::runtime_error("WAV export failed");
}
} // namespace loops
