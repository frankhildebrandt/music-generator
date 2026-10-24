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
    label(g, titles[v], rect, 14, view == v ? White : Muted, EAlign::Center);
    hits.push_back({rect, "view" + std::to_string(v)});
    if (view == v)
      g.FillRoundRect(Mint, {rect.L + 8, 39, rect.R - 8, 42}, 2);
  }
}
void LoopEditor::chrome(IGraphics &g) {
  label(g, "LOOPFORGE", {20, 4, 180, 27}, 22);
  label(g, "LOOP INSTRUMENT", {21, 28, 180, 43}, 9, Muted);
  button(g, {187, 10, 365, 39},
         p.bank.document.value("name", "Untitled bank") + "  v", "settings",
         false, EAlign::Near);
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
         "ideaName", false, EAlign::Near);
  const auto idea = p.bank.document.value("idea", "");
  label(g, idea.empty() ? "A shared direction for every new instrument…" : idea,
        {356, 62, 877, 90}, 12, idea.empty() ? Muted : White);
  hits.push_back({{350, 60, 885, 92}, "idea"});
  button(g, {894, 62, 986, 89}, "Templates", "templates");
  button(g, {994, 62, 1099, 89}, view == 2 ? "Save template" : "Edit idea",
         view == 2 ? "saveIdea" : "idea");
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
  label(g, "Alphatester x KRGSH", {980, 768, 1110, 780}, 8, Muted, EAlign::Far);
  if (takeDrag >= 0 && takeMoved) {
    IRECT ghost(dragX, dragY - 30, dragX + 150, dragY);
    g.FillRoundRect(Mint, ghost, 4);
    label(g, "Drop take on a key", ghost.GetPadded(-6), 11, Background);
  }
  if (templateOpen)
    templatesPanel(g);
}
} // namespace loops::ui
