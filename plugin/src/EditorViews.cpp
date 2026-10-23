#include "Editor.h"
namespace loops::ui {
void LoopEditor::DrawDetail(IGraphics &g) {
  std::lock_guard<std::recursive_mutex> lock(p.stateMutex);
  hits.clear();
  g.FillRect(Background, mRECT);
  label(g, "LOOPS", {24, 14, 500, 42}, 22);
  label(g, "ALPHATESTER x KRGSH", {25, 43, 500, 62}, 11, Mint);
  button(g, {650, 22, 760, 55}, "Import audio", "import");
  button(g, {770, 22, 865, 55}, "Open bank", "open");
  button(g, {875, 22, 970, 55}, "Save bank", "save");
  button(g, {980, 22, 1095, 55}, "Check models", "models");
  g.DrawLine(Line, 24, 74, 1096, 74);
  label(g, "MIDI MAP", {24, 84, 190, 110}, 12, Muted);
  button(g, {209, 83, 247, 110}, "<", "prevpage");
  label(g,
        "Notes " + std::to_string(p.page * 16) + "–" +
            std::to_string(p.page * 16 + 15),
        {260, 83, 406, 110}, 12);
  button(g, {406, 83, 444, 110}, ">", "nextpage");
  label(g, "Click: select · double-click: play · right-click: AI context",
        {470, 83, 1096, 110}, 11, Muted);
  for (int n = 0; n < 16; ++n) {
    int slot = p.page * 16 + n;
    IRECT r(24 + (n % 8) * 135, 120 + (n / 8) * 66, 151 + (n % 8) * 135,
            177 + (n / 8) * 66);
    button(g, r, std::to_string(slot) + "  " + p.bank[slot].name,
           "slot" + std::to_string(slot), slot == p.selected);
    if (p.context[slot])
      label(g, "AI", {r.R - 25, r.T + 3, r.R - 2, r.T + 18}, 10, Gold);
  }
  auto &slot = p.bank[p.selected];
  label(g, "NOTE " + std::to_string(p.selected) + " / " + slot.name,
        {24, 259, 620, 285}, 17);
  button(g, {690, 256, 784, 284}, "Play", "play");
  button(g, {794, 256, 888, 284}, "Stop", "stop");
  button(g, {898, 256, 992, 284}, "A: original", "original");
  button(g, {1002, 256, 1096, 284}, "B: edited", "edited");
  IRECT wave(24, 298, 1096, 438);
  g.FillRoundRect(Panel, wave, 5);
  g.DrawLine(Line, wave.L, wave.MH(), wave.R, wave.MH());
  if (slot.edited) {
    const auto &a = *slot.edited;
    size_t size = a.samples.size();
    double visible = 1 / p.zoom,
           from = std::clamp(p.viewStart, 0.0, 1 - visible);
    for (int x = 0; x < 1072; ++x) {
      size_t lo = size_t((from + x / 1072.0 * visible) * size),
             hi = std::min(
                 size, lo + std::max<size_t>(1, size_t(size * visible / 1072)));
      float low = 0, high = 0;
      for (size_t i = lo; i < hi; ++i) {
        low = std::min(low, std::min(a.samples[i][0], a.samples[i][1]));
        high = std::max(high, std::max(a.samples[i][0], a.samples[i][1]));
      }
      g.DrawLine(Mint, 24 + x, 368 - std::clamp(high, -1.0f, 1.0f) * 62, 24 + x,
                 368 - std::clamp(low, -1.0f, 1.0f) * 62);
    }
    for (int n = 0; n <= int(p.beats); ++n) {
      float x = 24 + float((n / p.beats - from) / visible) * 1072;
      if (x >= 24 && x <= 1096)
        g.DrawLine(Line, x, 302, x, 310);
    }
    for (int param : {Start, End}) {
      double value = p.GetParam(p.selected * 8 + param)->Value();
      float x = 24 + float((value - from) / visible) * 1072;
      if (x >= 24 && x <= 1096) {
        g.DrawLine(Gold, x, 298, x, 438, 0, 2);
        label(g, param == Start ? "IN" : "OUT", {x + 4, 308, x + 50, 328}, 10,
              Gold);
      }
    }
    const double position = p.playPosition();
    if (position >= 0) {
      const double start = p.GetParam(p.selected * 8 + Start)->Value();
      const double end = p.GetParam(p.selected * 8 + End)->Value();
      const float x =
          24 +
          float((start + position * (end - start) - from) / visible) * 1072;
      if (x >= wave.L && x <= wave.R) {
        g.DrawLine(White, x, wave.T, x, wave.B, nullptr, 1.5f);
        g.FillTriangle(White, x - 4, wave.T, x + 4, wave.T, x, wave.T + 6);
      }
    }
    label(g,
          number(size / a.rate, 3) + " s · " + number(slot.beats / p.barBeats) +
              " bars · " + slot.key,
          {36, 411, 800, 432}, 11, Muted);
  } else
    label(g,
          "Drop WAV, MP3 or an app ZIP here. Each MIDI note can hold a "
          "different loop.",
          {60, 342, 1050, 396}, 16, Muted);
  hits.push_back({wave, "wave"});
  input(g, {24, 452, 126, 505}, "MODE",
        slot.mode == Mode::Gate     ? "Gate"
        : slot.mode == Mode::Toggle ? "Toggle"
                                    : "One-shot",
        "mode");
  input(g, {136, 452, 238, 505}, "QUANTIZE",
        slot.quantize == 0      ? "Immediate"
        : slot.quantize == 0.25 ? "1/16"
        : slot.quantize == 1    ? "Beat"
                                : "Bar",
        "quantize");
  input(g, {248, 452, 350, 505}, "OUTPUT",
#ifdef APP_API
        "Stereo", "bus");
#else
        slot.bus ? "Bus " + std::to_string(slot.bus) : "Master", "bus");
#endif
  input(g, {360, 452, 462, 505}, "CHOKE",
        slot.choke ? std::to_string(slot.choke) : "Off", "choke");
  const char *names[] = {"GAIN", "PAN", "PITCH", "HIGH PASS", "LOW PASS"};
  for (int n = 0; n < 5; ++n)
    input(g, {float(472 + n * 126), 452, float(588 + n * 126), 505}, names[n],
          number(p.GetParam(p.selected * 8 + n + 1)->Value(), n > 2 ? 0 : 2),
          "param" + std::to_string(n + 1));
  input(g, {24, 516, 154, 568}, "LOOP START / SEC",
        slot.edited
            ? number(p.GetParam(p.selected * 8 + Start)->Value() *
                         slot.edited->samples.size() / slot.edited->rate,
                     3)
            : "0",
        "startsec");
  input(g, {164, 516, 294, 568}, "LOOP END / SEC",
        slot.edited
            ? number(p.GetParam(p.selected * 8 + End)->Value() *
                         slot.edited->samples.size() / slot.edited->rate,
                     3)
            : "0",
        "endsec");
  input(g, {304, 516, 394, 568}, "FADE / MS", number(p.crossfade, 1), "fade");
  button(g, {404, 536, 488, 568}, "Zero check", "zero");
  button(g, {498, 536, 582, 568}, "Trim / fade", "trim");
  button(g, {592, 536, 676, 568}, "Normalize", "normalize");
  button(g, {686, 536, 770, 568}, "De-hiss", "denoise");
  button(g, {780, 536, 837, 568}, "Undo", "undo");
  button(g, {847, 536, 904, 568}, "Redo", "redo");
  button(g, {914, 536, 1096, 568},
         p.exportPath.empty() ? "Render WAV" : "Drag WAV → REAPER", "export",
         !p.exportPath.empty());
  label(g,
        "Noise profile: " + number(p.noiseStart, 3) + "–" +
            number(p.noiseEnd, 3) + " s · drag waveform with Shift",
        {24, 571, 610, 593}, 10, Muted);
  button(g, {480, 574, 630, 596}, "Reduction " + number(p.noiseDb, 1) + " dB",
         "noisedb");
  button(g, {640, 574, 750, 596}, "Zoom +", "zoomin");
  button(g, {760, 574, 870, 596}, "Zoom −", "zoomout");
  button(g, {880, 574, 1096, 596}, "Save WAV as…", "savewav");
  g.DrawLine(Line, 24, 606, 1096, 606);
  input(g, {24, 616, 154, 669}, "INSTRUMENT", p.instrument + "  ▾",
        "instrument");
  input(g, {164, 616, 294, 669}, "KEY", p.key, "key");
  input(g, {304, 616, 394, 669}, "BPM", number(p.bpm, 1), "bpm");
  input(g, {404, 616, 494, 669}, "LENGTH / BARS", number(p.beats / p.barBeats),
        "bars");
  input(g, {504, 616, 584, 669}, "STEPS", std::to_string(p.steps), "steps");
  input(g, {594, 616, 674, 669}, "VARIANTS", std::to_string(p.variants),
        "variants");
  input(g, {684, 616, 804, 669}, "SEED", std::to_string(p.seed), "seed");
  button(g, {814, 636, 934, 669}, "Fit loop", "fit");
  button(g, {944, 636, 1096, 669}, p.busy ? "Cancel job" : "Generate stem",
         p.busy ? "cancel" : "generate", true);
  input(g, {24, 678, 735, 731}, "DESCRIPTION", p.prompt, "prompt");
  button(g, {745, 698, 842, 731}, "Variant " + std::to_string(p.variant + 1),
         "variant");
  button(g, {852, 698, 969, 731}, "Audition take", "candidate");
  button(g, {979, 698, 1096, 731}, "Accept take", "accept");
  label(g, p.status, {24, 742, 1096, 770}, 12, p.busy ? Gold : Muted);
}
void LoopEditor::navigation(IGraphics &g) {
  button(g, {350, 23, 430, 58}, "PLAY", "view0", view == 0);
  button(g, {440, 23, 540, 58}, "GENERATE", "view2", view == 2);
  button(g, {550, 23, 630, 58}, "EDIT", "view1", view == 1);
}
void LoopEditor::keyboard(IGraphics &g) {
  label(g, "YOUR LOOP KEYS", {36, 573, 300, 600}, 10, Muted);
  label(g, "Play a key. Layer a sound. Right-click to use it as AI context.",
        {340, 573, 930, 600}, 10, Muted);
  button(g, {950, 572, 990, 599}, "<", "prevpage");
  button(g, {1000, 572, 1084, 599},
         std::to_string(p.page * 16) + "–" +
             std::to_string(std::min(127, p.page * 16 + 15)) + "  >",
         "nextpage");
  const auto black = [](int note) {
    const int pitch = note % 12;
    return pitch == 1 || pitch == 3 || pitch == 6 || pitch == 8 || pitch == 10;
  };
  const int base = p.page * 16;
  int whites = 0;
  for (int i = 0; i < 16; ++i)
    if (!black(base + i))
      ++whites;
  const float width = 1048.f / (whites + 1);
  int index = 0;
  std::vector<std::pair<IRECT, int>> dark;
  for (int i = 0; i < 16; ++i) {
    int note = base + i;
    const bool isBlack = black(note);
    IRECT r;
    if (isBlack) {
      float x = 36 + (index + 0.5f) * width;
      r = {x - width * .3f, 610, x + width * .3f, 686};
      dark.push_back({r, note});
      continue;
    }
    r = {36 + (index + .5f) * width, 610, 36 + (index + 1.5f) * width - 3, 735};
    ++index;
    bool loaded = bool(p.bank[note].edited);
    auto color = note == p.selected ? Mint
                 : loaded           ? IColor(255, 216, 216, 209)
                                    : IColor(255, 177, 177, 174);
    g.FillRoundRect(color, r, 4);
    g.DrawRoundRect(Background, r, 4);
    label(g, std::to_string(note), r.GetFromBottom(24).GetPadded(-8), 11,
          Background);
    if (loaded) {
      g.FillCircle(Background, r.MW(), 702, 3);
      label(g, p.bank[note].name.substr(0, 10), {r.L + 5, 709, r.R - 5, 727}, 8,
            Background);
    }
    if (p.context[note])
      g.FillCircle(Gold, r.R - 9, 720, 3);
    hits.push_back({r, "key" + std::to_string(note)});
  }
  for (const auto &key : dark) {
    auto r = key.first;
    int note = key.second;
    g.FillRoundRect(note == p.selected ? Mint : IColor(255, 25, 25, 29), r, 3);
    g.DrawRoundRect(Line, r, 3);
    label(g, std::to_string(note), r.GetFromBottom(25).GetPadded(-6), 10,
          note == p.selected ? Background : Muted);
    if (p.bank[note].edited)
      g.FillCircle(note == p.selected ? Background : Mint, r.MW(), 628, 3);
    if (p.context[note])
      g.FillCircle(Gold, r.R - 7, 630, 3);
    hits.push_back({r, "key" + std::to_string(note)});
  }
}
void LoopEditor::performance(IGraphics &g) {
  auto &slot = p.bank[p.selected];
  label(g, "ONE NOTE. ANOTHER POSSIBILITY.", {360, 103, 950, 125}, 10, Muted);
  label(g, slot.edited ? slot.name : "Start with a sound.",
        {330, 132, 990, 180}, 34);
  const float cx = 560, cy = 310;
  // Native vector artwork responds to the selected loop; no external assets.
  for (int ring = 0; ring < 36; ++ring) {
    float radius = 56 + ring * 2.3f;
    IColor c(255, 54 + ring, 57 + ring, 65 + ring);
    float lastX = cx + radius, lastY = cy;
    for (int n = 1; n <= 96; ++n) {
      double angle = n * 6.28318530718 / 96;
      float signal = 0;
      if (slot.edited) {
        size_t index =
            size_t(double(n % 96) / 96 * slot.edited->samples.size());
        signal = std::abs(slot.edited->samples[index][0]);
      }
      float shape =
          radius + (6 + signal * 18) * std::sin(float(angle * 3 + ring * .12));
      float x = cx + shape * std::cos(angle),
            y = cy + shape * .72f * std::sin(angle);
      g.DrawLine(c, lastX, lastY, x, y, 0, 1);
      lastX = x;
      lastY = y;
    }
  }
  g.FillCircle(Mint, cx, cy, 32);
  const double position = p.playPosition();
  if (position >= 0) {
    const double angle = position * 6.28318530718 - 1.57079632679;
    const float x = cx + 150 * std::cos(angle), y = cy + 108 * std::sin(angle);
    g.DrawLine(Mint, cx + 40 * std::cos(angle), cy + 29 * std::sin(angle), x, y,
               nullptr, 1.5f);
    g.FillCircle(White, x, y, 3);
  }
  g.FillTriangle(Background, cx - 6, cy - 11, cx - 6, cy + 11, cx + 12, cy);
  hits.push_back({{cx - 32, cy - 32, cx + 32, cy + 32}, "masterplay"});
  const int ids[] = {Gain, Lowpass, Pitch, Pan};
  const char *names[] = {"LEVEL", "TONE", "TUNE", "PAN"};
  const float positions[] = {150, 275, 845, 970};
  for (int n = 0; n < 4; ++n) {
    int param = ids[n];
    double value = p.GetParam(p.selected * 8 + param)->Value();
    double norm = p.GetParam(p.selected * 8 + param)->ToNormalized(value);
    float x = positions[n], y = 399 - float(norm) * 160;
    label(g, names[n], {x - 30, 205, x + 55, 230}, 10, Muted);
    g.FillRoundRect(Line, {x - 2, 240, x + 2, 400}, 2);
    g.FillRoundRect(Mint, {x - 2, y, x + 2, 400}, 2);
    g.FillRoundRect(White, {x - 13, y - 4, x + 13, y + 4}, 2);
    label(g,
          number(value, param == Lowpass ? 0 : 2) +
              (param == Pitch ? " st" : ""),
          {x - 35, 421, x + 65, 446}, 12);
    hits.push_back(
        {{x - 35, 235, x + 35, 411}, "param" + std::to_string(param)});
  }
  label(g,
        "NOTE " + std::to_string(p.selected) + "  /  " +
            number(slot.beats / p.barBeats) + " BARS  /  " + slot.key,
        {350, 457, 920, 480}, 11, Muted);
  button(g, {36, 505, 140, 542},
         slot.mode == Mode::Gate     ? "GATE"
         : slot.mode == Mode::Toggle ? "TOGGLE"
                                     : "ONE-SHOT",
         "mode");
  button(g, {150, 505, 265, 542},
         slot.quantize == 0      ? "FREE START"
         : slot.quantize == 0.25 ? "1/16 GRID"
         : slot.quantize == 1    ? "BEAT GRID"
                                 : "BAR GRID",
         "quantize");
  button(g, {275, 505, 390, 542},
#ifdef APP_API
         "STEREO OUT", "bus");
#else
         slot.bus ? "OUTPUT " + std::to_string(slot.bus) : "MASTER OUT", "bus");
