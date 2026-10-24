#include "Bank.h"
#include <cstring>
#include <stdexcept>
namespace loops {
namespace {
constexpr uint64_t Budget = 512ULL * 1024 * 1024;
void put32(std::vector<uint8_t> &b, uint32_t n) {
  for (int i = 0; i < 4; ++i)
    b.push_back(uint8_t(n >> (8 * i)));
}
struct Reader {
  const uint8_t *b;
  size_t size, pos = 0;
  uint32_t u32() {
    if (size - pos < 4)
      throw std::runtime_error("Truncated bank");
    uint32_t n = 0;
    for (int i = 0; i < 4; ++i)
      n |= uint32_t(b[pos++]) << (8 * i);
    return n;
  }
};
} // namespace
std::vector<uint8_t> saveBank(const Bank &bank) {
  Json metadata = Json::array();
  for (const auto &s : bank)
    metadata.push_back({{"name", s.name},
                        {"bpm", s.bpm},
                        {"beats", s.beats},
                        {"key", s.key},
                        {"mode", int(s.mode)},
                        {"quantize", s.quantize},
                        {"choke", s.choke},
                        {"bus", s.bus},
                        {"group", s.group},
                        {"retrigger", s.retrigger},
                        {"macros", s.macros},
                        {"params", s.params},
                        {"recipe", s.recipe},
                        {"metadata", s.metadata}});
  const auto text =
      Json{{"slots", metadata}, {"document", bank.document}}.dump();
  std::vector<uint8_t> b{'M', 'G', 'L', 'B'};
  put32(b, 2);
  put32(b, uint32_t(text.size()));
  b.insert(b.end(), text.begin(), text.end());
  for (const auto &s : bank)
    for (const auto &audio : {s.original, s.edited}) {
      const auto n = audio ? audio->samples.size() : 0;
      put32(b, uint32_t(n));
      put32(b, audio ? uint32_t(audio->rate) : 48000);
      if (n) {
        const auto *p =
            reinterpret_cast<const uint8_t *>(audio->samples.data());
        b.insert(b.end(), p, p + n * 8);
      }
      if (b.size() > Budget)
        throw std::runtime_error(
            "Loop bank exceeds 512 MiB; shorten or remove loops");
    }
  return b;
}
Bank loadBank(const uint8_t *b, size_t size) {
  if (size < 12 || size > Budget || std::memcmp(b, "MGLB", 4))
    throw std::runtime_error("Invalid loop bank");
  Reader r{b, size, 4};
  const auto version = r.u32();
  if (version != 1 && version != 2)
    throw std::runtime_error("Unsupported bank version");
  auto length = r.u32();
  if (length > 16 * 1024 * 1024 || length > size - r.pos)
    throw std::runtime_error("Invalid bank metadata");
  auto json = Json::parse(b + r.pos, b + r.pos + length);
  r.pos += length;
  Json document = version == 2 ? json.at("document") : Json::object();
  if (version == 2)
    json = json.at("slots");
  if (!json.is_array() || json.size() != Slots)
    throw std::runtime_error("Invalid slot count");
  Bank bank;
  if (!document.is_object())
    throw std::runtime_error("Invalid bank document");
  bank.document.update(document);
  const auto &doc = bank.document;
  const double grid = doc.at("quantize").get<double>();
  if (!std::isfinite(grid) || grid < -4 || grid > 64 ||
      !doc.at("groups").is_array() || doc.at("groups").size() > 16 ||
      !doc.at("templates").is_array() || doc.at("templates").size() > 256 ||
      doc.at("idea").get<std::string>().size() > 8000 ||
      doc.at("name").get<std::string>().size() > 1024 ||
      doc.at("idea_name").get<std::string>().size() > 1024 ||
      !doc.at("revision").is_number_integer())
    throw std::runtime_error("Invalid bank workspace");
  std::array<bool, 17> groups{};
  for (const auto &group : doc.at("groups")) {
    const int id = group.at("id"), color = group.at("color");
    if (id < 1 || id > 16 || groups[id] || color < 0 || color > 7 ||
        group.at("name").get<std::string>().size() > 1024)
      throw std::runtime_error("Invalid group metadata");
    groups[id] = true;
  }
  if (doc.contains("artwork") &&
      (!doc.at("artwork").is_string() ||
       doc.at("artwork").get_ref<const std::string &>().size() >
           8 * 1024 * 1024))
    throw std::runtime_error("Invalid bank artwork");
  for (int i = 0; i < Slots; ++i) {
    auto &s = bank[i];
    auto j = json[i];
    s.name = j.at("name").get<std::string>();
    s.bpm = j.at("bpm");
    s.beats = j.at("beats");
    s.key = j.at("key");
    int mode = j.at("mode");
    s.quantize = j.at("quantize");
    s.choke = j.at("choke");
    s.bus = j.at("bus");
    s.group = j.value("group", 0);
    s.retrigger = j.value("retrigger", true);
    s.macros = j.value("macros", std::array<int, 4>{Gain, Lowpass, Pitch, Pan});
    if (s.group < 0 || s.group > 16 ||
        (version == 2 && s.group > 0 && !groups[s.group]))
      throw std::runtime_error("Invalid sound group");
    for (auto parameter : s.macros)
      if (parameter < Gain || parameter > End)
        throw std::runtime_error("Invalid macro mapping");
    s.params = j.at("params").get<std::array<double, ParamsPerSlot>>();
    s.recipe = j.at("recipe");
    s.metadata = j.at("metadata");
    if (mode < 0 || mode > 2 || s.bus < 0 || s.bus > 16 || s.choke < 0 ||
        s.choke > 16 || !std::isfinite(s.bpm) || s.bpm < 30 || s.bpm > 300 ||
        !std::isfinite(s.beats) || s.beats < 0.01 ||
        s.beats * 60 / s.bpm > 600 || !std::isfinite(s.quantize) ||
        s.quantize < 0 || s.quantize > 12 || s.name.size() > 1024 ||
        s.key.size() > 80)
      throw std::runtime_error("Invalid slot metadata");
    s.mode = Mode(mode);
    for (auto v : s.params)
      if (!std::isfinite(v))
        throw std::runtime_error("Invalid parameter value");
    for (auto *target : {&s.original, &s.edited}) {
      uint64_t n = r.u32();
      double rate = r.u32();
      if (n > 48000 * 600 || rate < 8000 || rate > 192000 ||
          n * 8 > size - r.pos)
        throw std::runtime_error("Invalid embedded audio length");
      if (n) {
        auto a = std::make_shared<Audio>();
        a->rate = rate;
        a->samples.resize(n);
        memcpy(a->samples.data(), b + r.pos, n * 8);
        for (auto pair : a->samples)
          for (auto v : pair)
            if (!std::isfinite(v) || std::abs(v) > 16)
              throw std::runtime_error("Invalid embedded sample");
        *target = a;
      }
      r.pos += n * 8;
    }
  }
  if (r.pos != size)
    throw std::runtime_error("Unexpected bank data");
  return bank;
}
} // namespace loops
