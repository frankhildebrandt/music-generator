#include "Editor.h"
namespace loops::ui {
namespace {
int keyPitch(const std::string &key) {
  if (key.empty())
    return -1;
  const auto note =
      std::string("C D EF G A B").find(char(std::toupper(key[0])));
  if (note == std::string::npos || key[0] == ' ')
    return -1;
  int pitch = int(note);
  if (key.size() > 1 && key[1] == '#')
    ++pitch;
  if (key.size() > 1 && key[1] == 'b')
    --pitch;
  return (pitch + 12) % 12;
}
} // namespace

IText LoopEditor::text(float size, IColor color) const {
  return IText(size, color, "Roboto", EAlign::Near, EVAlign::Middle);
}
void LoopEditor::label(IGraphics &g, const std::string &value, IRECT r,
                       float size, IColor color) {
  g.DrawText(text(size, color), value.c_str(), r);
}
void LoopEditor::button(IGraphics &g, IRECT r, const std::string &title,
                        const std::string &action, bool active) {
  g.FillRoundRect(active ? Mint : Panel, r, 4);
  g.DrawRoundRect(active ? Mint : Line, r, 5);
  label(g, title, r.GetPadded(-8), 12, active ? Background : White);
  hits.push_back({r, action});
}
void LoopEditor::input(IGraphics &g, IRECT r, const std::string &title,
                       const std::string &value, const std::string &action) {
  label(g, title, r.GetFromTop(18), 10, Muted);
  button(g, r.GetReducedFromTop(20), value, action);
}
void LoopEditor::entry(const std::string &id, const std::string &value,
                       IRECT r) {
  field = id;
  GetUI()->CreateTextEntry(*this, text(14), r, value.c_str());
}
void LoopEditor::importDialog() {
  WDL_String file, path;
  GetUI()->PromptForFile(file, path, EFileAction::Open, "wav mp3 zip",
                         [this](const WDL_String &f, const WDL_String &) {
                           if (f.GetLength())
                             p.importFile(f.Get());
                         });
}
void LoopEditor::bankDialog(bool save) {
  WDL_String file, path;
  GetUI()->PromptForFile(
      file, path, save ? EFileAction::Save : EFileAction::Open, "mgloops",
      [this, save](const WDL_String &f, const WDL_String &) {
        if (f.GetLength()) {
          if (save)
            p.saveBankFile(f.Get());
          else
            p.openBankFile(f.Get());
        }
      });
}
LoopEditor::LoopEditor(MusicGeneratorLoops &plugin)
    : IControl(IRECT(0, 0, 1120, 780)), p(plugin) {
  for (const char *instrument : Instruments)
    instrumentMenu.AddItem(instrument);
}
void LoopEditor::OnMouseDown(float x, float y, const IMouseMod &mod) {
  std::lock_guard<std::recursive_mutex> lock(p.stateMutex);
  for (auto it = hits.rbegin(); it != hits.rend(); ++it) {
    if (!it->rect.Contains(x, y))
      continue;
    auto id = it->action;
    if (forgeAction(id, it->rect)) {
      SetDirty(false);
      return;
    }
    if (id == "settingsScroll" || id == "takeScroll") {
      scrollDrag = id == "settingsScroll" ? 0 : 1;
      origin = y;
      initial = scrollDrag == 0 ? settingsScroll : takeScroll;
      SetDirty(false);
      return;
    }
    if (id.rfind("takeDrag", 0) == 0) {
      takeDrag = std::stoi(id.substr(8));
      takeOriginX = dragX = x;
      takeOriginY = dragY = y;
      takeMoved = false;
      return;
    }
    if (id.rfind("takePlay", 0) == 0) {
      p.previewTake(std::stoi(id.substr(8)));
      SetDirty(false);
      return;
    }
    if (id.rfind("takeAssign", 0) == 0) {
      p.assignTake(std::stoi(id.substr(10)), p.selected);
      SetDirty(false);
      return;
    }
    if (id.rfind("takeWav", 0) == 0) {
      const auto &result = p.takes.at(std::stoi(id.substr(7))).result;
      GetUI()->InitiateExternalFileDragDrop(
          result.at("export_path").get<std::string>().c_str(), it->rect);
      return;
    }
    if (id.rfind("view", 0) == 0) {
      view = std::stoi(id.substr(4));
      SetDirty(false);
      return;
    }
    if (id.size() > 3 && id.rfind("key", 0) == 0) {
      int note = std::stoi(id.substr(3));
      if (mod.R) {
        p.context[note] = !p.context[note];
      } else {
        p.selected = note;
        p.key = p.bank[note].key;
        p.beats = p.bank[note].beats;
        if (!mod.S) {
          p.trigger(note, true);
          held = note;
        }
      }
      SetDirty(false);
      return;
    }
    if (id == "scale") {
      const float current = GetUI()->GetDrawScale();
      GetUI()->Resize(PLUG_WIDTH, PLUG_HEIGHT,
                      current > 0.9f   ? 0.75f
                      : current > 0.6f ? 0.5f
                                       : 1.f);
      return;
    }
    if (id == "masterplay") {
      double active =
          p.position(p.selected) >= 0 || p.queued(p.selected) ? 1 : 0;
      p.parameter(Active, active > 0.5 ? 0 : 1);
      SetDirty(false);
      return;
    }
    if (id.rfind("slot", 0) == 0) {
      int n = std::stoi(id.substr(4));
      if (mod.R)
        p.context[n] = !p.context[n];
      else {
        p.selected = n;
        p.bpm = p.bank[n].bpm;
        p.beats = p.bank[n].beats;
        p.key = p.bank[n].key;
        p.zoom = 1;
        p.viewStart = 0;
      }
      break;
    }
    auto &slot = p.bank[p.selected];
    if (id == "wave" && slot.edited) {
      boundaryDrag = true;
      double ratio = std::clamp(
          p.viewStart + (x - waveBounds.L) / waveBounds.W() / p.zoom, 0.0, 1.0);
      if (mod.S) {
        p.noiseStart = ratio * slot.edited->samples.size() / slot.edited->rate;
        p.noiseEnd = p.noiseStart;
        drag = 100;
      } else {
        double a = p.GetParam(p.selected * 8 + Start)->Value(),
               b = p.GetParam(p.selected * 8 + End)->Value();
        drag = std::abs(ratio - a) < std::abs(ratio - b) ? Start : End;
        p.parameter(drag, ratio);
      }
      break;
    }
    if (id == "import")
      importDialog();
    else if (id == "open")
      bankDialog(false);
    else if (id == "save")
      bankDialog(true);
    else if (id == "models") {
      p.install();
    } else if (id == "prevpage")
      p.page = (p.page + 7) % 8;
    else if (id == "nextpage")
      p.page = (p.page + 1) % 8;
    else if (id == "play") {
      p.audition();
      held = p.selected;
    } else if (id == "stop") {
      p.stopPreview();
      p.parameter(Active, 0);
      p.publish(p.selected);
    } else if (id == "original") {
      p.audition(true);
      held = p.selected;
    } else if (id == "edited") {
      p.audition(false);
      held = p.selected;
    } else if (id == "mode") {
      slot.mode = Mode((int(slot.mode) + 1) % 3);
      p.publish(p.selected);
    } else if (id == "quantize") {
      slot.quantize = slot.quantize == 0      ? 0.25
                      : slot.quantize == 0.25 ? 1
                      : slot.quantize == 1    ? p.barBeats
                                              : 0;
      p.publish(p.selected);
    } else if (id == "bus") {
#ifndef APP_API
      slot.bus = (slot.bus + 1) % 17;
      p.publish(p.selected);
#endif
    } else if (id == "choke") {
      slot.choke = (slot.choke + 1) % 17;
      p.publish(p.selected);
    } else if (id.rfind("param", 0) == 0) {
      boundaryDrag = false;
      drag = std::stoi(id.substr(5));
      origin = y;
      initial = p.GetParam(p.selected * 8 + drag)->Value();
    } else if (id == "startsec" || id == "endsec") {
      int param = id == "startsec" ? Start : End;
      double duration =
          slot.edited ? slot.edited->samples.size() / slot.edited->rate : 1;
      entry(id,
            number(p.GetParam(p.selected * 8 + param)->Value() * duration, 3),
            it->rect);
    } else if (id == "zero")
      p.zero();
    else if (id == "trim")
      p.edit();
    else if (id == "normalize")
      p.edit(true);
    else if (id == "fit")
      p.edit(false, false, true);
    else if (id == "denoise")
      p.edit(false, true);
    else if (id == "undo")
      p.undo();
    else if (id == "redo")
      p.undo(true);
    else if (id == "export") {
      if (p.exportPath.empty())
        p.exportWav();
      else
        GetUI()->InitiateExternalFileDragDrop(p.exportPath.c_str(), it->rect);
    } else if (id == "savewav") {
      if (p.exportPath.empty())
        p.exportWav();
      else {
        WDL_String file, path;
        auto source = p.exportPath;
        GetUI()->PromptForFile(
            file, path, EFileAction::Save, "wav",
            [this, source](const WDL_String &f, const WDL_String &) {
              if (f.GetLength()) {
                try {
                  std::filesystem::copy_file(
                      std::filesystem::u8path(source),
                      std::filesystem::u8path(f.Get()),
                      std::filesystem::copy_options::overwrite_existing);
                } catch (const std::exception &e) {
                  p.status = e.what();
                }
              }
            });
      }
    } else if (id == "zoomin")
      p.zoom = std::min(64.0, p.zoom * 2);
    else if (id == "zoomout") {
      p.zoom = std::max(1.0, p.zoom / 2);
      p.viewStart = std::min(p.viewStart, 1 - 1 / p.zoom);
    } else if (id == "instrument") {
      const auto selected =
          std::find(Instruments.begin(), Instruments.end(), p.instrument);
      if (selected != Instruments.end())
        instrumentMenu.CheckItemAlone(int(selected - Instruments.begin()));
      GetUI()->CreatePopupMenu(*this, instrumentMenu, it->rect);
    } else if (id == "key")
      entry(id, p.key, it->rect);
    else if (id == "prompt")
      entry(id, p.prompt, it->rect);
    else if (id == "noisedb")
      entry(id, number(p.noiseDb), it->rect);
    else if (id == "fade")
      entry(id, number(p.crossfade), it->rect);
    else if (id == "bpm")
      entry(id, number(p.bpm), it->rect);
    else if (id == "bars")
      entry(id, number(p.beats / p.barBeats), it->rect);
    else if (id == "steps")
      entry(id, std::to_string(p.steps), it->rect);
    else if (id == "variants")
      entry(id, std::to_string(p.variants), it->rect);
    else if (id == "seed")
      entry(id, std::to_string(p.seed), it->rect);
    else if (id == "host") {
      p.followHostGeneration = true;
      p.bpm = p.hostBpm;
    } else if (id == "generate") {
      const auto previous = p.takes.size();
      p.generate();
      if (p.takes.size() > previous)
        takeScroll = previous * 132.0;
    } else if (id == "cancel")
      p.cancel();
    else if (id == "variant" && !p.candidates.empty())
      p.variant = (p.variant + 1) % p.candidates.size();
    else if (id == "accept")
      p.accept();
    else if (id == "candidate")
      p.previewCandidate();
    break;
  }
  SetDirty(false);
}
void LoopEditor::OnMouseDrag(float x, float y, float, float,
                             const IMouseMod &mod) {
  if (scrollDrag >= 0) {
    if (scrollDrag == 0)
      settingsScroll =
          std::clamp(initial + (y - origin) * 520.0 / 330.0, 0.0, 190.0);
    else {
      const double content = std::max(395.0, p.takes.size() * 132.0);
      takeScroll = std::clamp(initial + (y - origin) * content / 395.0, 0.0,
                              content - 395.0);
    }
    SetDirty(false);
    return;
  }
  if (takeDrag >= 0) {
    dragX = x;
    dragY = y;
    takeMoved = takeMoved || std::hypot(x - takeOriginX, y - takeOriginY) > 4;
    if (takeMoved && !mRECT.Contains(x, y)) {
      const auto path =
          p.takes.at(takeDrag).result.at("export_path").get<std::string>();
      takeDrag = -1;
      GetUI()->InitiateExternalFileDragDrop(path.c_str(),
                                            IRECT(x - 8, y - 8, x + 8, y + 8));
    }
    SetDirty(false);
    return;
  }
  if (drag < 0)
    return;
  auto &slot = p.bank[p.selected];
  if (drag == 100 && slot.edited) {
    double ratio = std::clamp(
        p.viewStart + (x - waveBounds.L) / waveBounds.W() / p.zoom, 0.0, 1.0);
    p.noiseEnd = ratio * slot.edited->samples.size() / slot.edited->rate;
  } else if (boundaryDrag && (drag == Start || drag == End)) {
    double ratio = std::clamp(
        p.viewStart + (x - waveBounds.L) / waveBounds.W() / p.zoom, 0.0, 1.0);
    if (!mod.C && slot.edited && p.beats > 0 && p.snap > 0)
      ratio = std::round(ratio * slot.beats * p.snap) / (slot.beats * p.snap);
    p.parameter(drag, ratio);
  } else {
    double scale = drag == Gain                   ? 0.01
                   : drag == Pan                  ? 0.01
                   : drag == Pitch                ? 0.1
                   : drag == Start || drag == End ? .002
                                                  : 100;
    p.parameter(drag, initial + (origin - y) * scale);
  }
  SetDirty(false);
}
void LoopEditor::OnMouseUp(float x, float y, const IMouseMod &) {
  scrollDrag = -1;
  if (takeDrag >= 0) {
    if (!takeMoved)
      p.previewTake(takeDrag);
    else
      for (auto it = hits.rbegin(); it != hits.rend(); ++it)
        if (it->action.rfind("key", 0) == 0 && it->rect.Contains(x, y)) {
          p.assignTake(takeDrag, std::stoi(it->action.substr(3)));
          break;
        }
    takeDrag = -1;
    SetDirty(false);
  }
  drag = -1;
  if (held >= 0) {
    p.trigger(held, false);
    held = -1;
  }
}
void LoopEditor::OnMouseDblClick(float x, float y, const IMouseMod &) {
  for (const auto &h : hits)
    if (h.rect.Contains(x, y) && h.action.rfind("slot", 0) == 0) {
      held = std::stoi(h.action.substr(4));
      p.trigger(held, true);
      break;
    }
}
void LoopEditor::OnMouseWheel(float x, float y, const IMouseMod &mod,
                              float delta) {
  if (templateOpen) {
    templateScroll = std::max(0.0, templateScroll - delta * 35.0);
    SetDirty(false);
    return;
  }
  if (view == 1 && x > 844 && y > 209 && y < 373) {
    groupScroll = std::max(0.0, groupScroll - delta * 35.0);
    SetDirty(false);
    return;
  }
  if (view == 2) {
    if (y >= 145 && y <= 540) {
      if (x < 486)
        settingsScroll = std::clamp(settingsScroll - delta * 40.0, 0.0, 190.0);
      else
        takeScroll = std::clamp(takeScroll - delta * 40.0, 0.0,
                                std::max(0.0, p.takes.size() * 132.0 - 395));
    }
    SetDirty(false);
    return;
  }
  if (mod.S)
    p.viewStart = std::clamp(p.viewStart + double(delta) * 0.025 / p.zoom, 0.0,
                             1 - 1 / p.zoom);
  else
    p.zoom = std::clamp(p.zoom * std::pow(1.2, double(delta)), 1.0, 64.0);
  SetDirty(false);
}
void LoopEditor::OnDrop(const char *path) { p.importFile(path); }
void LoopEditor::OnPopupMenuSelection(IPopupMenu *menu, int) {
  std::lock_guard<std::recursive_mutex> stateLock(p.stateMutex);
  if (menu == &choiceMenu && menu->GetChosenItemIdx() >= 0) {
    const int chosen = menuValues.at(menu->GetChosenItemIdx());
    auto &slot = p.bank[p.selected];
    if (menuAction == "global") {
      p.bank.document["quantize"] = chosen / 1000.0;
      p.republish();
    } else if (menuAction == "snap")
      p.snap = chosen;
    else if (menuAction == "assignGroup") {
      slot.group = chosen;
      p.publish(p.selected);
      groupSelected = chosen;
    } else if (menuAction == "groupSelect")
      groupSelected = chosen;
    else if (menuAction == "groupAdd") {
      p.bank[chosen].group = groupSelected;
      p.publish(chosen);
    } else if (menuAction == "busSelect") {
      slot.bus = chosen;
      p.publish(p.selected);
    } else if (menuAction == "keySelect") {
      static const char *roots[]{"C",  "C#", "D",  "D#", "E",  "F",
                                 "F#", "G",  "G#", "A",  "A#", "B"};
      p.key =
          std::string(roots[chosen % 12]) + (chosen < 12 ? " minor" : " major");
    } else if (menuAction.rfind("map", 0) == 0)
      slot.macros.at(std::stoi(menuAction.substr(3))) = chosen;
    else if (menuAction == "settings") {
      static const char *actions[]{"open",   "save",  "bankName", "import",
                                   "models", "scale", "sync"};
      const auto action = std::string(actions[chosen]);
      if (!forgeAction(action, {170, 10, 350, 38})) {
        if (action == "open" || action == "save")
          bankDialog(action == "save");
        else if (action == "import")
          importDialog();
        else if (action == "models")
          p.install();
        else if (action == "scale")
          GetUI()->Resize(PLUG_WIDTH, PLUG_HEIGHT,
                          GetUI()->GetDrawScale() > .9 ? .75f : 1.f);
      }
    }
    SetDirty(false);
    return;
  }
  if (menu != &instrumentMenu || menu->GetChosenItemIdx() < 0 ||
      menu->GetChosenItemIdx() >= int(Instruments.size()))
    return;
  std::lock_guard<std::recursive_mutex> lock(p.stateMutex);
  p.instrument = Instruments[menu->GetChosenItemIdx()];
  SetDirty(false);
}
void LoopEditor::OnTextEntryCompletion(const char *value, int) {
  std::lock_guard<std::recursive_mutex> stateLock(p.stateMutex);
  try {
    std::string s = value;
    if (forgeEntry(field, s)) {
      SetDirty(false);
      return;
    }
    if (field == "key") {
      p.key = s.substr(0, 80);
      auto &slot = p.bank[p.selected];
      if (view == 1 && slot.edited) {
        const int oldRoot = keyPitch(slot.key), newRoot = keyPitch(p.key);
        const bool unknown =
            slot.metadata.contains("key") && slot.metadata["key"].is_null();
        if (unknown) {
          slot.key = p.key;
          slot.metadata["key"] = p.key;
        } else if (oldRoot >= 0 && newRoot >= 0 &&
                   (slot.key.find("minor") != std::string::npos) ==
                       (p.key.find("minor") != std::string::npos)) {
          const int delta = (newRoot - oldRoot + 18) % 12 - 6;
          p.parameter(Pitch,
                      p.GetParam(p.selected * 8 + Pitch)->Value() + delta);
          slot.key = p.key;
        } else
          p.status = "Generate a new take to change between major and minor";
      }
    } else if (field == "prompt")
      p.prompt = s.substr(0, 8000);
    else {
      double n = std::stod(s);
      if (!std::isfinite(n))
        return;
      if (field == "noisedb")
        p.noiseDb = std::clamp(n, 0.0, 30.0);
      else if (field == "fade")
        p.crossfade = std::clamp(n, 0.0, 1000.0);
      else if (field == "bpm") {
        p.followHostGeneration = false;
        p.bpm = std::clamp(n, 30.0, 300.0);
        if (view == 1) {
          auto &slot = p.bank[p.selected];
          slot.bpm = p.bpm;
          if (slot.edited)
            slot.beats = p.beats =
                slot.edited->samples.size() / slot.edited->rate * p.bpm / 60;
          p.publish(p.selected);
        }
      } else if (field == "bars") {
        p.beats = std::clamp(n * p.barBeats, 0.25, 600 * p.bpm / 60);
        if (view == 1) {
          p.bank[p.selected].beats = p.beats;
          p.publish(p.selected);
        }
      } else if (field == "steps")
        p.steps = std::clamp(int(n), 1, 100);
      else if (field == "variants")
        p.variants = std::clamp(int(n), 1, 8);
      else if (field == "seed")
        p.seed = int64_t(std::clamp(n, -1.0, 2147483647.0));
      else if (field == "startsec" || field == "endsec") {
        auto a = p.bank[p.selected].edited;
        if (a)
          p.parameter(field == "startsec" ? Start : End,
                      n * a->rate / a->samples.size());
      }
    }
  } catch (...) {
    p.status = "Enter a valid number";
  }
  SetDirty(false);
}
} // namespace loops::ui

using namespace iplug::igraphics;
void MusicGeneratorLoops::layout(IGraphics *g) {
  g->LoadFont("Roboto", "Roboto-Regular.ttf");
  g->EnableMouseOver(true);
  g->AttachControl(new loops::ui::LoopEditor(*this));
  g->AttachCornerResizer(EUIResizerMode::Scale, false);
}

void MusicGeneratorLoops::OnParentWindowResize(int width, int height) {
  if (auto *graphics = GetUI()) {
    const float platform = graphics->GetPlatformWindowScale();
    const float scale = std::min(width / platform / PLUG_WIDTH,
                                 height / platform / PLUG_HEIGHT);
    graphics->Resize(PLUG_WIDTH, PLUG_HEIGHT, scale, false);
  }
}
