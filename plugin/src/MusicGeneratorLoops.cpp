#include "MusicGeneratorLoops.h"
#include "IPlug_include_in_plug_src.h"
using namespace iplug;
using namespace igraphics;
using namespace loops;
MusicGeneratorLoops::MusicGeneratorLoops(const InstanceInfo &info)
    : iplug::Plugin(info, MakeConfig(NumParams, 1)) {
  for (int s = 0; s < Slots; ++s) {
    const std::string prefix = "Note " + std::to_string(s) + " ";
    GetParam(s * 8 + Active)->InitBool((prefix + "Play").c_str(), false);
    GetParam(s * 8 + Gain)
        ->InitDouble((prefix + "Gain").c_str(), 1, 0, 2, 0.001);
    GetParam(s * 8 + Pan)
        ->InitDouble((prefix + "Pan").c_str(), 0, -1, 1, 0.001);
    GetParam(s * 8 + Pitch)
        ->InitDouble((prefix + "Pitch").c_str(), 0, -24, 24, 0.01, "st");
    GetParam(s * 8 + Highpass)
        ->InitDouble((prefix + "Highpass").c_str(), 20, 10, 20000, 1, "Hz");
    GetParam(s * 8 + Lowpass)
        ->InitDouble((prefix + "Lowpass").c_str(), 20000, 20, 22000, 1, "Hz");
    GetParam(s * 8 + Start)
        ->InitDouble((prefix + "Loop start").c_str(), 0, 0, 0.99999, 0.00001);
    GetParam(s * 8 + End)
        ->InitDouble((prefix + "Loop end").c_str(), 1, 0.00001, 1, 0.00001);
  }

  player.prepare(48000);
  mMakeGraphicsFunc = [this]() {
    return MakeGraphics(*this, PLUG_WIDTH, PLUG_HEIGHT, PLUG_FPS,
                        GetScaleForScreen(PLUG_WIDTH, PLUG_HEIGHT));
  };
  mLayoutFunc = [this](IGraphics *g) { layout(g); };
  worker = std::thread([this]() {
    for (;;) {
      Task task;
      {
        std::unique_lock<std::mutex> lock(workMutex);
        wake.wait(lock, [this]() { return ending || !work.empty(); });
        if (ending)
          break;
        task = std::move(work.front());
        work.pop_front();
      }
      Json result;
      try {
        result = task();
      } catch (const std::exception &e) {
        result = {{"type", "error"}, {"message", e.what()}};
      }
      {
        std::lock_guard<std::mutex> lock(workMutex);
        replies.push_back(std::move(result));
      }
    }
  });
}
MusicGeneratorLoops::~MusicGeneratorLoops() {
  {
    std::lock_guard<std::mutex> lock(workMutex);
    ending = true;
    work.clear();
  }
  wake.notify_one();
  if (worker.joinable())
    worker.join();
  try {
    service.request("detach");
  } catch (...) {
  }
}
void MusicGeneratorLoops::ProcessBlock(sample **, sample **outputs,
                                       int frames) {
  double tempo = mTimeInfo.mTempo;
  audioBpm.store(std::isfinite(tempo) && tempo >= 30 && tempo <= 300
                     ? tempo
                     : audioBpm.load());
  audioRate.store(GetSampleRate());
  int num = mTimeInfo.mNumerator > 0 ? mTimeInfo.mNumerator : 4,
      den = mTimeInfo.mDenominator > 0 ? mTimeInfo.mDenominator : 4;
  double bar = num * 4.0 / den;
  audioBarBeats.store(bar);
  audioNumerator.store(num);
  audioDenominator.store(den);
  player.process(
      outputs, NOutChansConnected(), frames,
      {audioBpm.load(), mTimeInfo.mPPQPos, mTimeInfo.mTransportIsRunning, bar});
}
void MusicGeneratorLoops::ProcessMidiMsg(const IMidiMsg &msg) {
  if (msg.StatusMsg() == IMidiMsg::kNoteOn)
    player.event({msg.mOffset, msg.NoteNumber(), -1, msg.Velocity() / 127.0});
  else if (msg.StatusMsg() == IMidiMsg::kNoteOff)
    player.event({msg.mOffset, msg.NoteNumber(), -1, 0});
  else if (msg.StatusMsg() == IMidiMsg::kControlChange &&
           (msg.ControlChangeIdx() == 123 || msg.ControlChangeIdx() == 120))
    for (int n = 0; n < 128; ++n)
      player.event({msg.mOffset, n, Active, 0});
}
void MusicGeneratorLoops::OnReset() {
  player.prepare(GetSampleRate());
  SetLatency(player.latencySamples());
  for (int p = 0; p < NumParams; ++p)
    player.uiParameter(p, GetParam(p)->Value());
}
void MusicGeneratorLoops::OnParamChange(int id, EParamSource source,
                                        int offset) {
  if (id < 0 || id >= NumParams)
    return;
  double value = GetParam(id)->Value();
  if (source == EParamSource::kHost && offset >= 0)
    player.event({offset, id / 8, id % 8, value});
  else
    player.uiParameter(id, value);
}
void MusicGeneratorLoops::parameter(int p, double value) {
  const int id = selected * 8 + p;
  BeginInformHostOfParamChangeFromUI(id);
  SendParameterValueFromUI(id, GetParam(id)->ToNormalized(value));
  EndInformHostOfParamChangeFromUI(id);
  bank[selected].params[p] = GetParam(id)->Value();
  changed();
}
void MusicGeneratorLoops::trigger(int note, bool on) {
  IMidiMsg msg;
  if (on)
    msg.MakeNoteOnMsg(note, 100, 0);
  else
    msg.MakeNoteOffMsg(note, 0);
  SendMidiMsgFromUI(msg);
}
void MusicGeneratorLoops::publish(int slot) {
  player.publish(slot, std::make_unique<loops::Clip>(bank[slot].clip()));
}
void MusicGeneratorLoops::changed() {
  exportPath.clear();
  if (GetUI())
    GetUI()->SetAllControlsDirty();
  DirtyParametersFromUI();
}
bool MusicGeneratorLoops::SerializeState(IByteChunk &chunk) const {
  try {
    std::lock_guard<std::recursive_mutex> lock(stateMutex);
    auto snapshot = bank;
    for (int s = 0; s < Slots; ++s)
      for (int p = 0; p < 8; ++p)
        snapshot[s].params[p] = GetParam(s * 8 + p)->Value();
    auto bytes = saveBank(snapshot);
    int size = int(bytes.size());
    chunk.Put(&size);
    chunk.PutBytes(bytes.data(), size);
    return SerializeParams(chunk);
  } catch (...) {
    return false;
  }
}
int MusicGeneratorLoops::UnserializeState(const IByteChunk &chunk, int pos) {
  try {
    int size = 0;
    pos = chunk.Get(&size, pos);
    if (pos < 0 || size < 12 || size > 512 * 1024 * 1024 ||
        size > chunk.Size() - pos)
      return -1;
    std::vector<uint8_t> bytes(size);
    pos = chunk.GetBytes(bytes.data(), size, pos);
    auto restored = loadBank(bytes.data(), bytes.size());
    int check = pos;
    for (int id = 0; id < NumParams; ++id) {
      double value = 0;
      check = chunk.Get(&value, check);
      if (check < 0 || !std::isfinite(value))
        return -1;
    }
    std::lock_guard<std::recursive_mutex> lock(stateMutex);
    bank = std::move(restored);
    for (int s = 0; s < Slots; ++s)
      publish(s);
    return UnserializeParams(chunk, pos);
  } catch (...) {
    return -1;
  }
}
void MusicGeneratorLoops::OnRestoreState() {
  iplug::Plugin::OnRestoreState();
  for (int id = 0; id < NumParams; ++id)
    player.uiParameter(id, GetParam(id)->Value());
  if (GetUI())
    GetUI()->SetAllControlsDirty();
}

void MusicGeneratorLoops::GetBusName(ERoute direction, int bus, int,
                                     WDL_String &name) const {
  name.Set(direction == ERoute::kOutput
               ? (bus == 0 ? "Master" : ("Loop " + std::to_string(bus)).c_str())
               : "Input");
}
