#include "Editor.h"
namespace loops::ui {
std::string LoopEditor::timing() const {
  const double grid = p.bank.document.value("quantize", -1.0);
  if (grid < 0)
    return number(-grid, 0) + " bar";
  if (grid == 0)
    return "Off";
  if (grid < 1)
    return grid == .25 ? "1/16" : "1/8";
  if (grid == 1)
    return "1/4";
  if (grid == 2)
    return "1/2";
  return number(grid / p.barBeats, 0) + " bar";
}
void LoopEditor::navigation(IGraphics &g) {
  const char *titles[]{"PLAY", "EDIT", "GENERATE"};
  const int order[]{0, 2, 1};
  for (int i = 0; i < 3; ++i) {
    const int v = order[i];
    IRECT rect(407.f + i * 78, 6, 480.f + i * 78, 43);
    label(g, titles[v], rect, 14, view == v ? White : Muted);
    hits.push_back({rect, "view" + std::to_string(v)});
    if (view == v)
      g.FillRoundRect(Mint, {rect.L + 8, 39, rect.R - 8, 42}, 2);
  }
}
void LoopEditor::chrome(IGraphics &g) {
  label(g, "LOOPFORGE", {20, 4, 180, 27}, 22);
  label(g, "LOOP INSTRUMENT", {21, 28, 180, 43}, 9, Muted);
  button(g, {187, 10, 365, 39},
         p.bank.document.value("name", "Untitled bank") + "  v", "settings");
  navigation(g);
  label(g, p.hostSync ? "HOST" : "CLOCK", {680, 13, 725, 36}, 9, Muted);
  label(g,
        number(p.hostBpm, 1) + " BPM  " + std::to_string(p.numerator) + "/" +
            std::to_string(p.denominator),
        {723, 13, 850, 36}, 11);
  const double ppq = p.transportPosition();
  label(g, musicalPosition(ppq, p.barBeats, p.denominator), {853, 13, 900, 36},
        10);
  label(g, "GLOBAL", {904, 10, 950, 23}, 8, Muted);
  button(g, {951, 9, 1030, 39}, timing() + " v", "global");
  label(g, p.hostSync ? "LOCKED" : "INTERNAL", {1036, 11, 1081, 22}, 7, Teal);
  for (int ch = 0; ch < 2; ++ch) {
    g.FillRect(Line, {1037.f, 26.f + ch * 5, 1081.f, 29.f + ch * 5});
    g.FillRect(Mint, {1037.f, 26.f + ch * 5,
                      1037.f + 44 * std::clamp(p.meters[ch].load(), 0.f, 1.f),
                      29.f + ch * 5});
  }
  button(g, {1087, 10, 1110, 39}, "+", "settings");
  soundIdea(g);
}
void LoopEditor::soundIdea(IGraphics &g) {
  IRECT rect(10, 52, 1110, 99);
  g.FillRoundRect(Panel, rect, 6);
  g.DrawRoundRect(Line, rect, 6);
  label(g, "SOUND IDEA", {26, 65, 151, 86}, 11, Teal);
  g.DrawLine(Line, 151, 60, 151, 91);
  button(g, {161, 61, 341, 90},
         p.bank.document.value("idea_name", "Sound idea") + "  v" +
             std::to_string(p.bank.document.value("revision", 1)),
         "ideaName");
  const auto idea = p.bank.document.value("idea", "");
  label(g, idea.empty() ? "A shared direction for every new instrument…" : idea,
        {356, 62, 877, 90}, 12, idea.empty() ? Muted : White);
  hits.push_back({{350, 60, 885, 92}, "idea"});
  button(g, {894, 62, 986, 89}, "Templates", "templates");
  button(g, {994, 62, 1099, 89}, view == 2 ? "Save template" : "Edit idea",
         view == 2 ? "saveIdea" : "idea");
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
  label(g, std::to_string(playing) + " / 32 PLAYING", {902, 578, 1025, 600}, 9);
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
  const float width = 1100.f / (whites + (black(base) ? .5f : 0.f));
  std::vector<std::pair<IRECT, int>> dark;
  int index = 0;
  auto paint = [&](IRECT r, int note, bool isBlack) {
    auto &slot = p.bank[note];
    const bool selected = note == p.selected, waiting = p.queued(note),
               playing = p.position(note) >= 0;
    const auto ink = isBlack && !selected ? White : Background;
    g.FillRoundRect(selected  ? Mint
                    : isBlack ? IColor(255, 23, 27, 30)
                              : IColor(255, 240, 236, 225),
                    r, 4);
    g.DrawRoundRect(waiting    ? Gold
                    : selected ? Mint
                               : Line,
                    r, 4, nullptr, waiting || selected ? 2.f : 1.f);
    if (slot.group > 0)
      g.FillRoundRect(groupColor(slot.group),
                      {r.L + 4, r.T + 5, r.R - 4, r.T + 12}, 2);
    if (p.context[note])
      label(g, "LINK", {r.L + 5, r.T + 18, r.R - 4, r.T + 31}, 8,
            isBlack ? Teal : IColor(255, 27, 128, 112));
    if (slot.edited && !isBlack) {
      label(g, slot.name.substr(0, 15), {r.L + 6, r.B - 79, r.R - 4, r.B - 61},
            10, ink);
      for (int n = 0; n < 42; ++n) {
        const size_t k = size_t(n) * slot.edited->samples.size() / 42;
        const float peak =
            std::min(1.f, std::abs(slot.edited->samples[k][0])) * 18;
        const float x = r.L + 7 + (r.W() - 14) * n / 42;
        g.DrawLine(waiting ? Gold : Teal, x, r.B - 44 - peak, x,
                   r.B - 44 + peak);
      }
    }
    label(g, noteName(note), {r.L + 6, r.B - 28, r.R - 4, r.B - 10},
          isBlack ? 10 : 12, ink);
    if (waiting)
      g.DrawCircle(Gold, r.R - 12, r.B - 18, 5, nullptr, 1.5f);
    else if (playing)
      g.FillTriangle(isBlack ? Mint : IColor(255, 24, 151, 73), r.R - 18,
                     r.B - 24, r.R - 18, r.B - 12, r.R - 8, r.B - 18);
    hits.push_back({r, "key" + std::to_string(note)});
  };
  for (int n = 0; n < 16; ++n) {
    int note = base + n;
    float x = 10 + (index + (black(base) ? .5f : 0.f)) * width;
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
        {756, 748, 1104, 767}, 9, Teal);
}
void LoopEditor::Draw(IGraphics &g) {
  std::lock_guard<std::recursive_mutex> lock(p.stateMutex);
  hits.clear();
  g.FillRect(Background, mRECT);
  chrome(g);
  if (view == 0)
    performance(g);
  else if (view == 1)
    DrawDetail(g);
  else
    generation(g);
  keyboard(g);
  label(g, p.status, {16, 768, 970, 780}, 8, p.busy ? Gold : Muted);
  label(g, "Alphatester x KRGSH", {980, 768, 1110, 780}, 8, Muted);
  if (takeDrag >= 0 && takeMoved) {
    IRECT ghost(dragX, dragY - 30, dragX + 150, dragY);
    g.FillRoundRect(Mint, ghost, 4);
    label(g, "Drop take on a key", ghost.GetPadded(-6), 11, Background);
  }
  if (templateOpen)
    templatesPanel(g);
}
} // namespace loops::ui
