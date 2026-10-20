#!/bin/bash
# Music Generator – Installer für macOS.
# Prüft den Mac, installiert was fehlt, lädt das passende Modell und startet die App.
# Kann jederzeit erneut gestartet werden (z. B. nach "git pull"): Erledigtes wird übersprungen.
#
# Optionen:  --yes        keine Rückfragen, Empfehlung übernehmen
#            --lm SIZE    Sprachmodell erzwingen: 0.6B, 1.7B oder 4B
#            --cpu        Sprachmodell immer auf der CPU rechnen (hilft, wenn der Klang auf der GPU verzerrt ist)
#            --auto       zurück zur automatischen Wahl (Standard)
#            --xl         zusätzlich das XL-Klangmodell laden (5,3 GB, in den Einstellungen wählbar)
#            --no-start   App am Ende nicht starten
#            --build      Modellserver selbst kompilieren statt die fertigen Programmdateien zu laden
source "$(dirname "$0")/scripts/common.sh"
cd "$ROOT" || exit 1

AUTO=0; START=1; LM_SIZE=""; XL=0; BUILD=0; DEVICE_ARG=""
while [ $# -gt 0 ]; do
  case "$1" in
    --yes) AUTO=1 ;;
    --no-start) START=0 ;;
    --xl) XL=1 ;;
    --build) BUILD=1 ;;
    --cpu) DEVICE_ARG=cpu ;;
    --auto) DEVICE_ARG=auto ;;
    --lm) LM_SIZE="$2"; shift ;;
  esac
  shift
done

abbruch() {
  echo ""; echo "✗ $1"
  [ "$AUTO" = 1 ] || read -n 1 -s -r -p "Press any key to close..."
  exit 1
}
schritt() { echo ""; echo "▸ $1"; }
ja() {   # Rückfrage mit Enter = ja
  [ "$AUTO" = 1 ] && return 0
  local a; read -r -p "$1 [Enter = yes, n = no] " a
  [ -z "$a" ] || [[ "$a" =~ ^[jJyY] ]]
}

mkdir -p "$DATA"
echo "═══════════════════════════════════════════"
echo "  MUSIC GENERATOR · Installation"
echo "═══════════════════════════════════════════"

# --- 1. Mac prüfen ------------------------------------------------------------
schritt "Checking Mac"
ARCH=$(uname -m)
CHIP=$(sysctl -n machdep.cpu.brand_string 2>/dev/null)
RAM_GB=$(( $(sysctl -n hw.memsize) / 1073741824 ))
FREE_GB=$(df -g "$ROOT" | awk 'NR==2 {print $4}')
echo "  Chip:        $CHIP ($ARCH)"
echo "  Memory:      $RAM_GB GB"
echo "  macOS:       $(sw_vers -productVersion)"
echo "  Free disk space: $FREE_GB GB"

if [ "$ARCH" != "arm64" ]; then
  echo ""
  echo "  Note: Intel Mac. Only Apple Silicon (M1 or newer) has been tested; it may be very slow."
  ja "  Continue anyway?" || abbruch "Cancelled."
fi

# --- 2. Modellgröße wählen ------------------------------------------------------
if   [ "$RAM_GB" -lt 12 ]; then EMPF="0.6B"
elif [ "$RAM_GB" -lt 24 ]; then EMPF="1.7B"
else                            EMPF="4B"
fi
if [ -z "$LM_SIZE" ]; then
  LM_SIZE="$EMPF"
  if [ "$AUTO" != 1 ]; then
    echo ""
    echo "  Language model (plans structure, tempo, key; bigger = better songs, more memory):"
    echo "    1) 0.6B  – 0.7 GB, for 8 GB RAM"
    echo "    2) 1.7B  – 2.0 GB, for 16 GB"
    echo "    3) 4B    – 4.5 GB, from 24 GB"
    read -r -p "  Choice [Enter = $EMPF recommended] " a
    case "$a" in 1) LM_SIZE="0.6B" ;; 2) LM_SIZE="1.7B" ;; 3) LM_SIZE="4B" ;; esac
  fi
fi
case "$LM_SIZE" in 0.6B|1.7B|4B) ;; *) abbruch "Unknown model size: $LM_SIZE (allowed: 0.6B, 1.7B, 4B)" ;; esac
LM_FILE="acestep-5Hz-lm-${LM_SIZE}-Q8_0.gguf"
echo "  → Language model $LM_SIZE, sound model SFT (high quality)"

