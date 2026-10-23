#include "Editor.h"
namespace loops::ui {
namespace {
constexpr float CardHeight = 174;
const IRECT Settings(36, 124, 422, 480), Results(454, 124, 1084, 539);
} // namespace

void LoopEditor::scrollBar(IGraphics &g, IRECT viewport, double scroll,
                           double content, const std::string &id) {
  if (content <= viewport.H())
    return;
  IRECT track(viewport.R - 8, viewport.T, viewport.R, viewport.B);
  g.FillRoundRect(Panel, track, 3);
  const float height =
      std::max(28.f, float(track.H() * viewport.H() / content));
  const float top =
      track.T + float(scroll / (content - viewport.H())) * (track.H() - height);
  g.FillRoundRect(Muted, {track.L, top, track.R, top + height}, 3);
  hits.push_back({track, id});
}

void LoopEditor::generation(IGraphics &g) {
  label(g, "GENERATION SETTINGS", {36, 93, 420, 116}, 12, Mint);
  label(g, "GENERATED LOOPS", {454, 93, 905, 116}, 12, Mint);
  label(g,
        "XL · " + std::to_string(p.takes.size()) +
            (p.takes.size() == 1 ? " take" : " takes"),
        {925, 93, 1084, 116}, 11, Muted);
  g.DrawLine(Line, 438, 94, 438, 539);
  settingsScroll = std::clamp(settingsScroll, 0.0, 460.0 - Settings.H());
  const float top = Settings.T - float(settingsScroll);
  const size_t settingsHits = hits.size();
  g.PathClipRegion(Settings);
  input(g, {36, top, 212, top + 57}, "INSTRUMENT", p.instrument + "  ▾",
        "instrument");
  input(g, {226, top, 404, top + 57}, "KEY / SCALE", p.key, "key");
  input(g, {36, top + 72, 212, top + 129}, "TEMPO / BPM", number(p.bpm, 1),
        "bpm");
  input(g, {226, top + 72, 404, top + 129}, "LENGTH / BARS",
        number(p.beats / p.barBeats), "bars");
  input(g, {36, top + 146, 404, top + 236}, "DESCRIBE THE SOUND", "", "prompt");
  g.DrawMultiLineText(text(12), p.prompt.c_str(),
                      {44, top + 172, 396, top + 230});
  input(g, {36, top + 252, 148, top + 309}, "STEPS", std::to_string(p.steps),
        "steps");
  input(g, {160, top + 252, 272, top + 309}, "VARIANTS",
        std::to_string(p.variants), "variants");
  input(g, {284, top + 252, 404, top + 309}, "SEED", std::to_string(p.seed),
        "seed");
#ifdef APP_API
  label(g, "ACE-Step XL · standalone tempo", {36, top + 327, 404, top + 352},
        11, Muted);
#else
  button(g, {36, top + 327, 210, top + 359}, "Use host tempo", "host");
  label(g, "ACE-Step XL", {226, top + 327, 404, top + 359}, 11, Muted);
#endif
  int context = 0;
  for (int note = 0; note < 128; ++note)
    if (p.context[note] && note != p.selected && p.bank[note].edited)
      ++context;
  label(g,
        std::to_string(context) + " context loops · target note " +
            std::to_string(p.selected),
        {36, top + 378, 404, top + 404}, 11, Muted);
  label(g, "Right-click keys to include them as context.",
        {36, top + 408, 404, top + 448}, 11, Muted);
  g.PathClipRegion();
  for (size_t n = settingsHits; n < hits.size(); ++n)
    hits[n].rect = hits[n].rect.Intersect(Settings);
  scrollBar(g, Settings, settingsScroll, 460, "settingsScroll");
  button(g, {36, 496, 422, 539},
         p.busy ? "Cancel generation" : "Generate loops",
         p.busy ? "cancel" : "generate", true);

  const double content =
      std::max(double(Results.H()), p.takes.size() * double(CardHeight));
  takeScroll = std::clamp(takeScroll, 0.0, content - Results.H());
  const size_t takeHits = hits.size();
  g.PathClipRegion(Results);
  if (p.takes.empty()) {
    g.FillRoundRect(Panel, Results.GetPadded(-14), 8);
    label(g, "Your next loops start here.", {485, 215, 1040, 255}, 23);
    label(g, "Generate to create a waveform slot immediately.",
          {485, 265, 1040, 290}, 12, Muted);
    label(g, "Audition · drag to a key · drag WAV into your DAW",
          {485, 300, 1040, 335}, 12, Muted);
  }
  for (size_t i = 0; i < p.takes.size(); ++i) {
    const float y = Results.T + float(i * CardHeight - takeScroll);
    if (y + CardHeight <= Results.T || y >= Results.B)
      continue;
    const auto &take = p.takes[i];
    const bool ready = !take.result.empty();
    const IRECT card(454, y, 1066, y + 162), wave(468, y + 42, 1052, y + 101);
    g.FillRoundRect(Panel, card, 7);
    g.DrawRoundRect(p.auditionTake == int(i) ? Mint : Line, card, 7);
    label(g, take.title, {468, y + 7, 810, y + 34}, 14);
    label(g, number(take.bars, 0) + " bars · " + number(take.bpm, 1) + " BPM",
          {820, y + 7, 1052, y + 34}, 11, Muted);
    g.FillRoundRect(Background, wave, 4);
    g.DrawLine(Line, wave.L, wave.MH(), wave.R, wave.MH());
    if (ready && take.result.contains("waveform")) {
      const auto &bins = take.result.at("waveform");
      for (size_t column = 0; column < bins.size(); ++column) {
        const float x = wave.L + float(column) / bins.size() * wave.W();
        g.DrawLine(Mint, x, wave.MH() - bins[column][1].get<float>() * 26, x,
                   wave.MH() - bins[column][0].get<float>() * 26);
      }
      hits.push_back({wave, "takeDrag" + std::to_string(i)});
      const double position = p.takePosition(int(i));
      if (position >= 0) {
        const float x = wave.L + float(position) * wave.W();
        g.DrawLine(White, x, wave.T, x, wave.B, nullptr, 1.5f);
        g.FillTriangle(White, x - 4, wave.T, x + 4, wave.T, x, wave.T + 5);
      }
      button(g, {468, y + 117, 560, y + 149},
             p.auditionTake == int(i) ? "Stop" : "Play",
             "takePlay" + std::to_string(i));
      button(g, {572, y + 117, 814, y + 149},
             "Use note " + std::to_string(p.selected),
             "takeAssign" + std::to_string(i));
      button(g, {826, y + 117, 1052, y + 149}, "Drag WAV → DAW",
             "takeWav" + std::to_string(i));
    } else {
      label(g, take.pending ? "Waiting for audio..." : "No audio produced",
            wave.GetPadded(-12), 12, Muted);
      const IRECT progress(468, y + 105, 1052, y + 109);
      g.FillRect(Line, progress);
      if (take.progress >= 0)
        g.FillRect(Mint,
                   progress.GetFromLeft(float(
                       std::clamp(take.progress, 0.0, 1.0) * progress.W())));
      else if (take.pending && p.busy &&
               take.phase != "Waiting for previous take") {
        const auto tick =
            std::chrono::duration_cast<std::chrono::milliseconds>(
                std::chrono::steady_clock::now().time_since_epoch())
                .count();
        const float left =
            progress.L + float(tick % 1400) / 1400 * (progress.W() - 100);
        g.FillRect(Mint, {left, progress.T, left + 100, progress.B});
      }
      label(g, take.phase, {468, y + 118, 960, y + 149}, 11, Gold);
      if (take.progress >= 0)
        label(g, number(take.progress * 100, 0) + "%",
              {980, y + 118, 1052, y + 149}, 12, Mint);
    }
  }
  g.PathClipRegion();
  for (size_t n = takeHits; n < hits.size(); ++n)
    hits[n].rect = hits[n].rect.Intersect(Results);
  scrollBar(g, Results, takeScroll, content, "takeScroll");
}
} // namespace loops::ui