#endif
  button(g, {705, 505, 820, 542}, "Edit loop", "view1");
  button(g, {830, 505, 945, 542}, "Import sound", "import");
  button(g, {955, 505, 1084, 542},
         p.exportPath.empty() ? "Render WAV" : "Drag WAV", "export",
         !p.exportPath.empty());
}
void LoopEditor::Draw(IGraphics &g) {
  std::lock_guard<std::recursive_mutex> lock(p.stateMutex);
  if (view == 1) {
    DrawDetail(g);
    navigation(g);
    return;
  }
  hits.clear();
  g.FillRect(Background, mRECT);
  label(g, "ALPHATESTER x KRGSH", {36, 16, 330, 36}, 10, Muted);
  label(g, "LOOPS", {36, 36, 300, 68}, 25);
  navigation(g);
  button(g, {650, 23, 740, 58}, number(GetUI()->GetDrawScale() * 100, 0) + "%",
         "scale");
  button(g, {760, 23, 860, 58}, "Open bank", "open");
  button(g, {870, 23, 970, 58}, "Save bank", "save");
  button(g, {980, 23, 1084, 58}, "Check models", "models");
  g.DrawLine(Line, 36, 82, 1084, 82);
  if (view == 0)
    performance(g);
  else
    generation(g);
  g.DrawLine(Line, 36, 558, 1084, 558);
  keyboard(g);
  if (takeDrag >= 0 && takeMoved) {
    const IRECT ghost(std::clamp(dragX, 36.f, 864.f),
                      std::clamp(dragY - 40, 90.f, 710.f),
                      std::clamp(dragX, 36.f, 864.f) + 220,
                      std::clamp(dragY - 40, 90.f, 710.f) + 32);
    g.FillRoundRect(Mint, ghost, 5);
    label(g, "Drop take on a key", ghost.GetPadded(-8), 12, Background);
  }
  label(g, p.status, {36, 746, 1084, 774}, 11, p.busy ? Gold : Muted);
}
} // namespace loops::ui
