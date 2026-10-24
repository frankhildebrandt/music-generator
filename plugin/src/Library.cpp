#include "Library.h"
namespace loops {
Json factoryTemplates() {
  return Json::array(
      {{{"name", "Midnight Electro"},
        {"kind", "idea"},
        {"prompt", "Dark, hypnotic electro. Warm analog texture, tight groove, "
                   "sparse space."}},
       {{"name", "Dusty Basement"},
        {"kind", "idea"},
        {"prompt", "Raw, dry underground groove. Dusty drums, restrained bass, "
                   "intimate space."}},
       {{"name", "Warm Tape House"},
        {"kind", "idea"},
        {"prompt", "Warm, loose house groove. Tape saturation, soft "
                   "transients, soulful chords."}},
       {{"name", "Clean Minimal"},
        {"kind", "idea"},
        {"prompt", "Sparse, precise minimal electronic groove. Clean "
                   "transients, subtle motion."}},
       {{"name", "Syncopated Sub"},
        {"kind", "instrument"},
        {"instrument", "bass"},
        {"prompt", "Monophonic sub bass, short syncopated notes, a few slides. "
                   "Leave space for the kick."}},
       {{"name", "Tight Machine"},
        {"kind", "instrument"},
        {"instrument", "drums"},
        {"prompt", "Dry machine drums, tight kick, crisp hats, restrained "
                   "fills. No melodic instruments."}},
       {{"name", "Analog Chords"},
        {"kind", "instrument"},
        {"instrument", "synth"},
        {"prompt", "Warm analog chord stabs with subtle movement. Leave space "
                   "for bass and drums."}},
       {{"name", "Shifting Texture"},
        {"kind", "instrument"},
        {"instrument", "fx"},
        {"prompt", "Evolving atmospheric texture, subtle rhythmic motion, "
                   "seamless repeating ambience."}}});
}
std::string fullPrompt(const Json &document,
                       const std::string &instrumentPrompt) {
  const auto idea = document.value("idea", "");
  return idea.empty()
             ? instrumentPrompt
             : "Sound idea: " + idea + "\nInstrument: " + instrumentPrompt;
}
std::string noteName(int midi) {
  static const char *names[]{"C",  "C#", "D",  "D#", "E",  "F",
                             "F#", "G",  "G#", "A",  "A#", "B"};
  return std::string(names[midi % 12]) + std::to_string(midi / 12 - 1);
}
} // namespace loops
