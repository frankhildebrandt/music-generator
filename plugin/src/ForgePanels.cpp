#include "Editor.h"
namespace loops::ui {
IColor LoopEditor::groupColor(int id) const {
  static const IColor colors[]{IColor(255, 48, 139, 250),
                               Teal,
                               Gold,
                               IColor(255, 190, 126, 243),
                               IColor(255, 242, 113, 139),
                               Mint,
                               IColor(255, 244, 140, 74),
                               IColor(255, 107, 175, 230)};
  for (const auto &group : p.bank.document.at("groups"))
    if (group.value("id", 0) == id)
      return colors[std::clamp(group.value("color", 0), 0, 7)];
  return colors[0];
}
std::string LoopEditor::groupName(int id) const {
  if (!id)
    return "No group";
  for (const auto &group : p.bank.document.at("groups"))
    if (group.value("id", 0) == id)
      return group.value("name", "Group");
  return "Group " + std::to_string(id);
}
void LoopEditor::choose(const std::string &id,
                        const std::vector<std::string> &labels,
                        const std::vector<int> &values, IRECT bounds) {
  choiceMenu.Clear();
  menuAction = id;
  menuValues = values;
  for (const auto &name : labels)
    choiceMenu.AddItem(name.c_str());
  GetUI()->CreatePopupMenu(*this, choiceMenu, bounds);
}
bool LoopEditor::forgeAction(const std::string &id, IRECT bounds) {
  auto &slot = p.bank[p.selected];
  if (id == "waveView")
    coverView = false;
  else if (id == "coverView")
    coverView = true;
  else if (id == "noop") {
  } else if (id == "stopAll")
    p.stopAll();
  else if (id.rfind("page", 0) == 0)
    p.page = std::stoi(id.substr(4));
  else if (id == "global") {
    choose(id,
           {"Off", "1/16", "1/8", "1/4", "1/2", "1 bar", "2 bars", "4 bars"},
           {0, 250, 500, 1000, 2000, -1000, -2000, -4000}, bounds);
  } else if (id == "snap")
    choose(id, {"Off", "1/4", "1/8", "1/16", "1/32"}, {0, 1, 2, 4, 8}, bounds);
  else if (id == "settings")
    choose(id,
           {"Open bank…", "Save bank…", "Rename bank…", "Import audio…",
            "Check / install XL models", "UI scale",
            "Toggle host sync / internal clock", "Save rendered WAV as…"},
           {0, 1, 2, 3, 4, 5, 6, 7}, bounds);
  else if (id == "bankName")
    entry(id, p.bank.document.value("name", "Untitled bank"),
          {187, 10, 365, 39});
  else if (id == "sync") {
    p.hostSync = !p.hostSync;
    p.followHostGeneration = p.hostSync;
  } else if (id == "ideaName")
    entry(id, p.bank.document.value("idea_name", "Sound idea"), bounds);
  else if (id == "idea")
    entry(id, p.bank.document.value("idea", ""), {356, 61, 884, 91});
  else if (id == "renameSlot")
    entry(id, slot.name, bounds);
  else if (id == "templates" || id == "instrumentTemplates") {
    templateOpen = true;
    templateInstrument = id == "instrumentTemplates";
    templateSelected = 0;
  } else if (id == "templateClose")
    templateOpen = false;
  else if (id == "templateIdeas" || id == "templateInstruments") {
    templateInstrument = id == "templateInstruments";
    templateSelected = 0;
  } else if (id == "templateSearch")
    entry(id, templateSearch, bounds);
  else if (id == "saveIdea")
    p.saveTemplate(true);
  else if (id == "saveInstrument")
    p.saveTemplate(false);
  else if (id == "templateSave")
    p.saveTemplate(!templateInstrument);
  else if (id.rfind("templatePick", 0) == 0)
    templateSelected = std::stoi(id.substr(12));
  else if (id == "templateApply") {
    if (templateSelected >= 0 && templateSelected < int(p.templates.size()))
      p.applyTemplate(p.templates[templateSelected]);
    templateOpen = false;
  } else if (id == "fullPrompt")
    fullPromptOpen = !fullPromptOpen;
  else if (id == "randomSeed")
    p.seed =
        int64_t(std::chrono::steady_clock::now().time_since_epoch().count() &
                0x7fffffff);
  else if (id == "retrigger") {
    if (!slot.group) {
      slot.retrigger = !slot.retrigger;
      p.publish(p.selected);
    }
  } else if (id == "mapping")
    mappingOpen = !mappingOpen;
  else if (id.rfind("map", 0) == 0)
    choose(id,
           {"Level", "Pan", "Tune", "High pass", "Low pass / Tone",
            "Loop start", "Loop end"},
           {Gain, Pan, Pitch, Highpass, Lowpass, Start, End}, bounds);
  else if (id == "keySelect") {
    std::vector<std::string> labels;
    std::vector<int> values;
    static const char *notes[]{"C",  "C#", "D",  "D#", "E",  "F",
                               "F#", "G",  "G#", "A",  "A#", "B"};
    for (int n = 0; n < 24; ++n) {
      labels.push_back(std::string(notes[n % 12]) +
                       (n < 12 ? " minor" : " major"));
      values.push_back(n);
    }
    choose(id, labels, values, bounds);
  } else if (id == "busSelect") {
#ifdef APP_API
    p.status = "Standalone monitors all slots on stereo output";
#else
    std::vector<std::string> labels;
    std::vector<int> values;
    for (int n = 0; n < 17; ++n) {
      labels.push_back(n ? "Stereo " + std::to_string(n + 1) : "Master");
      values.push_back(n);
    }
    choose(id, labels, values, bounds);
#endif
  } else if (id == "newGroup") {
    auto &groups = p.bank.document["groups"];
    int next = 1;
    for (; next <= 16; ++next) {
      bool exists = false;
      for (const auto &g : groups)
        if (g.value("id", 0) == next)
          exists = true;
      if (!exists)
        break;
    }
    if (next > 16) {
      p.status = "All 16 sound groups are in use";
      return true;
    }
    groups.push_back({{"id", next},
                      {"name", "Sound group " + std::to_string(next)},
                      {"color", (next - 1) % 8}});
    groupSelected = slot.group = next;
    p.publish(p.selected);
    p.markChanged();
  } else if (id == "groupSelect" || id == "assignGroup") {
    std::vector<std::string> labels;
    std::vector<int> values;
    if (id == "assignGroup") {
      labels.push_back("No group");
      values.push_back(0);
    }
    for (const auto &g : p.bank.document["groups"]) {
      labels.push_back(g.value("name", "Group"));
      values.push_back(g.value("id", 0));
    }
    if (!labels.empty())
      choose(id, labels, values, bounds);
    else
      p.status = "Create a sound group in EDIT first";
  } else if (id == "groupName")
    entry(id, groupName(groupSelected), bounds);
  else if (id == "groupColor") {
    for (auto &group : p.bank.document["groups"])
      if (group.value("id", 0) == groupSelected)
        group["color"] = (group.value("color", 0) + 1) % 8;
  } else if (id == "groupAdd") {
    std::vector<std::string> labels;
    std::vector<int> values;
    for (int n = 0; n < Slots; ++n)
      if (p.bank[n].edited && p.bank[n].group != groupSelected) {
        labels.push_back(noteName(n) + " · " + p.bank[n].name);
        values.push_back(n);
      }
    if (groupSelected && !labels.empty())
      choose(id, labels, values, bounds);
  } else if (id == "groupDelete") {
    for (auto &group : p.bank.document["groups"])
      if (group.value("id", 0) == groupSelected) {
        group["deleted"] = true;
      }
    auto &groups = p.bank.document["groups"];
    groups.erase(
        std::remove_if(groups.begin(), groups.end(),
                       [](const Json &g) { return g.value("deleted", false); }),
        groups.end());
    for (int n = 0; n < Slots; ++n)
      if (p.bank[n].group == groupSelected) {
        p.bank[n].group = 0;
        p.publish(n);
      }
    groupSelected = 0;
  } else if (id.rfind("groupRemove", 0) == 0) {
    int note = std::stoi(id.substr(11));
    p.bank[note].group = 0;
    p.publish(note);
  } else if (id == "artwork") {
    WDL_String file, path;
    GetUI()->PromptForFile(file, path, EFileAction::Open, "png jpg jpeg",
                           [this](const WDL_String &f, const WDL_String &) {
                             if (f.GetLength())
                               p.importArtwork(f.Get());
                           });
  } else
    return false;
  return true;
}
bool LoopEditor::forgeEntry(const std::string &id, const std::string &value) {
  if (id == "idea")
    p.setIdea(value);
  else if (id == "ideaName" || id == "bankName")
    p.bank.document[id == "ideaName" ? "idea_name" : "name"] =
        value.substr(0, 80);
  else if (id == "renameSlot")
    p.bank[p.selected].name = value.substr(0, 80);
  else if (id == "templateSearch")
    templateSearch = value.substr(0, 80);
  else if (id == "groupName") {
    for (auto &group : p.bank.document["groups"])
      if (group.value("id", 0) == groupSelected)
        group["name"] = value.substr(0, 80);
  } else
    return false;
  p.DirtyParametersFromUI();
  return true;
}
void LoopEditor::groups(IGraphics &g) {
  g.FillRoundRect(Panel, {844, 106, 1110, 562}, 6);
  g.DrawRoundRect(Line, {844, 106, 1110, 562}, 6);
  label(g, "SOUND GROUPS", {855, 112, 1021, 141}, 14);
  button(g, {1028, 113, 1100, 139}, "+ New", "newGroup");
  if (!groupSelected && p.bank[p.selected].group)
    groupSelected = p.bank[p.selected].group;
  if (!groupSelected) {
    label(g, "Create a group, then add slots.", {859, 179, 1098, 226}, 12,
          Muted);
    return;
  }
  button(g, {856, 154, 884, 181}, "", "groupColor");
  g.FillRoundRect(groupColor(groupSelected), {860, 158, 880, 177}, 3);
  button(g, {891, 154, 1065, 181}, groupName(groupSelected), "groupSelect");
  button(g, {1071, 154, 1100, 181}, "…", "groupName");
  int members = 0, active = 0, next = -1;
  std::vector<int> notes;
  for (int n = 0; n < Slots; ++n)
    if (p.bank[n].group == groupSelected) {
      notes.push_back(n);
      ++members;
      if (p.position(n) >= 0)
        ++active;
      if (p.queued(n))
        next = n;
    }
  label(g,
        std::to_string(active) + " active · " + std::to_string(members) +
            " members",
        {890, 183, 1100, 204}, 10, Muted);
  // Group membership can span all eight pages; the list scrolls independently.
  const double maxScroll = std::max(0.0, notes.size() * 34.0 - 163);
  groupScroll = std::clamp(groupScroll, 0.0, maxScroll);
  const IRECT viewport(854, 209, 1100, 372);
  const size_t startHits = hits.size();
  g.PathClipRegion(viewport);
  for (size_t i = 0; i < notes.size(); ++i) {
    const int note = notes[i];
    const float y = 211 + float(i * 34 - groupScroll);
    if (y + 30 < viewport.T || y > viewport.B)
      continue;
    button(g, {855, y, 1071, y + 29},
           noteName(note) + "  " + p.bank[note].name.substr(0, 13),
           "slot" + std::to_string(note), p.selected == note, EAlign::Near);
    label(g,
          p.queued(note)          ? "Q"
          : p.position(note) >= 0 ? ">"
                                  : "o",
          {1049, y + 3, 1069, y + 25}, 12,
          p.queued(note)       ? Gold
          : p.selected == note ? Background
                               : Mint);
    button(g, {1076, y, 1100, y + 29}, "x",
           "groupRemove" + std::to_string(note));
  }
  g.PathClipRegion();
  for (size_t n = startHits; n < hits.size(); ++n)
    hits[n].rect = hits[n].rect.Intersect(viewport);
  button(g, {855, 379, 1100, 406}, "+ Add slots", "groupAdd");
  label(g, "GROUP RULES · LOCKED", {856, 417, 1100, 438}, 10, Muted);
  label(g, "One sound at a time", {860, 443, 1100, 463}, 11, Mint);
  label(g, "Keep musical position", {860, 466, 1100, 487}, 11, Teal);
  button(g, {855, 493, 1100, 518}, "Timing: Global · " + timing(), "global");
  label(g,
        next < 0 ? "Shorter loops wrap to their own length."
                 : "NEXT: " + noteName(next) + " · " + timing(),
        {855, 521, 1100, 540}, 9, next < 0 ? Muted : Gold);
  button(g, {855, 542, 1100, 558}, "Remove group", "groupDelete");
}
void LoopEditor::templatesPanel(IGraphics &g) {
  hits.clear();
  hits.push_back({mRECT, "noop"});
  g.FillRect(IColor(180, 6, 12, 14), mRECT);
  g.FillRoundRect(Panel, {250, 115, 1109, 561}, 7);
  g.DrawRoundRect(Line, {250, 115, 1109, 561}, 7);
  label(g, "PROMPT TEMPLATES", {267, 127, 660, 154}, 19);
  button(g, {925, 126, 1051, 154}, "Save current", "templateSave");
  button(g, {1063, 126, 1097, 154}, "x", "templateClose");
  button(g, {266, 164, 414, 192}, "Sound ideas", "templateIdeas",
         !templateInstrument);
  button(g, {425, 164, 604, 192}, "Instrument prompts", "templateInstruments",
         templateInstrument);
  button(g, {782, 164, 1096, 192},
         templateSearch.empty() ? "Search templates…" : templateSearch,
         "templateSearch");
  std::vector<int> visible;
  for (size_t n = 0; n < p.templates.size(); ++n) {
    const auto &item = p.templates[n];
    std::string name = item.value("name", "");
    std::string search = templateSearch;
    std::transform(name.begin(), name.end(), name.begin(),
                   [](unsigned char c) { return char(std::tolower(c)); });
    std::transform(search.begin(), search.end(), search.begin(),
                   [](unsigned char c) { return char(std::tolower(c)); });
    if ((item.value("kind", "idea") == "instrument") == templateInstrument &&
        (search.empty() || name.find(search) != std::string::npos))
      visible.push_back(int(n));
  }
  if (std::find(visible.begin(), visible.end(), templateSelected) ==
      visible.end())
    templateSelected = visible.empty() ? -1 : visible.front();
  templateScroll = std::clamp(templateScroll, 0.0,
                              std::max(0.0, visible.size() * 49.0 - 305));
  const IRECT list(263, 205, 459, 511);
  const size_t startHits = hits.size();
  g.PathClipRegion(list);
  for (size_t n = 0; n < visible.size(); ++n) {
    int index = visible[n];
    float y = 207 + float(n * 49 - templateScroll);
    if (y + 44 < list.T || y > list.B)
      continue;
    button(g, {265, y, 450, y + 43},
           p.templates[index].value("name", "Template"),
           "templatePick" + std::to_string(index), templateSelected == index);
  }
  g.PathClipRegion();
  for (size_t n = startHits; n < hits.size(); ++n)
    hits[n].rect = hits[n].rect.Intersect(list);
  if (templateSelected >= 0) {
    const auto &item = p.templates[templateSelected];
    label(g, item.value("name", "Template"), {480, 211, 1092, 243}, 21);
    label(g, item.value("custom", false) ? "MY TEMPLATE" : "FACTORY",
          {480, 246, 1092, 267}, 9, Teal);
    g.DrawMultiLineText(text(14), item.value("prompt", "").c_str(),
                        {481, 282, 1076, 356});
    label(g, "WITH YOUR CURRENT SETTINGS", {480, 369, 1090, 389}, 11, Muted);
    const auto combined =
        templateInstrument
            ? fullPrompt(p.bank.document, item.value("prompt", ""))
            : item.value("prompt", "") + "\nInstrument: " + p.prompt;
    g.DrawMultiLineText(text(11), combined.c_str(), {481, 395, 1076, 466});
    label(g,
          p.key + "  ·  " + number(p.bpm, 1) + " BPM  ·  " +
              number(p.beats / p.barBeats, 0) + " bars",
          {481, 480, 1090, 505}, 11, Teal);
  } else
    label(g, "No matching templates", {481, 280, 1090, 330}, 16, Muted);
  label(g, "Applies to new generations. Existing loops keep their sound idea.",
        {269, 522, 826, 547}, 10, Muted);
  button(g, {850, 521, 1096, 550},
         templateInstrument ? "Use instrument prompt" : "Use sound idea",
         "templateApply", true);
}
} // namespace loops::ui
