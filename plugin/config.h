#define PLUG_NAME "Music Generator Loops"
#define PLUG_MFR "Alphatester x KRGSH"
#define PLUG_VERSION_HEX 0x00000102
#define PLUG_VERSION_STR "0.1.2"
#define PLUG_UNIQUE_ID 'MgLp'
#define PLUG_MFR_ID 'AKRG'
#define PLUG_URL_STR "https://github.com/frankhildebrandt/music-generator"
#define PLUG_EMAIL_STR ""
#define PLUG_COPYRIGHT_STR "Copyright 2026 Alphatester x KRGSH"
#define PLUG_CLASS_NAME MusicGeneratorLoops
#define BUNDLE_NAME "MusicGeneratorLoops"
#define BUNDLE_MFR "alphatester-krgsh"
#define BUNDLE_DOMAIN "de"
#ifdef APP_API
#define PLUG_CHANNEL_IO "0-2"
#else
#define PLUG_CHANNEL_IO "0-2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2"
#endif
#define SHARED_RESOURCES_SUBPATH "MusicGeneratorLoops"
#define PLUG_LATENCY 0
#define PLUG_TYPE 1
#define PLUG_DOES_MIDI_IN 1
#define PLUG_DOES_MIDI_OUT 0
#define PLUG_DOES_MPE 0
#define PLUG_DOES_STATE_CHUNKS 1
#define PLUG_HAS_UI 1
#define PLUG_WIDTH 1120
#define PLUG_HEIGHT 780
#define PLUG_FPS 30
#define PLUG_SHARED_RESOURCES 0
#define PLUG_HOST_RESIZE 1
#define VST3_SUBCATEGORY "Instrument|Sampler"
#define CLAP_MANUAL_URL "https://github.com/frankhildebrandt/music-generator/blob/main/docs/loop-plugin.md"
#define CLAP_SUPPORT_URL "https://github.com/frankhildebrandt/music-generator/issues"
#define CLAP_DESCRIPTION "Local ACE-Step stem generation and MIDI loop instrument"
#define CLAP_FEATURES "instrument", "sampler", "stereo"

#define APP_NUM_CHANNELS 2
#define APP_N_VECTOR_WAIT 0
#define APP_MULT 1
#define APP_COPY_AUV3 0
#define APP_SIGNAL_VECTOR_SIZE 64
