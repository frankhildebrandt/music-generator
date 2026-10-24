#include "Editor.h"
namespace loops::ui {
void LoopEditor::keyWave(IGraphics &g, IRECT r, int note, IColor color) {
  const auto audio = p.bank[note].edited;
  if (!audio || audio->samples.empty())
    return;
  auto &wave = keyWaves[note];
  if (wave.audio.lock() != audio) {
    wave.audio = audio;
    wave.peak = 0;
    for (size_t x = 0; x < wave.bins.size(); ++x) {
      auto &bin = wave.bins[x];
      bin.fill(0);
      const size_t begin = x * audio->samples.size() / wave.bins.size();
      const size_t end =
          std::min(audio->samples.size(),
                   std::max(begin + 1, (x + 1) * audio->samples.size() /
                                           wave.bins.size()));
      for (size_t n = begin; n < end; ++n)
        for (int ch = 0; ch < 2; ++ch) {
          bin[0] = std::min(bin[0], audio->samples[n][ch]);
          bin[1] = std::max(bin[1], audio->samples[n][ch]);
        }
      wave.peak = std::max({wave.peak, -bin[0], bin[1]});
    }
  }
  const float gain = r.H() * .45f / std::max(.001f, wave.peak);
  for (size_t n = 0; n < wave.bins.size(); ++n) {
    const float x = r.L + (n + .5f) / wave.bins.size() * r.W();
    g.DrawLine(color, x, r.MH() - wave.bins[n][1] * gain, x,
               r.MH() - wave.bins[n][0] * gain);
  }
}

void LoopEditor::keyboard(IGraphics &g) {
  g.FillRoundRect(Panel, {10, 572, 1110, 606}, 4);
  label(g, "SLOTS", {22, 578, 75, 600}, 11);
  for (int n = 0; n < 8; ++n)
    button(g, {float(81 + n * 40), 578, float(116 + n * 40), 600},
           std::to_string(n + 1), "page" + std::to_string(n), p.page == n);
  label(g,
        "MIDI " + std::to_string(p.page * 16) + "–" +
            std::to_string(p.page * 16 + 15),
        {415, 578, 530, 600}, 10, Muted);
  label(g, noteName(p.selected) + " · " + p.bank[p.selected].name,
        {542, 578, 815, 600}, 11, Mint);
  int playing = 0;
  for (int n = 0; n < Slots; ++n)
    if (p.position(n) >= 0)
      ++playing;
  label(g, std::to_string(playing) + " / 32 PLAYING", {902, 578, 1025, 600}, 10,
        Muted, EAlign::Far);
  button(g, {1030, 578, 1100, 600}, "Stop all", "stopAll");
  const auto black = [](int note) {
    int pitch = note % 12;
    return pitch == 1 || pitch == 3 || pitch == 6 || pitch == 8 || pitch == 10;
  };
  const int base = p.page * 16;
  int whites = 0;
  for (int n = 0; n < 16; ++n)
    if (!black(base + n))
      ++whites;
  const float leading = black(base) ? .5f : 0.f;
  const float trailing = black(base + 15) ? .5f : 0.f;
  const float width = 1100.f / (whites + leading + trailing);
  const IColor ivory(255, 240, 236, 225), darkInk(255, 27, 128, 112);
  // Half white keys close the piano edges when a 16-note page starts/ends
  // black. They are visual backing only; every playable hit still maps to this
  // page.
  if (leading)
    g.FillRoundRect(ivory, {10, 610, 10 + leading * width - 2, 741}, 4);
  if (trailing)
    g.FillRoundRect(ivory, {10 + (whites + leading) * width, 610, 1110, 741},
                    4);
  auto paint = [&](IRECT r, int note, bool isBlack) {
    auto &slot = p.bank[note];
    const bool selected = note == p.selected, waiting = p.queued(note),
               playing = p.position(note) >= 0;
    const auto ink = isBlack && !selected ? White : Background;
    const auto waveInk = waiting ? Gold : isBlack && !selected ? Teal : darkInk;
    g.FillRoundRect(selected  ? Mint
                    : isBlack ? IColor(255, 23, 27, 30)
                              : ivory,
                    r, 4);
    g.DrawRoundRect(waiting    ? Gold
                    : selected ? Mint
                               : Line,
                    r, 4, nullptr, waiting || selected ? 2.f : 1.f);
    if (slot.group > 0)
      g.FillRoundRect(groupColor(slot.group),
                      {r.L + 4, r.T + 5, r.R - 4, r.T + 11}, 2);
    if (slot.edited) {
      const float titleY = isBlack ? r.T + 17 : r.T + 77;
      label(g, slot.name, {r.L + 5, titleY, r.R - 5, titleY + 16},
            isBlack ? 8 : 10, ink, EAlign::Center);
      keyWave(g, {r.L + 7, titleY + 17, r.R - 7, titleY + (isBlack ? 32 : 34)},
              note, waveInk);
    }
    const float noteY = isBlack ? r.B - 24 : r.B - 22;
    label(g, noteName(note), {r.L + 21, noteY, r.R - 21, noteY + 18},
          isBlack ? 9 : 12, ink, EAlign::Center);
    if (p.context[note]) {
      const float x = r.L + 11, y = noteY + 9;
      g.DrawCircle(waveInk, x - 1, y + 2, 3, nullptr, 1.5f);
      g.DrawCircle(waveInk, x + 2, y - 2, 3, nullptr, 1.5f);
    }
    if (waiting)
      g.DrawCircle(Gold, r.R - 11, noteY + 9, 4, nullptr, 1.5f);
    else if (playing)
      g.FillTriangle(isBlack && !selected ? Mint : darkInk, r.R - 16, noteY + 4,
                     r.R - 16, noteY + 14, r.R - 7, noteY + 9);
    hits.push_back({r, "key" + std::to_string(note)});
  };
  std::vector<std::pair<IRECT, int>> dark;
  int index = 0;
  for (int n = 0; n < 16; ++n) {
    const int note = base + n;
    const float x = 10 + (index + leading) * width;
    if (black(note)) {
      dark.push_back({{x - width * .30f, 610, x + width * .30f, 683}, note});
      continue;
    }
    paint({x, 610, x + width - 2, 741}, note, false);
    ++index;
  }
  for (const auto &key : dark)
    paint(key.first, key.second, true);
  label(g, "Right-click a slot to use it as context", {16, 748, 420, 767}, 9,
        Muted);
  label(g, "Playing    ·    Queued    ·    Sound group    ·    Context",
        {756, 748, 1104, 767}, 9, Teal, EAlign::Far);
}
} // namespace loops::ui
