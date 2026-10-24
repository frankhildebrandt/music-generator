#include "Editor.h"
namespace loops::ui {
namespace {
constexpr float CardHeight = 132;
const IRECT Settings(22, 145, 477, 475), Results(496, 145, 1108, 540);
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
  g.FillRoundRect(Panel, {10, 106, 486, 563}, 5);
  label(g, "INSTRUMENT PROMPT", {22, 112, 265, 137}, 15);
  label(g,
        "Shared idea v" + std::to_string(p.bank.document.value("revision", 1)),
        {290, 112, 477, 137}, 10, Teal);
  label(g, "VARIANTS", {507, 111, 750, 137}, 15);
  label(g,
        "Target " + noteName(p.selected) +
            (p.bank[p.selected].edited ? " · occupied" : " · empty"),
        {770, 111, 1107, 137}, 11, Muted);
  settingsScroll = std::clamp(settingsScroll, 0.0, 520.0 - Settings.H());
  const float top = Settings.T - float(settingsScroll);
  const size_t settingsHits = hits.size();
  g.PathClipRegion(Settings);
  input(g, {22, top, 226, top + 52}, "INSTRUMENT", p.instrument + "  v",
        "instrument");
  input(g, {239, top, 460, top + 52}, "TEMPLATE", "Browse instrument prompts",
        "instrumentTemplates");
  input(g, {22, top + 63, 460, top + 139}, "INSTRUMENT DESCRIPTION", "",
        "prompt");
  g.DrawMultiLineText(text(12), p.prompt.c_str(),
                      {30, top + 89, 450, top + 134});
  label(g, "AUDIO CONTEXT", {22, top + 147, 130, top + 167}, 9, Muted);
  float contextX = 132;
  for (int note = 0; note < Slots; ++note)
    if (p.context[note] && note != p.selected && p.bank[note].edited) {
      if (contextX < 450)
        label(g, noteName(note) + " " + p.bank[note].name.substr(0, 10),
              {contextX, top + 147, contextX + 110, top + 167}, 9, Teal);
      contextX += 110;
    }
  input(g, {22, top + 178, 226, top + 230}, "KEY / SCALE", p.key + "  v",
        "keySelect");
  input(g, {239, top + 178, 460, top + 230}, "TEMPO / BPM", number(p.bpm, 1),
        "bpm");
  input(g, {22, top + 242, 226, top + 294}, "LENGTH / BARS",
        number(p.beats / p.barBeats), "bars");
  input(g, {239, top + 242, 460, top + 294}, "STEPS", std::to_string(p.steps),
        "steps");
  input(g, {22, top + 305, 169, top + 357}, "SEED", std::to_string(p.seed),
        "seed");
  button(g, {175, top + 325, 226, top + 357}, "Dice", "randomSeed");
  input(g, {239, top + 305, 460, top + 357}, "VARIANTS",
        std::to_string(p.variants), "variants");
  button(g, {22, top + 370, 460, top + 400},
         fullPromptOpen ? "Full prompt  v" : "Full prompt  >", "fullPrompt");
  if (fullPromptOpen)
    g.DrawMultiLineText(text(10), fullPrompt(p.bank.document, p.prompt).c_str(),
                        {30, top + 408, 451, top + 514});
#ifndef APP_API
  button(g, {22, top + 465, 175, top + 493}, "Use host tempo", "host");
#endif
  g.PathClipRegion();
  for (size_t n = settingsHits; n < hits.size(); ++n)
    hits[n].rect = hits[n].rect.Intersect(Settings);
  scrollBar(g, Settings, settingsScroll, 520, "settingsScroll");
  label(g, "ACE-Step XL · Local", {24, 497, 227, 533}, 11, Muted);
  button(g, {244, 491, 477, 538},
         p.busy ? "Cancel generation" : "Generate loops",
         p.busy ? "cancel" : "generate", true);
  const double content =
      std::max(double(Results.H()), p.takes.size() * double(CardHeight));
  takeScroll = std::clamp(takeScroll, 0.0, content - Results.H());
  const size_t takeHits = hits.size();
  g.PathClipRegion(Results);
  if (p.takes.empty()) {
    g.FillRoundRect(Panel, Results.GetPadded(-14), 8);
    label(g, "Your next loops start here.", {518, 225, 1080, 265}, 23);
    label(g, "Generate to create a waveform slot immediately.",
          {518, 275, 1080, 300}, 12, Muted);
    label(g, "Audition · drag to a key · drag WAV into your DAW",
          {518, 310, 1080, 345}, 12, Muted);
  }
  for (size_t i = 0; i < p.takes.size(); ++i) {
    const float y = Results.T + float(i * CardHeight - takeScroll);
    if (y + CardHeight <= Results.T || y >= Results.B)
      continue;
    const auto &take = p.takes[i];
    const bool ready = !take.result.empty();
    const IRECT card(496, y, 1094, y + 124), wave(510, y + 36, 1080, y + 80);
    g.FillRoundRect(Panel, card, 7);
    g.DrawRoundRect(p.auditionTake == int(i) ? Mint : Line, card, 7);
    label(g,
          take.title + " · v" + std::to_string(take.idea.value("revision", 1)),
          {510, y + 4, 815, y + 30}, 14);
    label(g, number(take.bars, 0) + " bars · " + number(take.bpm, 1) + " BPM",
          {825, y + 4, 1080, y + 30}, 11, Muted);
    g.FillRoundRect(Background, wave, 4);
    g.DrawLine(Line, wave.L, wave.MH(), wave.R, wave.MH());
    if (ready && take.result.contains("waveform")) {
      const auto &bins = take.result.at("waveform");
      for (size_t column = 0; column < bins.size(); ++column) {
        const float x = wave.L + float(column) / bins.size() * wave.W();
        g.DrawLine(Mint, x, wave.MH() - bins[column][1].get<float>() * 19, x,
                   wave.MH() - bins[column][0].get<float>() * 19);
      }
      hits.push_back({wave, "takeDrag" + std::to_string(i)});
      const double position = p.takePosition(int(i));
      if (position >= 0) {
        const float x = wave.L + float(position) * wave.W();
        g.DrawLine(White, x, wave.T, x, wave.B, nullptr, 1.5f);
        g.FillTriangle(White, x - 4, wave.T, x + 4, wave.T, x, wave.T + 5);
      }
      button(g, {510, y + 88, 580, y + 115},
             p.auditionTake == int(i) ? "Stop" : "Play",
             "takePlay" + std::to_string(i));
      button(g, {590, y + 88, 861, y + 115},
             "Replace " + noteName(p.selected) + "…",
             "takeAssign" + std::to_string(i));
      button(g, {871, y + 88, 1080, y + 115}, "WAV",
             "takeWav" + std::to_string(i));
    } else {
      label(g, take.pending ? "Waiting for audio..." : "No audio produced",
            wave.GetPadded(-12), 12, Muted);
      const IRECT progress(510, y + 84, 1080, y + 87);
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
      label(g, take.phase, {510, y + 92, 977, y + 115}, 11, Gold);
      if (take.progress >= 0)
        label(g, number(take.progress * 100, 0) + "%",
              {998, y + 92, 1080, y + 115}, 12, Mint);
    }
  }
  g.PathClipRegion();
  for (size_t n = takeHits; n < hits.size(); ++n)
    hits[n].rect = hits[n].rect.Intersect(Results);
  scrollBar(g, Results, takeScroll, content, "takeScroll");
}
} // namespace loops::ui
