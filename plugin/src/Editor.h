#pragma once
#include "MusicGeneratorLoops.h"
#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>
namespace loops::ui {
using namespace iplug;
using namespace iplug::igraphics;
inline const IColor Background{255, 16, 17, 20}, Panel{255, 28, 29, 34},
    Line{255, 55, 56, 63}, White{255, 241, 239, 230}, Muted{255, 148, 148, 154},
    Mint{255, 205, 230, 138}, Gold{255, 255, 201, 112};
inline std::string number(double n, int digits = 2) {
  std::ostringstream s;
  s << std::fixed << std::setprecision(digits) << n;
  return s.str();
}
class LoopEditor final : public IControl {
  MusicGeneratorLoops &p;
  std::string field;
  int drag = -1;
  float origin = 0;
  double initial = 0;
  int held = -1;
  int view = 0;
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
  void DrawDetail(IGraphics &g);
  void navigation(IGraphics &g);
  void keyboard(IGraphics &g);
  void performance(IGraphics &g);
  void generation(IGraphics &g);
  void Draw(IGraphics &g) override;
  void OnMouseDown(float x, float y, const IMouseMod &mod) override;
  void OnMouseDrag(float x, float y, float, float,
                   const IMouseMod &mod) override;
  void OnMouseUp(float, float, const IMouseMod &) override;
  void OnMouseDblClick(float x, float y, const IMouseMod &) override;
  void OnMouseWheel(float, float, const IMouseMod &mod, float delta) override;
  void OnDrop(const char *path) override;
  void OnTextEntryCompletion(const char *value, int) override;
};
} // namespace loops::ui
