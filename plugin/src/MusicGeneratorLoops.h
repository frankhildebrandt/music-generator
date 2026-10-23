#pragma once
#include "Bank.h"
#include "IControls.h"
#include "IPlug_include_in_plug_hdr.h"
#include "Service.h"
#include <condition_variable>
#include <deque>
#include <functional>
#include <mutex>
#include <thread>
class MusicGeneratorLoops final : public iplug::Plugin {
public:
  explicit MusicGeneratorLoops(const iplug::InstanceInfo &info);
  ~MusicGeneratorLoops() override;
  void ProcessBlock(iplug::sample **, iplug::sample **, int) override;
  void ProcessMidiMsg(const iplug::IMidiMsg &) override;
  void OnReset() override;
  void OnParentWindowResize(int width, int height) override;
  void GetBusName(iplug::ERoute, int, int, WDL_String &) const override;
  void OnParamChange(int, iplug::EParamSource, int = -1) override;
  void OnIdle() override;
  bool SerializeState(iplug::IByteChunk &) const override;
  int UnserializeState(const iplug::IByteChunk &, int) override;
  void OnRestoreState() override;
  void parameter(int param, double value);
  void trigger(int note, bool on);
  void publish(int slot);
  void generate();
  void install();
  void cancel();
  void importFile(const std::string &path);
  void edit(bool normalize = false, bool noise = false, bool fit = false);
  void zero();
  void undo(bool redo = false);
  void accept();
  void previewCandidate();
  void previewTake(int index);
  void assignTake(int index, int note);
  double takePosition(int index) const;
  double playPosition() const { return player.position(selected); }
  void stopPreview();
  void audition(bool original = false);
  void exportWav();
  void saveBankFile(const std::string &);
  void openBankFile(const std::string &);
  void layout(iplug::igraphics::IGraphics *);
  loops::Bank bank;
  int selected = 36, page = 2, variant = 0, previewSlot = -1;
  std::string prompt = "Dry, repeating groove. Isolated instrument only.",
              instrument = "drums", key = "A minor",
              status = "Import a loop or generate a new instrument.", models;
  double bpm = 120, beats = 16, crossfade = 5, noiseDb = 6, noiseStart = 0,
         noiseEnd = 0;
  int steps = 50, variants = 1;
  int64_t seed = -1;
  bool followHostGeneration = true;
  bool context[128]{}, busy = false, compareOriginal = false;
  double zoom = 1, viewStart = 0;
  double hostRate = 48000, hostBpm = 120, barBeats = 4;
  int numerator = 4, denominator = 4;
  std::string exportPath;
  loops::Json candidates = loops::Json::array();
  struct GeneratedTake {
    loops::Json result = loops::Json::object();
    std::string title, phase = "Queued";
    double progress = -1, bpm = 120, bars = 4;
    bool pending = true;
  };
  std::vector<GeneratedTake> takes;
  int auditionTake = -1;
  mutable std::recursive_mutex stateMutex;

private:
  loops::Player player;
  loops::Service service;
  std::atomic<double> audioBpm{120}, audioRate{48000}, audioBarBeats{4};
  std::atomic<int> audioNumerator{4}, audioDenominator{4};
  std::thread worker;
  std::mutex workMutex;
  std::condition_variable wake;
  bool ending = false;
  using Task = std::function<loops::Json()>;
  std::deque<Task> work;
  std::deque<loops::Json> replies;
  std::string job;
  int jobSlot = -1;
  bool pollPending = false;
  bool generationJob = false, cancelRequested = false;
  size_t generationStart = 0;
  loops::Json takeCache = loops::Json::object();
  std::chrono::steady_clock::time_point lastPoll{};
  struct History {
    int slot;
    loops::Slot value;
  };
  std::vector<History> history, future;
  void enqueue(Task task);
  void processReply(const loops::Json &reply);
  void remember(int slot);
  void loadResult(int slot, const loops::Json &result);
  void changed();
  loops::Json prepareTake(const loops::Json &, double rate, double tempo);
  void auditionResult(const loops::Json &, int slot);
  void updateTakes(const loops::Json &);
};
