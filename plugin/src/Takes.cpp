#include "MusicGeneratorLoops.h"
#include <cctype>
using namespace loops;

Json MusicGeneratorLoops::prepareTake(const Json &source, double rate,
                                      double tempo) {
  const auto cacheKey = source.at("path").get<std::string>() + ":" +
                        std::to_string(rate) + ":" + std::to_string(tempo);
  if (takeCache.contains(cacheKey))
    return takeCache.at(cacheKey);
  auto result = source;
  auto audio = std::make_shared<Audio>(
      readWav(std::filesystem::u8path(source.at("path").get<std::string>())));
  Json waveform = Json::array();
  constexpr size_t columns = 384;
  for (size_t column = 0; column < columns; ++column) {
    const size_t from = column * audio->samples.size() / columns;
    const size_t to =
        std::max(from + 1, (column + 1) * audio->samples.size() / columns);
    float low = 0, high = 0;
    for (size_t n = from; n < std::min(to, audio->samples.size()); ++n)
      for (float sample : audio->samples[n]) {
        low = std::min(low, sample);
        high = std::max(high, sample);
      }
    waveform.push_back({low, high});
  }
  Clip clip;
  clip.audio = audio;
  clip.bpm = source.value("bpm", 120.0);
  clip.beats = source.value("beats", 16.0);
  const std::array<double, ParamsPerSlot> neutral{0, 1, 0, 0, 20, 20000, 0, 1};
  const auto rendered = renderCycle(clip, neutral, tempo, rate);
  auto name = source.value("name", "Loop") + "-" +
              source.value("key", "Unknown") + "-" +
              std::to_string(int(std::round(tempo))) + "bpm-" +
              std::to_string(source.value("bars", clip.beats / 4)) + "bars";
  for (char &c : name)
    if (!std::isalnum(static_cast<unsigned char>(c)) && c != '-' && c != '.')
      c = '_';
  const auto directory = profileRoot() / "exports";
  std::filesystem::create_directories(directory);
  const auto path =
      directory /
      (name + "-take-" +
       std::to_string(
           std::chrono::steady_clock::now().time_since_epoch().count()) +
       ".wav");
  writeWav(path, rendered);
  result["waveform"] = std::move(waveform);
  result["export_path"] = path.u8string();
  result["export_bpm"] = tempo;
  result["export_rate"] = rate;
  takeCache[cacheKey] = result;
  return result;
}

void MusicGeneratorLoops::updateTakes(const Json &data) {
  const auto state = data.value("status", "queued");
  const auto message = data.value("message", "Queued");
  const size_t current = data.value("variant", size_t(0));
  const auto results = data.value("results", Json::array());
  for (size_t i = generationStart; i < takes.size(); ++i) {
    auto &take = takes[i];
    const size_t index = i - generationStart;
    if (index < results.size()) {
      take.result = results[index];
      take.result["sound_idea"] = take.idea;
      take.phase = "Ready";
      take.progress = 1;
      take.pending = false;
    } else if (take.result.empty()) {
      if (state == "error" || state == "cancelled") {
        take.phase = state == "cancelled" ? "Cancelled" : message;
        take.progress = -1;
        take.pending = false;
      } else if (index == current) {
        take.phase = message;
        take.progress =
            data.contains("progress") && data["progress"].is_number()
                ? data["progress"].get<double>()
                : -1;
      }
    }
  }
}

void MusicGeneratorLoops::previewTake(int index) {
  if (index < 0 || size_t(index) >= takes.size() || takes[index].result.empty())
    return;
  if (auditionTake == index) {
    stopPreview();
    return;
  }
  auditionResult(takes[index].result, selected);
  if (previewSlot >= 0)
    auditionTake = index;
}

double MusicGeneratorLoops::takePosition(int index) const {
  if (index < 0 || size_t(index) >= takes.size() || takes[index].result.empty())
    return -1;
  if (auditionTake == index)
    return player.position(previewSlot);
  for (int slot = 0; slot < Slots; ++slot)
    if (bank[slot].metadata.contains("path") &&
        bank[slot].metadata["path"] == takes[index].result["path"] &&
        player.position(slot) >= 0)
      return player.position(slot);
  return -1;
}

void MusicGeneratorLoops::assignTake(int index, int note) {
  if (index < 0 || size_t(index) >= takes.size() || note < 0 || note >= Slots ||
      takes[index].result.empty())
    return;
  stopPreview();
  remember(note);
  loadResult(note, takes[index].result);
  publish(note);
  selected = note;
  page = note / 16;
  for (int param = 0; param < ParamsPerSlot; ++param)
    parameter(param, bank[note].params[param]);
  beats = bank[note].beats;
  key = bank[note].key;
  status = "Take assigned to MIDI note " + std::to_string(note) +
           ". Undo restores the previous loop.";
  changed();
}
