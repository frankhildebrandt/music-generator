#include "Editor.h"
namespace loops::ui {
void LoopEditor::stereoWave(IGraphics &g, IRECT r, bool editing) {
  const auto &slot = p.bank[p.selected];
  const auto audio = p.compareOriginal ? slot.original : slot.edited;
  g.FillRoundRect(IColor(255, 16, 23, 26), r, 4);
  g.DrawRoundRect(Line, r, 4);
  if (!audio || audio->samples.empty()) {
    label(g, "Import audio or generate your first loop", r.GetPadded(-25), 18,
          Muted);
    return;
  }
  if (audio != waveAudio) {
    waveAudio = audio;
    for (size_t x = 0; x < waveBins.size(); ++x) {
      auto &bin = waveBins[x];
      bin.fill(0);
      const auto end =
          std::min(audio->samples.size(),
                   std::max(x * audio->samples.size() / waveBins.size() + 1,
                            (x + 1) * audio->samples.size() / waveBins.size()));
      for (size_t n = x * audio->samples.size() / waveBins.size(); n < end; ++n)
        for (int ch = 0; ch < 2; ++ch) {
          bin[ch * 2] = std::min(bin[ch * 2], audio->samples[n][ch]);
          bin[ch * 2 + 1] = std::max(bin[ch * 2 + 1], audio->samples[n][ch]);
        }
    }
  }
  const double visible = editing ? 1 / p.zoom : 1;
  const double from = editing ? std::clamp(p.viewStart, 0.0, 1 - visible) : 0;
  IRECT content(r.L + 23, r.T + 23, r.R - 12, r.B - 8);
  if (editing)
    waveBounds = content;
  for (int beat = 0; beat <= std::ceil(slot.beats); ++beat) {
    const float x =
        content.L + float((beat / slot.beats - from) / visible) * content.W();
    if (x < content.L || x > content.R)
      continue;
    const bool bar = std::fmod(beat, p.barBeats) < .001;
    g.DrawLine(bar ? Line : IColor(255, 34, 43, 47), x, content.T, x,
               content.B);
    if (bar) {
      const bool end = beat >= slot.beats;
      label(g, end ? "END" : std::to_string(int(beat / p.barBeats) + 1),
            {end ? x - 35 : x + 2, r.T + 3, end ? x : x + 37, r.T + 20}, 10,
            Muted, end ? EAlign::Far : EAlign::Near);
    }
  }
  for (int ch = 0; ch < 2; ++ch) {
    const float center = content.T + (ch + .5f) * content.H() / 2;
    label(g, ch ? "R" : "L", {r.L + 5, center - 10, r.L + 21, center + 10}, 10,
          Muted);
    g.DrawLine(Line, content.L, center, content.R, center);
    for (int x = 0; x < int(content.W()); ++x) {
      const auto bin = std::min(
          waveBins.size() - 1,
          size_t((from + x / content.W() * visible) * waveBins.size()));
      const float scale = content.H() * .22f;
      g.DrawLine(editing ? IColor(255, 215, 228, 166) : Teal, content.L + x,
                 center -
                     std::clamp(waveBins[bin][ch * 2 + 1], -1.f, 1.f) * scale,
                 content.L + x,
                 center - std::clamp(waveBins[bin][ch * 2], -1.f, 1.f) * scale);
    }
  }
  auto xFor = [&](double ratio) {
    return content.L + float((ratio - from) / visible) * content.W();
  };
  const double start = p.GetParam(p.selected * 8 + Start)->Value(),
               end = p.GetParam(p.selected * 8 + End)->Value();
  if (editing) {
    for (double boundary : {start, end}) {
      const float x = xFor(boundary);
      if (x < content.L || x > content.R)
        continue;
      g.DrawLine(Mint, x, content.T, x, content.B, nullptr, 1.5f);
      g.FillRoundRect(Mint, {x - 4, content.T - 4, x + 4, content.T + 5}, 1);
    }
    if (p.noiseEnd != p.noiseStart) {
      const double duration = audio->samples.size() / audio->rate;
      const float a = std::clamp(
                      xFor(std::min(p.noiseStart, p.noiseEnd) / duration),
                      content.L, content.R),
                  b = std::clamp(
                      xFor(std::max(p.noiseStart, p.noiseEnd) / duration),
                      content.L, content.R);
      g.FillRect(IColor(45, 127, 230, 205), {a, content.T, b, content.B});
      label(g, "Noise print", {a + 5, content.T + 5, b, content.T + 22}, 8,
            Teal);
    }
    const double duration = audio->samples.size() / audio->rate;
    const float fade =
        float(p.crossfade / 1000 / duration / visible * content.W());
    for (int side = 0; side < 2; ++side) {
      const float edge = xFor(side ? end : start);
      if (edge < content.L || edge > content.R)
        continue;
      for (int n = 1; n < 24; ++n) {
        const float t = n / 24.f, prev = (n - 1) / 24.f;
        const float sign = side ? -1.f : 1.f;
        g.DrawLine(Gold, edge + sign * fade * prev,
                   content.T + content.H() * .5f * prev, edge + sign * fade * t,
                   content.T + content.H() * .5f * t);
      }
    }
    hits.push_back({content, "wave"});
  }
  const double position = p.playPosition();
  if (position >= 0) {
    const float x = xFor(start + position * (end - start));
    if (x >= content.L && x <= content.R) {
      g.DrawLine(Mint, x, content.T, x, content.B, nullptr, 1.5f);
      g.FillTriangle(Mint, x - 5, content.T - 4, x + 5, content.T - 4, x,
                     content.T + 3);
    }
  }
}
void LoopEditor::knobs(IGraphics &g, IRECT r, bool vertical) {
  auto &slot = p.bank[p.selected];
  static const char *names[]{"Play",      "LEVEL", "PAN",        "TUNE",
                             "HIGH PASS", "TONE",  "LOOP START", "LOOP END"};
  for (int n = 0; n < 4; ++n) {
    const int param = slot.macros[n];
    const float cx = vertical ? r.L + (n % 2 + .5f) * r.W() / 2
                              : r.L + (n + .5f) * r.W() / 4;
    const float cy = vertical ? r.T + (n / 2 + .5f) * r.H() / 2 : r.MH();
    const float radius = vertical ? 38 : 32;
    const double value = p.GetParam(p.selected * 8 + param)->Value(),
                 norm = p.GetParam(p.selected * 8 + param)->ToNormalized(value);
    label(g, "M" + std::to_string(n + 1) + "  " + names[param],
          {cx - 88, cy - radius - 27, cx + 88, cy - radius - 7}, 11, Muted,
          EAlign::Center);
    g.DrawArc(Line, cx, cy, radius + 4, -135, 135, nullptr, 5);
    g.DrawArc(Mint, cx, cy, radius + 4, -135, float(-135 + norm * 270), nullptr,
              4);
    g.FillCircle(IColor(255, 16, 21, 23), cx + 2, cy + 3, radius);
    g.FillCircle(Panel, cx, cy, radius - 2);
    const double angle = (-225 + norm * 270) * 3.141592653589793 / 180;
    g.DrawLine(Mint, cx + float(std::cos(angle)) * radius * .55f,
               cy + float(std::sin(angle)) * radius * .55f,
               cx + float(std::cos(angle)) * radius * .91f,
               cy + float(std::sin(angle)) * radius * .91f, nullptr, 2);
    std::string display =
        param == Gain
            ? number(value > 0 ? 20 * std::log10(value) : -90, 1) + " dB"
        : param == Pan
            ? (std::abs(value) < .001 ? "C"
                                      : number(std::abs(value) * 100, 0) +
                                            (value < 0 ? " L" : " R"))
            : number(value, param == Pitch ? 1 : 0) +
                  (param == Pitch                          ? " st"
                   : param == Lowpass || param == Highpass ? " Hz"
                                                           : "");
    label(g, display, {cx - 88, cy + radius + 8, cx + 88, cy + radius + 28}, 12,
          White, EAlign::Center);
    hits.push_back(
        {{cx - radius - 7, cy - radius - 3, cx + radius + 7, cy + radius + 4},
         "param" + std::to_string(param)});
    if (mappingOpen)
      button(g, {cx - 61, cy - 12, cx + 61, cy + 14},
             "Map M" + std::to_string(n + 1), "map" + std::to_string(n));
  }
}
void LoopEditor::performance(IGraphics &g) {
  auto &slot = p.bank[p.selected];
  g.FillRoundRect(Panel, {10, 106, 1110, 564}, 6);
  g.DrawRoundRect(Line, {10, 106, 1110, 564}, 6);
  label(g, noteName(p.selected) + " / " + slot.name, {27, 112, 500, 142}, 24);
  label(g,
        slot.metadata.value("instrument",
                            slot.metadata.value("name", std::string("Loop"))) +
            " · " + slot.key + " · " + number(slot.beats / p.barBeats, 0) +
            " BARS · MIDI " + std::to_string(p.selected),
        {28, 144, 520, 166}, 10, Muted);
  hits.push_back({{27, 112, 480, 142}, "renameSlot"});
  if (slot.group)
    button(g, {500, 117, 677, 142}, groupName(slot.group), "assignGroup");
  button(g, {691, 117, 824, 147},
         "Choke: " + (slot.choke ? std::to_string(slot.choke) : "Off"),
         "choke");
  button(g, {838, 117, 961, 147}, "Waveform", "waveView", !coverView);
  button(g, {965, 117, 1097, 147}, "Coverart", "coverView", coverView);
  if (!coverView) {
    stereoWave(g, {22, 171, 1099, 348}, false);
    knobs(g, {40, 400, 1016, 508}, false);
    button(g, {1017, 421, 1099, 449}, "Map", "mapping", mappingOpen);
  } else {
    IRECT art(24, 173, 603, 458);
    g.FillRoundRect(Background, art, 4);
    const auto &doc = p.bank.document;
    if (doc.contains("artwork")) {
      const auto &source = doc["artwork"].get_ref<const std::string &>();
      if (source.data() != artPointer || source.size() != artSize) {
        artPointer = source.data();
        artSize = source.size();
        std::vector<uint8_t> data;
        auto digit = [](char c) {
          return c >= '0' && c <= '9'   ? c - '0'
                 : c >= 'a' && c <= 'f' ? c - 'a' + 10
                                        : -1;
        };
        for (size_t n = 0; n + 1 < source.size(); n += 2) {
          if (digit(source[n]) < 0 || digit(source[n + 1]) < 0)
            break;
          data.push_back(uint8_t(digit(source[n]) * 16 + digit(source[n + 1])));
        }
        artKey =
            "loopforge-art-" +
            std::to_string(
                std::chrono::steady_clock::now().time_since_epoch().count()) +
            "." + doc.value("artwork_type", "png");
        if (!data.empty())
          artwork = g.LoadBitmap(artKey.c_str(), data.data(), int(data.size()));
      }
      if (artwork.IsValid()) {
        const float fit =
            std::min(art.W() / artwork.W(), art.H() / artwork.H());
        const float width = artwork.W() * fit, height = artwork.H() * fit;
        g.DrawFittedBitmap(artwork,
                           {art.MW() - width * .5f, art.MH() - height * .5f,
                            art.MW() + width * .5f, art.MH() + height * .5f});
      }
    } else {
      // A code-native bank cover remains available before the user imports art.
      for (int n = 0; n < 35; ++n)
        g.DrawLine(IColor(255, 35 + n, 85 + n * 2, 89 + n * 2), art.L,
                   art.T + n * 9, art.R, art.B - n * 4, nullptr, 3);
      label(g, p.bank.document.value("name", "LOOPFORGE"),
            {art.L + 24, art.B - 80, art.R - 20, art.B - 30}, 32);
    }
    button(g, {24, 462, 603, 485}, "Loopbank artwork · Import PNG / JPEG",
           "artwork");
    label(g, "MACROS  ·  Selected sound", {627, 169, 970, 195}, 13);
    button(g, {1020, 168, 1098, 194}, "Map", "mapping", mappingOpen);
    knobs(g, {624, 218, 1090, 497}, true);
  }
  const double pos = p.playPosition();
  const float y = coverView ? 503 : 368;
  button(g, {24, y - 11, 70, y + 19}, pos >= 0 ? "II" : ">", "masterplay");
  g.FillRoundRect(Line, {82, y, 433, y + 5}, 2);
  if (pos >= 0)
    g.FillRoundRect(Mint, {82, y, 82 + float(pos) * 351, y + 5}, 2);
  label(g,
        pos >= 0
            ? musicalPosition(pos * slot.beats, p.barBeats, p.denominator) +
                  " / " + number(slot.beats / p.barBeats, 0) + " BARS"
            : "READY",
        {447, y - 8, 610, y + 14}, 10, Muted);
  std::string next = "No queued switch";
  for (int n = 0; n < Slots; ++n)
    if (p.queued(n) && (slot.group == 0 || p.bank[n].group == slot.group))
      next = "NEXT: " + noteName(n) + " / " + p.bank[n].name + " · " + timing();
  label(g, next, {650, y - 9, 1090, y + 15}, 11,
        next == "No queued switch" ? Muted : Gold);
  button(g, {23, 528, 132, 553},
         slot.group || slot.mode == Mode::Toggle ? "Toggle"
         : slot.mode == Mode::Gate               ? "Gate"
                                                 : "One-shot",
         "mode");
  button(g, {141, 528, 329, 553}, "Timing: Global · " + timing(), "global");
  button(g, {337, 528, 462, 553},
         slot.group       ? "Keep position"
         : slot.retrigger ? "Retrigger: On"
                          : "Retrigger: Off",
         "retrigger");
  button(g, {470, 528, 611, 553},
         slot.group ? groupName(slot.group) : "Sound group: Off",
         "assignGroup");
  button(g, {619, 528, 737, 553},
         slot.bus ? "Stereo " + std::to_string(slot.bus + 1) : "Master",
         "busSelect");
  button(g, {745, 528, 846, 553}, "Edit loop", "view1");
  button(g, {854, 528, 960, 553}, "Import", "import");
  button(g, {968, 528, 1098, 553},
         p.exportPath.empty() ? "Render WAV" : "Drag WAV", "export");
}
void LoopEditor::DrawDetail(IGraphics &g) {
  const auto &slot = p.bank[p.selected];
  g.FillRoundRect(Panel, {10, 106, 836, 562}, 6);
  label(g, noteName(p.selected) + " / " + slot.name, {25, 111, 435, 142}, 23);
  label(g,
        slot.key + " · MIDI " + std::to_string(p.selected) + " · " +
            groupName(slot.group),
        {26, 143, 430, 164}, 10, Muted);
  button(g, {444, 119, 514, 147}, "Undo", "undo");
  button(g, {521, 119, 589, 147}, "Redo", "redo");
  button(g, {600, 119, 702, 147}, "Original", "original", p.compareOriginal);
  button(g, {708, 119, 820, 147}, "Edited", "edited", !p.compareOriginal);
  label(g,
        "CYCLE " + number(slot.beats / p.barBeats, 0) + " BARS · " +
            (slot.edited
                 ? number(slot.edited->samples.size() / slot.edited->rate, 3)
                 : "0") +
            " s  ·  " + number(p.hostBpm, 1) + " BPM",
        {25, 167, 530, 190}, 11);
  button(g, {578, 164, 682, 187},
         p.snap ? "Snap: 1/" + std::to_string(p.snap * 4) : "Snap: Off",
         "snap");
  button(g, {691, 164, 728, 187}, "−", "zoomout");
  button(g, {735, 164, 775, 187}, "+", "zoomin");
  button(g, {781, 164, 821, 187}, ">", "play");
  stereoWave(g, {22, 192, 823, 374}, true);
  const double duration =
      slot.edited ? slot.edited->samples.size() / slot.edited->rate : 0;
  input(g, {24, 382, 173, 430}, "START / SEC",
        number(duration * p.GetParam(p.selected * 8 + Start)->Value(), 3),
        "startsec");
  input(g, {182, 382, 328, 430}, "END / SEC",
        number(duration * p.GetParam(p.selected * 8 + End)->Value(), 3),
        "endsec");
  input(g, {339, 382, 470, 430}, "CROSSFADE / MS", number(p.crossfade, 1),
        "fade");
  std::string zeroLabel = "Zero check";
  if (slot.edited && !slot.edited->samples.empty()) {
    const auto frames = slot.edited->samples.size();
    const size_t start =
        std::min(frames - 1,
                 size_t(p.GetParam(p.selected * 8 + Start)->Value() * frames));
    const size_t end = std::min(
        frames - 1,
        std::max(start + 1,
                 size_t(p.GetParam(p.selected * 8 + End)->Value() * frames)) -
            1);
    zeroLabel = "Zero ";
    for (int ch = 0; ch < 2; ++ch) {
      const float a = slot.edited->samples[start][ch],
                  b = slot.edited->samples[end][ch];
      zeroLabel +=
          std::string(ch ? " R" : "L") +
          (std::abs(a - b) < .005 && std::abs(a) < .01 && std::abs(b) < .01
               ? ":ok"
               : ":!");
    }
  }
  button(g, {480, 402, 591, 430}, zeroLabel, "zero");
  button(g, {600, 402, 706, 430}, "Apply fade", "trim");
  button(g, {715, 402, 822, 430}, "Stop", "stop");
  g.DrawLine(Line, 24, 441, 822, 441);
  label(g, "PREPARE", {24, 449, 269, 468}, 11, Teal);
  button(g, {24, 476, 120, 507}, "Fit loop", "fit", true);
  button(g, {129, 476, 248, 507}, "Normalize", "normalize");
  const bool dc =
      slot.recipe.value("stage", slot.recipe).value("remove_dc", false);
  label(g,
        dc ? "−1 dB · DC removed · preserve cycle" : "−1 dB · preserve cycle",
        {24, 517, 276, 544}, 9, Muted);
  label(g, "FILTER", {286, 449, 512, 468}, 11, Teal);
  input(g, {286, 473, 402, 521}, "HP / HZ",
        number(p.GetParam(p.selected * 8 + Highpass)->Value(), 0), "param4");
  input(g, {410, 473, 525, 521}, "LP / HZ",
        number(p.GetParam(p.selected * 8 + Lowpass)->Value(), 0), "param5");
  label(g, "HISS REDUCTION", {552, 449, 820, 468}, 11, Teal);
  button(g, {552, 476, 677, 507}, "Learn / apply", "denoise");
  button(g, {685, 476, 821, 507}, number(p.noiseDb, 1) + " dB", "noisedb");
  label(g,
        p.noiseEnd != p.noiseStart
            ? "Noise print selected · Shift-drag to change"
            : "Shift-drag waveform to select noise print",
        {552, 517, 825, 544}, 8, Muted);
  groups(g);
}
} // namespace loops::ui
