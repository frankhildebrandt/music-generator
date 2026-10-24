#include "Bank.h"
#include <clap/clap.h>
#include <cstring>
#include <dlfcn.h>
#include <iostream>
#include <stdexcept>
using namespace loops;
void require(bool b, const char *m) {
  if (!b)
    throw std::runtime_error(m);
}
struct Stream {
  std::vector<uint8_t> bytes;
  size_t pos = 0;
};
int64_t readStream(const clap_istream_t *stream, void *buffer, uint64_t bytes) {
  auto &s = *static_cast<Stream *>(stream->ctx);
  size_t n = std::min<size_t>(bytes, s.bytes.size() - s.pos);
  memcpy(buffer, s.bytes.data() + s.pos, n);
  s.pos += n;
  return n;
}
int64_t writeStream(const clap_ostream_t *stream, const void *buffer,
                    uint64_t bytes) {
  auto &s = *static_cast<Stream *>(stream->ctx);
  auto *b = static_cast<const uint8_t *>(buffer);
  s.bytes.insert(s.bytes.end(), b, b + bytes);
  return bytes;
}
int main(int argc, char **argv) {
  try {
    require(argc == 2, "Pass CLAP bundle executable");
    void *module = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!module)
      throw std::runtime_error(dlerror());
    auto *entry =
        static_cast<const clap_plugin_entry_t *>(dlsym(module, "clap_entry"));
    require(entry && entry->init(argv[1]), "CLAP entry init failed");
    auto *factory = static_cast<const clap_plugin_factory_t *>(
        entry->get_factory(CLAP_PLUGIN_FACTORY_ID));
    require(factory && factory->get_plugin_count(factory) == 1,
            "Expected one instrument");
    auto *descriptor = factory->get_plugin_descriptor(factory, 0);
    clap_host_t host{CLAP_VERSION,
                     nullptr,
                     "Loops contract host",
                     "Alphatester x KRGSH",
                     "https://example.com",
                     "1",
                     [](const clap_host_t *, const char *) -> const void * {
                       return nullptr;
                     },
                     [](const clap_host_t *) {},
                     [](const clap_host_t *) {},
                     [](const clap_host_t *) {}};
    auto *plugin = factory->create_plugin(factory, &host, descriptor->id);
    require(plugin && plugin->init(plugin), "Plugin init failed");
    auto *ports = static_cast<const clap_plugin_audio_ports_t *>(
        plugin->get_extension(plugin, CLAP_EXT_AUDIO_PORTS));
    require(ports && ports->count(plugin, false) == 17,
            "Expected 17 stereo buses");
    auto *params = static_cast<const clap_plugin_params_t *>(
        plugin->get_extension(plugin, CLAP_EXT_PARAMS));
    require(params && params->count(plugin) == 1024,
            "Expected stable per-note parameters");
    auto *state = static_cast<const clap_plugin_state_t *>(
        plugin->get_extension(plugin, CLAP_EXT_STATE));
    require(state, "Missing state extension");
    Bank bank;
    auto a = std::make_shared<Audio>();
    a->samples.resize(24000);
    for (size_t n = 0; n < a->samples.size(); ++n) {
      float v = float(std::sin(n * 2 * 3.141592653589793 * 440 / 48000) * 0.25);
      a->samples[n] = {v, v};
    }
    bank[36].original = a;
    bank[36].edited = a;
    bank[36].beats = 1;
    bank[36].name = "Embedded test";
    auto bytes = saveBank(bank);
    Stream input;
    int32_t size = int32_t(bytes.size());
    input.bytes.resize(4);
    memcpy(input.bytes.data(), &size, 4);
    input.bytes.insert(input.bytes.end(), bytes.begin(), bytes.end());
    for (const auto &slot : bank)
      for (double value : slot.params) {
        const auto *b = reinterpret_cast<const uint8_t *>(&value);
        input.bytes.insert(input.bytes.end(), b, b + 8);
      }
    clap_istream_t in{&input, readStream};
    require(state->load(plugin, &in), "Embedded bank restore failed");
    require(plugin->activate(plugin, 48000, 64, 256), "Activate failed");
    require(plugin->start_processing(plugin), "Start processing failed");
    std::array<std::array<double, 256>, 34> data{};
    std::array<std::array<double *, 2>, 17> ptrs{};
    std::array<clap_audio_buffer_t, 17> buffers{};
    for (int bus = 0; bus < 17; ++bus) {
      ptrs[bus] = {data[bus * 2].data(), data[bus * 2 + 1].data()};
      buffers[bus].data64 = ptrs[bus].data();
      buffers[bus].channel_count = 2;
    }
    clap_event_midi_t note{};
    note.header = {sizeof(note), 37, CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_MIDI,
                   0};
    note.port_index = 0;
    note.data[0] = 0x90;
    note.data[1] = 36;
    note.data[2] = 100;
    clap_input_events_t events{
        &note, [](const clap_input_events_t *) -> uint32_t { return 1; },
        [](const clap_input_events_t *in,
           uint32_t) -> const clap_event_header_t * {
          return &static_cast<const clap_event_midi_t *>(in->ctx)->header;
        }};
    clap_input_events_t empty{
        nullptr, [](const clap_input_events_t *) -> uint32_t { return 0; },
        [](const clap_input_events_t *,
           uint32_t) -> const clap_event_header_t * { return nullptr; }};
    clap_process_t process{};
    process.frames_count = 256;
    process.audio_outputs = buffers.data();
    process.audio_outputs_count = 17;
    process.in_events = &events;
    double energy = 0;
    for (int block = 0; block < 128; ++block) {
      process.in_events = block == 0 ? &events : &empty;
      require(plugin->process(plugin, &process) != CLAP_PROCESS_ERROR,
              "Process failed");
      for (int ch = 0; ch < 34; ++ch)
        for (int n = 0; n < 256; ++n) {
          require(std::isfinite(data[ch][n]), "Nonfinite output");
          if (ch < 2)
            energy += data[ch][n] * data[ch][n];
          else
            require(data[ch][n] == 0, "Output leaked to unassigned bus");
        }
      if (block == 0)
        for (int n = 0; n < 37; ++n)
          require(data[0][n] == 0, "MIDI fired before sample offset");
    }
    require(energy > 10, "Restored instrument is silent");
    plugin->stop_processing(plugin);
    plugin->deactivate(plugin);
    Stream saved;
    clap_ostream_t out{&saved, writeStream};
    require(state->save(plugin, &out), "State save failed");
    require(saved.bytes.size() > 384000, "Audio not embedded");
    plugin->destroy(plugin);
    auto *restored = factory->create_plugin(factory, &host, descriptor->id);
    require(restored && restored->init(restored),
            "Second instance init failed");
    auto *state2 = static_cast<const clap_plugin_state_t *>(
        restored->get_extension(restored, CLAP_EXT_STATE));
    clap_istream_t back{&saved, readStream};
    require(state2->load(restored, &back), "Saved state failed to reopen");
    restored->destroy(restored);
    entry->deinit();
    dlclose(module);
    std::cout << "CLAP scan, 17 buses, 1024 parameters, embedded audio, MIDI "
                 "sample offsets, rendering and project recall passed\n";
  } catch (const std::exception &e) {
    std::cerr << e.what() << '\n';
    return 1;
  }
}