# Optional: XL-Klangmodell (4 statt 2 Mrd. Parameter). Ist es schon da, bleibt es; sonst nur auf Wunsch.
XL_FILE=$(echo "$MODEL_FILES" | grep 'xl-sft' | cut -d: -f1)
[ -f "$MODELS/$XL_FILE" ] && XL=1
if [ "$XL" != 1 ] && [ "$AUTO" != 1 ]; then
  echo ""
  echo "  Sound model XL (optional extra, 5.3 GB): better sound quality according to the developers,"
  echo "  but takes about twice as long and needs more memory. The smaller model stays the default;"
  echo "  you can pick XL afterwards in the app settings."
  [ "$RAM_GB" -lt 16 ] && echo "  Not recommended with $RAM_GB GB RAM."
  read -r -p "  Download XL as well? [Enter = no, y = yes] " a
  [[ "$a" =~ ^[yY] ]] && XL=1
fi
[ "$XL" = 1 ] && echo "  → plus XL sound model"

# Benötigte Dateien (Name:MB:SHA-256 aus common.sh): die drei festen, das gewählte Sprachmodell, auf Wunsch XL
FILES=$(echo "$MODEL_FILES" | grep -v 'acestep-5Hz-lm-\|xl-sft' ; echo "$MODEL_FILES" | grep "^$LM_FILE:")
[ "$XL" = 1 ] && FILES=$(echo "$FILES"; echo "$MODEL_FILES" | grep 'xl-sft')
NEED_MB=0
for e in $FILES; do
  IFS=: read -r f mb _ <<< "$e"
  [ -f "$MODELS/$f" ] || NEED_MB=$(( NEED_MB + mb ))
done
NEED_GB=$(( (NEED_MB + 1023) / 1024 + 3 ))   # + Build, Python und etwas Platz für Songs
echo "  Download: $(( (NEED_MB + 1023) / 1024 )) GB · about $NEED_GB GB free space needed in total"
[ "$FREE_GB" -ge "$NEED_GB" ] || abbruch "Not enough disk space: $FREE_GB GB free, about $NEED_GB GB needed."
ja "  Start installation?" || abbruch "Cancelled."

# Ab macOS ENGINE_MAC_MIN_OS gibt es fertige Programmdateien: dann braucht es keinen Compiler und kein cmake.
# Selbst bauen nur auf älteren Systemen oder mit --build (Apple-Werkzeuge waren bei mehreren Nutzern defekt).
OS_MAJOR=$(sw_vers -productVersion | cut -d. -f1)
PREBUILT=0
[ "$BUILD" = 0 ] && [ "$ARCH" = "arm64" ] && [ "$OS_MAJOR" -ge "$ENGINE_MAC_MIN_OS" ] && PREBUILT=1   # Intel-Macs bauen selbst

# --- 3. Werkzeuge -------------------------------------------------------------
schritt "Apple developer tools (git$([ "$PREBUILT" = 0 ] && echo ", compiler"))"
if ! xcode-select -p >/dev/null 2>&1; then
  xcode-select --install >/dev/null 2>&1
  abbruch "An Apple window opens to install the developer tools. Please confirm there, then run 'Install' again."
fi
echo "  present"

schritt "uv (Python environment)"
if ! command -v uv >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | sh || abbruch "uv could not be installed."
  export PATH="$HOME/.local/bin:$PATH"
fi
uv --version

if [ "$PREBUILT" = 0 ]; then
  schritt "cmake (to build the model server)"
  if ! command -v cmake >/dev/null 2>&1; then
    uv tool install cmake || abbruch "cmake could not be installed."
  fi
  cmake --version | head -1
fi

schritt "Python packages of the app"
uv sync --quiet || abbruch "Python packages could not be installed."
echo "  ok"

# --- 4. Modellserver (acestep.cpp) ---------------------------------------------
schritt "Model server acestep.cpp (${ACESTEP_REV:0:7})"
mkdir -p "$ENGINE/build" "$MODELS"
# Schon vorhanden (--help zeigt den kurzen Hash)? Mit --build werden fertige Programmdateien durch einen eigenen Build ersetzt.
if "$ENGINE/build/ace-server" --help 2>&1 | head -1 | grep -q "${ACESTEP_REV:0:7}" \
   && { [ "$BUILD" = 0 ] || ! grep -q '^PREBUILT=' "$ENGINE/build/VERSION.txt" 2>/dev/null; }; then
  echo "  already installed"
