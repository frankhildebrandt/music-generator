#include "MusicGeneratorLoops.h"
#include <fstream>
using namespace loops;
void MusicGeneratorLoops::enqueue(Task task) {
  {
    std::lock_guard<std::mutex> lock(workMutex);
    if (work.size() > 64) {
      status = "Background queue is full";
      return;
    }
    work.push_back(std::move(task));
  }
  wake.notify_one();
}
void MusicGeneratorLoops::OnIdle() {
  std::lock_guard<std::recursive_mutex> lock(stateMutex);
  player.collect();
#ifdef APP_API
  audioBpm.store(bpm);
#endif
  hostBpm = audioBpm.load();
  if (followHostGeneration)
    bpm = hostBpm;
  hostRate = audioRate.load();
  barBeats = audioBarBeats.load();
  numerator = audioNumerator.load();
  denominator = audioDenominator.load();
  std::deque<Json> ready;
  {
    std::lock_guard<std::mutex> lock(workMutex);
    ready.swap(replies);
  }
  for (const auto &r : ready) {
    try {
      processReply(r);
    } catch (const std::exception &e) {
      busy = false;
      status = e.what();
    }
  }
  if (!job.empty() && !pollPending &&
      std::chrono::steady_clock::now() - lastPoll >
          std::chrono::milliseconds(500)) {
    pollPending = true;
    lastPoll = std::chrono::steady_clock::now();
    auto jid = job;
    const double rate = hostRate, tempo = hostBpm;
    const bool preparingTakes = generationJob;
    enqueue([this, jid, rate, tempo, preparingTakes]() {
      auto data = service.request("job", {{"id", jid}});
      if (preparingTakes && data.contains("results"))
        for (auto &result : data["results"])
          result = prepareTake(result, rate, tempo);
      return Json{{"type", "job"}, {"data", data}};
    });
  }
  if (GetUI())
    GetUI()->SetAllControlsDirty();
}
void MusicGeneratorLoops::processReply(const Json &r) {
  const auto type = r.value("type", "");
  if (type == "error") {
    status = r.value("message", "Error");
    pollPending = false;
    busy = false;
    job.clear();
    if (generationJob) {
      updateTakes({{"status", "error"}, {"message", status}});
      generationJob = false;
    }
    return;
  }
  if (type == "enqueued") {
    job = r.at("data").at("id");
    if (r.contains("models")) {
      models = r.at("models");
      status = "Checking shared app models: " + models;
    } else
      status = "Queued";
    if (cancelRequested)
      cancel();
    return;
  }
  if (type == "job") {
    pollPending = false;
    auto data = r.at("data");
    status = data.value("message", "");
    const auto state = data.value("status", "");
    if (generationJob)
      updateTakes(data);
    if (state == "done" || state == "error" || state == "cancelled") {
      job.clear();
      busy = false;
      if (generationJob && data.contains("results") &&
          !data["results"].empty()) {
        candidates = data["results"];
        variant = 0;
      }
      generationJob = false;
    }
    return;
  }
  if (type == "import") {
    int slot = r.at("slot");
    const auto &results = r.at("data").at("results");
    if (results.size() > size_t(Slots - slot))
      throw std::runtime_error(
          "Not enough slots; select an earlier starting note");
    const auto previous = bank;
    try {
      for (const auto &result : results)
        loadResult(slot++, result);
    } catch (...) {
      bank = previous;
      throw;
    }
    slot = r.at("slot");
    for (size_t i = 0; i < results.size(); ++i, ++slot) {
      history.push_back({slot, previous[slot]});
      publish(slot);
      for (int param = 0; param < ParamsPerSlot; ++param) {
        const int id = slot * ParamsPerSlot + param;
        SendParameterValueFromUI(
            id, GetParam(id)->ToNormalized(bank[slot].params[param]));
      }
    }
    future.clear();
    status = "Imported. Set BPM, key and length for plain audio exports.";
    busy = false;
    changed();
    return;
  }
  if (type == "edit") {
    int slot = r.at("slot");
    remember(slot);
    bank[slot].edited = std::make_shared<Audio>(readWav(
        std::filesystem::u8path(r.at("data").at("path").get<std::string>())));
    bank[slot].recipe = r.at("recipe");
    bank[slot].params[Start] = 0;
    bank[slot].params[End] = 1;
    publish(slot);
    if (slot == selected) {
      parameter(Start, 0);
      parameter(End, 1);
    }
    status = "Edit applied; original preserved";
    busy = false;
    changed();
    return;
  }
  if (type == "zero") {
    const int slot = r.at("slot");
    if (bank[slot].edited) {
      selected = slot;
      page = slot / 16;
      const double size = bank[slot].edited->samples.size();
      const double start = r.at("data").at("start").get<double>();
      remember(slot);
      parameter(Start, start / size);
      parameter(End, (start + r.at("frames").get<double>()) / size);
    }
    busy = false;
    status = "Stereo seam score: " +
             std::to_string(r.at("data").value("score", 0.0)) +
             "; selection length preserved";
    return;
  }
  if (type == "export") {
    exportPath = r.at("path");
    status = "WAV ready — drag the WAV button into REAPER";
    busy = false;
    return;
  }
}
void MusicGeneratorLoops::remember(int slot) {
  history.push_back({slot, bank[slot]});
  if (history.size() > 50)
    history.erase(history.begin());
  future.clear();
}
void MusicGeneratorLoops::undo(bool redo) {
  auto &from = redo ? future : history;
  auto &to = redo ? history : future;
  if (from.empty())
    return;
  auto entry = from.back();
  from.pop_back();
  to.push_back({entry.slot, bank[entry.slot]});
  bank[entry.slot] = entry.value;
  selected = entry.slot;
  page = selected / 16;
  publish(selected);
  for (int p = 0; p < 8; ++p)
    parameter(p, bank[selected].params[p]);
  status = redo ? "Redo" : "Undo";
  changed();
}
void MusicGeneratorLoops::loadResult(int slot, const Json &r) {
  auto s = bank[slot];
  s.edited = std::make_shared<Audio>(
      readWav(std::filesystem::u8path(r.at("path").get<std::string>())));
  s.original = std::make_shared<Audio>(readWav(std::filesystem::u8path(
      r.value("original", r.at("path").get<std::string>()))));
  s.metadata = r;
  s.recipe = r.value("recipe", Json::object());
  s.name = r.value("name", "Loop");
  s.bpm =
      r.contains("bpm") && !r["bpm"].is_null() ? r["bpm"].get<double>() : bpm;
  s.beats = r.contains("beats") && !r["beats"].is_null()
                ? r["beats"].get<double>()
                : s.edited->samples.size() / s.edited->rate * s.bpm / 60;
  s.key = r.contains("key") && !r["key"].is_null() ? r["key"].get<std::string>()
                                                   : key;
  s.params = {0, 1, 0, 0, 20, 20000, 0, 1};
  bank[slot] = std::move(s);
}
void MusicGeneratorLoops::generate() {
  if (busy)
    return;
  busy = true;
  jobSlot = selected;
  stopPreview();
  generationJob = true;
  cancelRequested = false;
  if (takes.size() + variants > 128)
    takes.erase(takes.begin(), takes.begin() + (takes.size() + variants - 128));
  generationStart = takes.size();
  for (int n = 0; n < variants; ++n) {
    GeneratedTake take;
    take.title = instrument + " · Take " + std::to_string(n + 1);
    take.phase = n == 0 ? "Preparing context" : "Waiting for previous take";
    take.bpm = bpm;
    take.progress = 0;
    take.bars = beats / barBeats;
    takes.push_back(std::move(take));
  }
  candidates = Json::array();
  status = "Preparing immutable context";
  Json request = {{"slot", selected},
                  {"instrument", instrument},
                  {"prompt", prompt},
                  {"key", key},
                  {"bpm", bpm},
                  {"beats", beats},
                  {"numerator", numerator},
                  {"denominator", denominator},
                  {"seed", seed},
                  {"steps", steps},
                  {"variants", variants},
                  {"context", Json::array()}};
  std::vector<std::pair<int, Clip>> sources;
  std::vector<std::array<double, 8>> params;
  for (int s = 0; s < Slots; ++s)
    if (context[s] && s != selected && bank[s].edited) {
      sources.push_back({s, bank[s].clip()});
      std::array<double, 8> p;
      for (int n = 0; n < 8; ++n)
        p[n] = GetParam(s * 8 + n)->Value();
      params.push_back(p);
    }
  enqueue([this, request, sources, params]() mutable {
    takeCache.clear();
    auto dir = profileRoot() / "input";
    std::filesystem::create_directories(dir);
    std::vector<std::filesystem::path> paths;
    try {
      for (size_t n = 0; n < sources.size(); ++n) {
        auto path =
            dir /
            (std::to_string(
                 std::chrono::steady_clock::now().time_since_epoch().count()) +
             ".wav");
        writeWav(path, renderCycle(sources[n].second, params[n], request["bpm"],
                                   48000));
        paths.push_back(path);
        request["context"].push_back(
            {{"slot", sources[n].first}, {"path", path.u8string()}});
      }
      auto result = service.request("generate", request);
      for (auto path : paths)
        std::filesystem::remove(path);
      return Json{{"type", "enqueued"}, {"data", result}};
    } catch (...) {
      for (auto path : paths)
        std::filesystem::remove(path);
      throw;
    }
  });
}
void MusicGeneratorLoops::install() {
  if (busy)
    return;
  busy = true;
  generationJob = false;
  cancelRequested = false;
  status = "Checking models in the app data folder...";
  enqueue([this]() {
    auto location = service.request("models");
    return Json{{"type", "enqueued"},
                {"data", service.request("install")},
                {"models", location.at("folder")}};
  });
}
void MusicGeneratorLoops::cancel() {
  cancelRequested = true;
  if (job.empty())
    return;
  auto id = job;
  enqueue([this, id]() {
    service.request("cancel", {{"id", id}});
    return Json{{"type", "job"},
                {"data",
                 {{"status", "cancelled"},
                  {"message", "Cancelled; current loop preserved"}}}};
  });
}
void MusicGeneratorLoops::importFile(const std::string &path) {
  if (busy)
    return;
  busy = true;
  int slot = selected;
  enqueue([this, path, slot]() {
    return Json{{"type", "import"},
                {"slot", slot},
                {"data", service.request("import", {{"path", path}})}};
  });
}
void MusicGeneratorLoops::accept() {
  if (candidates.empty() || jobSlot < 0)
    return;
  stopPreview();
  int slot = jobSlot;
  remember(slot);
  loadResult(slot, candidates.at(variant));
  publish(slot);
  selected = slot;
  page = slot / 16;
  for (int p = 0; p < 8; ++p)
    parameter(p, bank[slot].params[p]);
  status = "Variant accepted. Original and audio saved with the project.";
  changed();
}
void MusicGeneratorLoops::audition(bool original) {
  if (!bank[selected].edited)
    return;
  auto c = bank[selected].clip();
  c.audio = original ? bank[selected].original : bank[selected].edited;
  player.publish(selected, std::make_unique<Clip>(c));
  compareOriginal = original;
  trigger(selected, true);
  status = original ? "Audition original" : "Audition edited loop";
}
void MusicGeneratorLoops::edit(bool normalize, bool noise, bool fit) {
  auto s = bank[selected];
  if (busy || !s.original)
    return;
  busy = true;
  int slot = selected;
  Json recipe = Json::object();
  recipe["start"] = uint64_t(GetParam(selected * 8 + Start)->Value() *
                             s.edited->samples.size());
  recipe["end"] = uint64_t(GetParam(selected * 8 + End)->Value() *
                           s.edited->samples.size());
  recipe["frames"] = uint64_t(std::llround(beats * 60.0 / bpm * 48000));
  recipe["crossfade_ms"] = crossfade;
  if (normalize)
    recipe["normalize_db"] = -1.0;
  if (noise) {
    recipe["noise_start"] = uint64_t(noiseStart * 48000);
    recipe["noise_end"] = uint64_t(noiseEnd * 48000);
    recipe["noise_db"] = noiseDb;
  }
  auto original = s.edited;
  auto previous = s.recipe;
  const double barFrames = barBeats * 60.0 / bpm * 48000;
  enqueue([this, slot, recipe, original, previous, fit, barFrames]() {
    auto path =
        profileRoot() / "input" /
        ("edit-" +
         std::to_string(
             std::chrono::steady_clock::now().time_since_epoch().count()) +
         ".wav");
    std::filesystem::create_directories(path.parent_path());
    writeWav(path, *original);
    try {
      auto result = service.request("edit", {{"path", path.u8string()},
                                             {"recipe", recipe},
                                             {"fit", fit},
                                             {"bar_frames", barFrames}});
      std::filesystem::remove(path);
      return Json{
          {"type", "edit"},
          {"slot", slot},
          {"recipe",
           {{"previous", previous}, {"stage", result.value("recipe", recipe)}}},
          {"data", result}};
    } catch (...) {
      std::filesystem::remove(path);
      throw;
    }
  });
}
void MusicGeneratorLoops::zero() {
  const auto s = bank[selected];
  if (busy || !s.edited)
    return;
  busy = true;
  const int slot = selected;
  const auto start = uint64_t(GetParam(selected * 8 + Start)->Value() *
                              s.edited->samples.size());
  const auto frames = uint64_t((GetParam(selected * 8 + End)->Value() -
                                GetParam(selected * 8 + Start)->Value()) *
                               s.edited->samples.size());
  auto audio = s.edited;
  enqueue([this, audio, start, frames, slot]() {
    auto path =
        profileRoot() / "input" /
        ("zero-" +
         std::to_string(
             std::chrono::steady_clock::now().time_since_epoch().count()) +
         ".wav");
    std::filesystem::create_directories(path.parent_path());
    writeWav(path, *audio);
    auto result = service.request(
        "zero",
        {{"path", path.u8string()}, {"start", start}, {"frames", frames}});
    std::filesystem::remove(path);
    return Json{
        {"type", "zero"}, {"slot", slot}, {"frames", frames}, {"data", result}};
  });
}
void MusicGeneratorLoops::exportWav() {
  if (busy || !bank[selected].edited)
    return;
  busy = true;
  auto c = bank[selected].clip();
  std::array<double, 8> p;
  for (int n = 0; n < 8; ++n)
    p[n] = GetParam(selected * 8 + n)->Value();
  auto name = bank[selected].name + "-" + bank[selected].key + "-" +
              std::to_string(int(hostBpm)) + "bpm-" +
              std::to_string(bank[selected].beats / barBeats) + "bars";
  for (auto &ch : name)
    if (!std::isalnum(static_cast<unsigned char>(ch)) && ch != '-' && ch != '.')
      ch = '_';
  double tempo = hostBpm, rate = hostRate;
  enqueue([c, p, name, tempo, rate]() {
    auto dir = profileRoot() / "exports";
    std::filesystem::create_directories(dir);
    auto path =
        dir /
        (name + "-" +
         std::to_string(
             std::chrono::steady_clock::now().time_since_epoch().count()) +
         ".wav");
    writeWav(path, renderCycle(c, p, tempo, rate));
    return Json{{"type", "export"}, {"path", path.u8string()}};
  });
}
void MusicGeneratorLoops::saveBankFile(const std::string &path) {
  try {
    auto snapshot = bank;
    for (int s = 0; s < 128; ++s)
      for (int p = 0; p < 8; ++p)
        snapshot[s].params[p] = GetParam(s * 8 + p)->Value();
    auto bytes = saveBank(snapshot);
    std::ofstream out(std::filesystem::u8path(path), std::ios::binary);
    out.write(reinterpret_cast<const char *>(bytes.data()), bytes.size());
    if (!out)
      throw std::runtime_error("Bank save failed");
    status = "Portable loop bank saved";
  } catch (const std::exception &e) {
    status = e.what();
  }
}
void MusicGeneratorLoops::openBankFile(const std::string &path) {
  try {
    std::ifstream in(std::filesystem::u8path(path),
                     std::ios::binary | std::ios::ate);
    auto size = in.tellg();
    if (size < 12 || size > 512 * 1024 * 1024)
      throw std::runtime_error("Invalid bank size");
    std::vector<uint8_t> bytes(static_cast<size_t>(size));
    in.seekg(0);
    in.read(reinterpret_cast<char *>(bytes.data()), size);
    auto restored = loadBank(bytes.data(), bytes.size());
    bank = std::move(restored);
    for (int s = 0; s < 128; ++s) {
      publish(s);
      for (int p = 0; p < 8; ++p) {
        const int id = s * 8 + p;
        SendParameterValueFromUI(id,
                                 GetParam(id)->ToNormalized(bank[s].params[p]));
      }
    }
    history.clear();
    future.clear();
    bpm = bank[selected].bpm;
    beats = bank[selected].beats;
    key = bank[selected].key;
    status = "Portable loop bank loaded";
    changed();
  } catch (const std::exception &e) {
    status = e.what();
  }
}

void MusicGeneratorLoops::previewCandidate() {
  if (!candidates.empty() && jobSlot >= 0)
    auditionResult(candidates.at(variant), jobSlot);
}

void MusicGeneratorLoops::auditionResult(const Json &r, int slot) {
  try {
    stopPreview();
    auto c = std::make_unique<Clip>();
    c->audio = std::make_shared<Audio>(
        readWav(std::filesystem::u8path(r.at("path").get<std::string>())));
    c->bpm = r.value("bpm", 120.0);
    c->beats = r.value("beats", 16.0);
    c->mode = Mode::Toggle;
    previewSlot = slot;
    player.publish(slot, std::move(c));
    trigger(slot, true);
    status = "Auditioning take. Stop restores the assigned loop.";
  } catch (const std::exception &e) {
    status = e.what();
  }
}

void MusicGeneratorLoops::stopPreview() {
  auditionTake = -1;
  if (previewSlot < 0)
    return;
  player.uiParameter(previewSlot * ParamsPerSlot + Active, 0);
  publish(previewSlot);
  previewSlot = -1;
}
