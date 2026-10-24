#pragma once
#include "Player.h"
#include "json.hpp"
#include <string>
namespace loops {
using Json = nlohmann::json;
struct Slot {
  std::shared_ptr<const Audio> original, edited;
  Json metadata = Json::object(), recipe = Json::object();
  std::array<double, ParamsPerSlot> params{0, 1, 0, 0, 20, 20000, 0, 1};
  Mode mode = Mode::Gate;
  double quantize = 0;
  int choke = 0, bus = 0, group = 0;
  bool retrigger = true;
  std::array<int, 4> macros{Gain, Lowpass, Pitch, Pan};
  std::string name = "Empty";
  double bpm = 120, beats = 16;
  std::string key = "A minor";
  Clip clip() const {
    return {edited, bpm, beats, mode, quantize, choke, bus, group, retrigger};
  }
};
struct Bank : std::array<Slot, Slots> {
  Json document = {{"name", "Untitled bank"},
                   {"idea_name", "Sound idea"},
                   {"idea", ""},
                   {"revision", 1},
                   {"quantize", -1.0},
                   {"groups", Json::array()},
                   {"templates", Json::array()}};
};
std::vector<uint8_t> saveBank(const Bank &bank);
Bank loadBank(const uint8_t *data, size_t size);
} // namespace loops