elif [ "$PREBUILT" = 1 ]; then
  # Fertige Programmdateien aus der Veröffentlichung des Projekts, per SHA-256 geprüft
  echo "  downloading program files $ENGINE_MAC_REV (about 4 MB) …"
  ZIP="$DATA/acestep-cpp-macos-arm64-$ENGINE_MAC_REV.zip"
  curl -L --fail --progress-bar -o "$ZIP.part" "$ENGINE_MAC_URL" || abbruch "Downloading the program files failed. Running Install again retries."
  if [ "$(shasum -a 256 "$ZIP.part" | cut -d' ' -f1)" != "$ENGINE_MAC_SHA256" ]; then
    rm -f "$ZIP.part"
    abbruch "Checksum of the program files does not match. The file was deleted; running Install again downloads it anew."
  fi
  echo "  checksum ok, unpacking …"
  TMP_X="$DATA/engine-unpack"
  rm -rf "$TMP_X" && mkdir -p "$TMP_X"
  ditto -x -k "$ZIP.part" "$TMP_X" || abbruch "Unpacking the program files failed."
  rm -rf "$ENGINE/build" && mkdir -p "$ENGINE/build"
  cp -R "$TMP_X"/acestep-cpp-macos-arm64-*/. "$ENGINE/build/"
  rm -rf "$TMP_X" "$ZIP.part"
  xattr -dr com.apple.quarantine "$ENGINE/build" 2>/dev/null
  { echo "PREBUILT=$(date +%Y-%m-%dT%H:%M:%S)"; echo "ENGINE_REV=$ENGINE_MAC_REV"; echo "SOURCE=$ENGINE_MAC_URL"; echo "ZIP_SHA256=$ENGINE_MAC_SHA256"; } > "$ENGINE/build/VERSION.txt"
  "$ENGINE/build/ace-server" --help 2>&1 | head -1 | grep -q "${ACESTEP_REV:0:7}" \
    || abbruch "The model server does not start. Run Install again with --build to compile it yourself."
  echo "  done ($(shasum -a 256 "$ENGINE/build/ace-server" | cut -c1-12)… ace-server)"
else
  if [ ! -d "$ENGINE/.git" ]; then
    # Ordner gibt es schon (Modelle, fertige Programmdateien): Quelltext hineinholen, ohne etwas zu löschen
    ( cd "$ENGINE" && git init --quiet && git remote add origin "$ACESTEP_REPO" && git fetch --quiet origin ) \
      || abbruch "Downloading acestep.cpp failed."
  fi
  ( cd "$ENGINE" \
    && { git cat-file -e "$ACESTEP_REV^{commit}" 2>/dev/null || git fetch --quiet origin; } \
    && git checkout --quiet --force "$ACESTEP_REV" \
    && git submodule update --init --recursive --quiet ) || abbruch "Could not switch acestep.cpp to version ${ACESTEP_REV:0:7}."
  echo "  building (takes a few minutes) …"
  ( cd "$ENGINE" && ./buildcpu.sh ) > "$DATA/build.log" 2>&1 || abbruch "Build failed, details in data/build.log"
  echo "  done"
fi
pfade_reparieren   # Bibliothekspfade relativ machen, damit der Projektordner verschoben werden darf

# --- 5. Modelle -----------------------------------------------------------------
schritt "Downloading models"
mkdir -p "$MODELS"
NEW_MODELS=0
for e in $FILES; do
  IFS=: read -r f _ sha <<< "$e"
  if [ -f "$MODELS/$f" ]; then echo "  ✓ $f"; continue; fi
  echo "  ↓ $f"
  curl -L --fail -C - --progress-bar -o "$MODELS/$f.part" "$HF_BASE/$f" \
    || abbruch "Downloading $f failed. Running Install again resumes the download."
  echo "    verifying checksum …"
  if [ "$(shasum -a 256 "$MODELS/$f.part" | cut -d' ' -f1)" != "$sha" ]; then
    rm -f "$MODELS/$f.part"
    abbruch "Checksum of $f does not match. The file was deleted; running Install again downloads it anew."
  fi
  mv "$MODELS/$f.part" "$MODELS/$f"
  NEW_MODELS=1
