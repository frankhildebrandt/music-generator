#include "MusicGeneratorLoops.h"
#include <fstream>
using namespace loops;
void MusicGeneratorLoops::stopAll() {
  stopPreview();
  const auto previous = selected;
  for (int slot = 0; slot < Slots; ++slot) {
    selected = slot;
    parameter(Active, 0);
  }
  selected = previous;
}
void MusicGeneratorLoops::republish() {
  for (int slot = 0; slot < Slots; ++slot)
    publish(slot);
  changed();
}
void MusicGeneratorLoops::loadTemplates() {
  templates = factoryTemplates();
  try {
    std::ifstream input(profileRoot() / "prompt-templates.json");
    Json custom;
    if (input)
      input >> custom;
    if (custom.is_array())
      for (const auto &item : custom)
        if (item.is_object() && item.contains("name") &&
            item.contains("prompt"))
          templates.push_back(item);
  } catch (...) {
    status = "Could not read custom prompt templates";
  }
  for (const auto &item : bank.document["templates"]) {
    bool exists = false;
    for (const auto &known : templates)
      if (known == item)
        exists = true;
    if (!exists && item.is_object() && item.contains("name") &&
        item.contains("prompt"))
      templates.push_back(item);
  }
}
void MusicGeneratorLoops::saveTemplate(bool idea) {
  Json value{{"kind", idea ? "idea" : "instrument"},
             {"custom", true},
             {"name", idea ? bank.document.value("idea_name", "My sound idea")
                           : instrument + " prompt"},
             {"instrument", instrument},
             {"prompt", idea ? bank.document.value("idea", "") : prompt}};
  templates.push_back(value);
  bank.document["templates"].push_back(value);
  try {
    Json custom = Json::array();
    for (const auto &item : templates)
      if (item.value("custom", false))
        custom.push_back(item);
    std::filesystem::create_directories(profileRoot());
    const auto temporary = profileRoot() / "prompt-templates.tmp";
    {
      std::ofstream out(temporary);
      out << custom.dump(2);
      if (!out)
        throw std::runtime_error("Template save failed");
    }
    const auto target = profileRoot() / "prompt-templates.json";
#ifdef _WIN32
    std::filesystem::remove(target);
#endif
    std::filesystem::rename(temporary, target);
    status = "Template saved to your library and this bank";
    changed();
  } catch (const std::exception &e) {
    status = e.what();
  }
}
void MusicGeneratorLoops::setIdea(const std::string &value) {
  if (bank.document.value("idea", "") == value)
    return;
  bank.document["idea"] = value.substr(0, 8000);
  bank.document["revision"] = bank.document.value("revision", 1) + 1;
  changed();
}
void MusicGeneratorLoops::applyTemplate(const Json &value) {
  if (value.value("kind", "idea") == "idea") {
    bank.document["idea_name"] = value.value("name", "Sound idea");
    setIdea(value.value("prompt", ""));
  } else {
    prompt = value.value("prompt", "");
    instrument = value.value("instrument", instrument);
    changed();
  }
}
void MusicGeneratorLoops::importArtwork(const std::string &path) {
  try {
    std::ifstream input(std::filesystem::u8path(path),
                        std::ios::binary | std::ios::ate);
    const auto size = input.tellg();
    if (size < 8 || size > 4 * 1024 * 1024)
      throw std::runtime_error("Choose PNG/JPEG artwork up to 4 MiB");
    std::vector<uint8_t> bytes(static_cast<size_t>(size));
    input.seekg(0);
    input.read(reinterpret_cast<char *>(bytes.data()), size);
    const bool png =
        bytes[0] == 137 && bytes[1] == 80 && bytes[2] == 78 && bytes[3] == 71;
    const bool jpeg = bytes[0] == 255 && bytes[1] == 216 && bytes[2] == 255;
    if (!input || (!png && !jpeg))
      throw std::runtime_error("Artwork must be PNG or JPEG");
    static const char hex[] = "0123456789abcdef";
    std::string encoded;
    encoded.reserve(bytes.size() * 2);
    for (auto b : bytes) {
      encoded += hex[b >> 4];
      encoded += hex[b & 15];
    }
    bank.document["artwork"] = encoded;
    bank.document["artwork_type"] = png ? "png" : "jpg";
    status = "Loopbank artwork embedded in bank and project";
    changed();
  } catch (const std::exception &e) {
    status = e.what();
  }
}

void MusicGeneratorLoops::storeWorkspace(Bank &snapshot) const {
  snapshot.document["workspace"] = {{"selected", selected},
                                    {"page", page},
                                    {"instrument", instrument},
                                    {"prompt", prompt},
                                    {"key", key},
                                    {"bpm", bpm},
                                    {"beats", beats},
                                    {"steps", steps},
                                    {"variants", variants},
                                    {"seed", seed},
                                    {"host_sync", hostSync},
                                    {"snap", snap}};
}
void MusicGeneratorLoops::restoreWorkspace() {
  const auto &w = bank.document.value("workspace", Json::object());
  selected = std::clamp(w.value("selected", selected), 0, 127);
  page = std::clamp(w.value("page", selected / 16), 0, 7);
  instrument = w.value("instrument", instrument);
  prompt = w.value("prompt", prompt).substr(0, 8000);
  key = w.value("key", bank[selected].key).substr(0, 80);
  bpm = std::clamp(w.value("bpm", bank[selected].bpm), 30.0, 300.0);
  beats = std::clamp(w.value("beats", bank[selected].beats), .25, 1200.0);
  steps = std::clamp(w.value("steps", steps), 1, 100);
  variants = std::clamp(w.value("variants", variants), 1, 8);
  seed = w.value("seed", seed);
  hostSync = w.value("host_sync", hostSync);
#ifdef APP_API
  hostSync = false;
#endif
  followHostGeneration = hostSync;
  snap = std::clamp(w.value("snap", snap), 0, 8);
  loadTemplates();
}
