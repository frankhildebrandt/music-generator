#pragma once
#include "MusicGeneratorLoops.h"
#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>
namespace loops::ui {
using namespace iplug;
using namespace iplug::igraphics;
inline const IColor Background{255, 20, 27, 30}, Panel{255, 28, 36, 39},
    Line{255, 61, 73, 77}, White{255, 238, 242, 244}, Muted{255, 168, 184, 195},
    Mint{255, 211, 250, 116}, Gold{255, 255, 177, 43}, Teal{255, 127, 230, 205};
inline std::string number(double n, int digits = 2) {
  std::ostringstream s;
  s << std::fixed << std::setprecision(digits) << n;
  return s.str();
}
inline constexpr std::array<const char *, 12> Instruments{
    "drums",      "bass",   "keyboard",       "guitar", "synth", "strings",
    "percussion", "vocals", "backing_vocals", "fx",     "brass", "woodwinds"};
class LoopEditor final : public IControl {
  MusicGeneratorLoops &p;
  std::string field;
  int drag = -1;
  float origin = 0;
  double initial = 0;
  int held = -1;
  int view = 0;
  bool coverView = false, templateOpen = false, templateInstrument = false,
       fullPromptOpen = false;
  bool mappingOpen = false;
  int templateSelected = 0, groupSelected = 0;
  std::string templateSearch, menuAction, artKey;
  IPopupMenu choiceMenu{"Choose"};
  std::vector<int> menuValues;
  IBitmap artwork;
  const char *artPointer = nullptr;
  size_t artSize = 0;
  IRECT waveBounds{24, 188, 820, 374};
  std::shared_ptr<const Audio> waveAudio;
  std::array<std::array<float, 4>, 2048> waveBins{};
  double settingsScroll = 0, takeScroll = 0, groupScroll = 0,
         templateScroll = 0;
  int takeDrag = -1;
  float dragX = 0, dragY = 0, takeOriginX = 0, takeOriginY = 0;
  bool takeMoved = false, boundaryDrag = false;
  int scrollDrag = -1;
  IPopupMenu instrumentMenu{"Instruments"};
  struct Hit {
    IRECT rect;
    std::string action;
  };
  std::vector<Hit> hits;

public:
  IText text(float size = 13, IColor color = White) const;
  void label(IGraphics &g, const std::string &value, IRECT r, float size = 13,
             IColor color = White);
  void button(IGraphics &g, IRECT r, const std::string &title,
              const std::string &action, bool active = false);
  void input(IGraphics &g, IRECT r, const std::string &title,
             const std::string &value, const std::string &action);
  void entry(const std::string &id, const std::string &value, IRECT r);
  void importDialog();
  void bankDialog(bool save);
  explicit LoopEditor(MusicGeneratorLoops &plugin);
  void chrome(IGraphics &g);
  void soundIdea(IGraphics &g);
  void stereoWave(IGraphics &g, IRECT bounds, bool editing);
  void knobs(IGraphics &g, IRECT bounds, bool vertical);
  void groups(IGraphics &g);
  void templatesPanel(IGraphics &g);
  void choose(const std::string &id, const std::vector<std::string> &labels,
              const std::vector<int> &values, IRECT bounds);
  bool forgeAction(const std::string &id, IRECT bounds);
  bool forgeEntry(const std::string &id, const std::string &value);
  IColor groupColor(int id) const;
  std::string groupName(int id) const;
  std::string timing() const;
  void DrawDetail(IGraphics &g);
  void navigation(IGraphics &g);
  void keyboard(IGraphics &g);
  void performance(IGraphics &g);
  void generation(IGraphics &g);
  void scrollBar(IGraphics &g, IRECT viewport, double scroll, double content,
                 const std::string &id);
  void Draw(IGraphics &g) override;
  void OnMouseDown(float x, float y, const IMouseMod &mod) override;
  void OnMouseDrag(float x, float y, float, float,
                   const IMouseMod &mod) override;
  void OnMouseUp(float, float, const IMouseMod &) override;
  void OnMouseDblClick(float x, float y, const IMouseMod &) override;
  void OnMouseWheel(float, float, const IMouseMod &mod, float delta) override;
  void OnDrop(const char *path) override;
  void OnTextEntryCompletion(const char *value, int) override;
  void OnPopupMenuSelection(IPopupMenu *menu, int) override;
};
} // namespace loops::ui