done
# Der Modellserver liest die Modellliste nur beim Start: nach neuen Modellen laufende Server beenden, ON startet sie neu.
if [ "$NEW_MODELS" = 1 ]; then
  for p in "$SYNTH_PORT" "$LM_PORT"; do laeuft "$p" && kill $(lsof -i :"$p" -sTCP:LISTEN -t) 2>/dev/null; done
  sleep 1
fi

# --- 6. Grafikchip testen ------------------------------------------------------
# Auf manchen Chips (z. B. M4) rechnet das Sprachmodell unter Metal falsch.
# Dann läuft es auf der CPU (Modus "split"), sonst reicht ein Server (Modus "single").
schritt "Testing graphics chip"
OLD_MODE=""; OLD_LM=""; OLD_REV=""; OLD_DEVICE=""
[ -f "$CONF" ] && { OLD_MODE=$(sed -n 's/^MODE=//p' "$CONF"); OLD_LM=$(sed -n 's/^LM_FILE=//p' "$CONF"); OLD_REV=$(sed -n 's/^TESTED_REV=//p' "$CONF"); OLD_DEVICE=$(sed -n 's/^LM_DEVICE=//p' "$CONF"); }
LM_DEVICE="$OLD_DEVICE"                                  # die Wahl aus den Einstellungen bleibt erhalten
[ "$DEVICE_ARG" = cpu ] && LM_DEVICE=cpu
[ "$DEVICE_ARG" = auto ] && LM_DEVICE=""
if [ "$LM_DEVICE" = cpu ]; then
  MODE="$OLD_MODE"   # das echte Testergebnis (falls vorhanden) bleibt für „automatisch“ erhalten; gestartet wird per LM_DEVICE auf der CPU
  echo "  language model forced to the CPU (mode split), no test needed"
elif [ -n "$OLD_MODE" ] && [ "$OLD_LM" = "$LM_FILE" ] && [ "${OLD_REV:0:7}" = "${ACESTEP_REV:0:7}" ]; then   # alte Installationen haben den kurzen Hash
  MODE="$OLD_MODE"
  echo "  already tested: mode $MODE"
else
  TEST_PORT=8099
  laeuft "$TEST_PORT" && abbruch "Port $TEST_PORT is in use, please try again later."
  "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$TEST_PORT" --models "$MODELS" --max-batch 1 \
    > "$DATA/selftest.log" 2>&1 &
  TEST_PID=$!
  for _ in $(seq 1 120); do curl -s "http://127.0.0.1:$TEST_PORT/health" >/dev/null && break; sleep 0.5; done
  echo "  Trial run of the language model on Metal (can take 1–2 minutes) …"
  if uv run python scripts/metal_selftest.py "$TEST_PORT" "$LM_FILE"; then MODE=single; else MODE=split; fi
  kill "$TEST_PID" 2>/dev/null; wait "$TEST_PID" 2>/dev/null
  echo "  → mode $MODE"
fi
# Kachelgröße des VAE-Decoders (--vae-chunk, Latent-Frames je Kachel, Standard 1024 = etwa 41 s am Stück).
# Metal teilt sich den Arbeitsspeicher mit allem anderen: bei 8 GB reicht die volle Kachel für Songs über
# 30 s nicht mehr, der Decoder liefert dann Rauschen statt Musik. Kleinere Kacheln brauchen weniger Speicher.
VAE_CHUNK=""
if   [ "$RAM_GB" -lt 12 ]; then VAE_CHUNK="256"
elif [ "$RAM_GB" -lt 16 ]; then VAE_CHUNK="512"
fi
[ -n "$VAE_CHUNK" ] && echo "  VAE tile size: $VAE_CHUNK (RAM $RAM_GB GB)"
cat > "$CONF" <<EOF
MODE=$MODE
LM_FILE=$LM_FILE
TESTED_REV=$ACESTEP_REV
VAE_CHUNK=$VAE_CHUNK
EOF
[ "$LM_DEVICE" = cpu ] && echo "LM_DEVICE=cpu" >> "$CONF"

# --- 7. Start-/Stopp-Symbole ----------------------------------------------------
schritt "Setting up start and stop icons"
bash "$ROOT/scripts/symbole.sh"
echo "  ok"

# --- Fertig ---------------------------------------------------------------------
echo ""
echo "═══════════════════════════════════════════"
echo "  Done. Start with 'Music Generator ON',"
echo "  stop with 'Music Generator OFF'."
echo "═══════════════════════════════════════════"
if [ "$START" = 1 ]; then
  exec "$ROOT/Music Generator ON.command"
fi
