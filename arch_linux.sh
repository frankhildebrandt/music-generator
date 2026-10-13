#!/bin/bash
# Music Generator – Arch Linux Installer, Starter und Manager.
# Autor: ZeroDot1
# Prüft das Arch-System, installiert fehlende Pakete (pacman/yay/uv),
# baut den Modellserver (Vulkan/ROCm/CUDA/CPU), lädt die Modelle und startet die App.
#
# Optionen:
#   --yes, -y              Keine Rückfragen, Empfehlungen automatisch übernehmen
#   --yay                  Fehlende Pakete mit yay (AUR-Helper) installieren
#   --start                Nur die Server starten und Browser öffnen
#   --stop                 Laufende Server beenden
#   --status               Status der Server anzeigen
#   --install-only         Nur installieren/bauen/Modelle laden, nicht starten
#   --backend [auto|rocm|hip|vulkan|cuda|cpu]   Grafik-/Rechen-Backend wählen (Standard: auto)
#                          (rocm/hip empfohlen für AMD Radeon wie RX 9070 XT)
#   --lm [0.6B|1.7B|4B]    Sprachmodellgröße festlegen
#   --xl                   Zusätzlich das XL-Klangmodell herunterladen (5,3 GB)
#   --no-browser           Browser beim Start nicht automatisch öffnen
#   --help, -h             Diese Hilfe anzeigen

set -e

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SOURCE_DIR/scripts/common.sh"
cd "$ROOT" || exit 1

AUTO=0
ACTION="run"       # run, install_only, start, stop, status
LM_SIZE=""
XL=0
REBUILD=0
BACKEND="auto"
OPEN_BROWSER=1
USE_YAY=0

while [ $# -gt 0 ]; do
  case "$1" in
    --yes|-y) AUTO=1 ;;
    --yay) USE_YAY=1 ;;
    --start) ACTION="start" ;;
    --stop) ACTION="stop" ;;
    --status) ACTION="status" ;;
    --install-only|--no-start) ACTION="install_only" ;;
    --rebuild) REBUILD=1 ;;
    --backend) BACKEND="$2"; shift ;;
    --lm) LM_SIZE="$2"; shift ;;
    --xl) XL=1 ;;
    --no-browser) OPEN_BROWSER=0 ;;
    --help|-h)
      echo "Music Generator – Arch Linux Setup & Launcher"
      echo "Autor: ZeroDot1"
      echo ""
      echo "Verwendung: $0 [OPTIONEN]"
      echo ""
      echo "Optionen:"
      echo "  --yes, -y              Automatische Installation ohne Rückfragen"
      echo "  --yay                  Fehlende Pakete mit yay (AUR-Helper) installieren"
      echo "  --start                Nur Server starten und Browser öffnen"
      echo "  --stop                 Laufende Server beenden"
      echo "  --status               Status der Server anzeigen"
      echo "  --install-only         Nur installieren und einrichten, nicht starten"
      echo "  --rebuild              acestep.cpp neu kompilieren"
      echo "  --backend [auto|rocm|hip|vulkan|cuda|cpu]   Backend wählen (Standard: auto)"
      echo "                         (rocm/hip für AMD Radeon wie RX 9070 XT)"
      echo "  --lm [0.6B|1.7B|4B]    Sprachmodellgröße erzwingen"
      echo "  --xl                   XL-Klangmodell herunterladen"
      echo "  --no-browser           Browser nicht automatisch öffnen"
      echo "  --help, -h             Hilfe anzeigen"
      exit 0
      ;;
    *)
      echo "Unbekannte Option: $1. Siehe $0 --help"
      exit 1
      ;;
  esac
  shift
done

abbruch() {
  echo ""
  echo "✗ $1"
  [ "$AUTO" = 1 ] || read -n 1 -s -r -p "Taste drücken zum Beenden..." 2>/dev/null || true
  echo ""
  exit 1
}

schritt() { echo ""; echo "▸ $1"; }

ja() {
  [ "$AUTO" = 1 ] && return 0
  local a
  read -r -p "$1 [Enter = ja, n = nein] " a
  [ -z "$a" ] || [[ "$a" =~ ^[jJyY] ]]
}

berechne_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

oeffne_browser() {
  local url="$1"
  [ "$OPEN_BROWSER" = 1 ] || return 0
  if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$url" >/dev/null 2>&1 &
  elif command -v python3 >/dev/null 2>&1; then
    python3 -m webbrowser "$url" >/dev/null 2>&1 &
  fi
}

# --- Server-Stopp -------------------------------------------------------------
stoppe_server() {
  schritt "Server beenden"
  local found=0
  for P in "$PORT" "$SYNTH_PORT" "$LM_PORT"; do
    local pids
    pids=$(port_pids "$P")
    if [ -n "$pids" ]; then
      echo "  Stoppe Port $P (PID: $pids)..."
      kill $pids 2>/dev/null || true
      found=1
    fi
  done
  sleep 1
  for P in "$PORT" "$SYNTH_PORT" "$LM_PORT"; do
    local pids
    pids=$(port_pids "$P")
    if [ -n "$pids" ]; then
      kill -9 $pids 2>/dev/null || true
    fi
  done
  if [ "$found" = 1 ]; then
    echo "  ✓ Server erfolgreich beendet."
  else
    echo "  Keine laufenden Server gefunden."
  fi
}

# --- Server-Status ------------------------------------------------------------
zeige_status() {
  schritt "Server-Status"
  lade_conf
  echo "  Konfiguration: Modus $MODE, LM $LM_FILE, VAE_CHUNK=${VAE_CHUNK:-Standard}"
  if laeuft "$PORT"; then
    echo "  ✓ Web-App läuft auf Port $PORT (http://localhost:$PORT)"
  else
    echo "  ✗ Web-App läuft nicht (Port $PORT frei)"
  fi
  if laeuft "$SYNTH_PORT"; then
    echo "  ✓ Synthese-Server läuft auf Port $SYNTH_PORT"
  else
    echo "  ✗ Synthese-Server läuft nicht (Port $SYNTH_PORT frei)"
  fi
  if [ "$MODE" = "split" ]; then
    if laeuft "$LM_PORT"; then
      echo "  ✓ Sprachmodell-Server (CPU) läuft auf Port $LM_PORT"
    else
      echo "  ✗ Sprachmodell-Server läuft nicht (Port $LM_PORT frei)"
    fi
  fi
}

# --- Server-Start -------------------------------------------------------------
starte_server() {
  schritt "Music Generator starten"
  lade_conf
  mkdir -p "$DATA"

  if [ ! -x "$ENGINE/build/ace-server" ]; then
    echo "  acestep.cpp ist noch nicht gebaut. Führe zuerst die Installation aus."
    installiere
  fi

  # Hardware- und GPU-Umgebungsvariablen (ROCm / AMD Radeon / Multi-GPU)
  [ -d "/opt/rocm/bin" ] && export PATH="/opt/rocm/bin:$PATH"
  [ -d "/opt/rocm/lib" ] && export LD_LIBRARY_PATH="/opt/rocm/lib:${LD_LIBRARY_PATH:-}"
  export HIP_VISIBLE_DEVICES=0
  export DRI_PRIME=1
  export GGML_VK_VISIBLE_DEVICES=0

  local SERVER_ARGS=(--models "$MODELS" --max-batch 1)
  [ -n "$VAE_CHUNK" ] && SERVER_ARGS+=(--vae-chunk "$VAE_CHUNK")

  # 1. Sprachmodell-Server (bei split auf CPU)
  if [ "$MODE" = "split" ]; then
    if laeuft "$LM_PORT"; then
      echo "  Sprachmodell-Server läuft bereits (Port $LM_PORT)."
    else
      echo "  Starte Sprachmodell-Server (CPU)..."
      if command -v setsid >/dev/null 2>&1; then
        setsid env GGML_BACKEND=CPU "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$LM_PORT" \
          "${SERVER_ARGS[@]}" > "$DATA/ace-lm.log" 2>&1 &
      else
        GGML_BACKEND=CPU nohup "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$LM_PORT" \
          "${SERVER_ARGS[@]}" > "$DATA/ace-lm.log" 2>&1 &
      fi
      disown 2>/dev/null || true
    fi
  fi

  # 2. Synthese-Server
  if laeuft "$SYNTH_PORT"; then
    echo "  Synthese-Server läuft bereits (Port $SYNTH_PORT)."
  else
    echo "  Starte Synthese-Server..."
    if command -v setsid >/dev/null 2>&1; then
      setsid "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$SYNTH_PORT" \
        "${SERVER_ARGS[@]}" > "$DATA/ace-server.log" 2>&1 &
    else
      nohup "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$SYNTH_PORT" \
        "${SERVER_ARGS[@]}" > "$DATA/ace-server.log" 2>&1 &
    fi
    disown 2>/dev/null || true
  fi

  # 3. Web-App
  local APP_PID=""
  APP_PID=$(port_pids "$PORT" | head -1 || true)
  if [ -n "$APP_PID" ]; then
    local START_TIME=""
    [ -d "/proc/$APP_PID" ] && START_TIME=$(stat -c '%Y' "/proc/$APP_PID" 2>/dev/null || true)
    local NEUESTE=""
    NEUESTE=$(find app static -type f \( -name '*.py' -o -name '*.js' -o -name '*.html' -o -name '*.css' \) -exec stat -c '%Y' {} + 2>/dev/null | sort -rn | head -1 || true)
    if [ -n "$START_TIME" ] && [ -n "$NEUESTE" ] && [ "$NEUESTE" -gt "$START_TIME" ]; then
      echo "  Code wurde aktualisiert. Web-App wird neu gestartet..."
      kill "$APP_PID" 2>/dev/null || true
      for _ in $(seq 1 20); do laeuft "$PORT" || break; sleep 0.5; done
      APP_PID=""
    else
      echo "  Web-App läuft bereits auf Port $PORT."
    fi
  fi

  if [ -z "$APP_PID" ]; then
    echo "  Starte Web-App..."
    if command -v setsid >/dev/null 2>&1; then
      setsid uv run uvicorn app.main:app --host 127.0.0.1 --port "$PORT" > "$DATA/server.log" 2>&1 &
    else
      nohup uv run uvicorn app.main:app --host 127.0.0.1 --port "$PORT" > "$DATA/server.log" 2>&1 &
    fi
    disown 2>/dev/null || true
  fi

  # 4. Warten, bis alles antwortet
  echo "  Warte auf Server..."
  for _ in $(seq 1 60); do
    laeuft "$PORT" && laeuft "$SYNTH_PORT" && { [ "$MODE" != "split" ] || laeuft "$LM_PORT"; } && break
    sleep 0.5
  done

  laeuft "$PORT" || abbruch "Web-App konnte nicht rechtzeitig starten. Siehe data/server.log"
  laeuft "$SYNTH_PORT" || abbruch "Synthese-Server konnte nicht rechtzeitig starten. Siehe data/ace-server.log"
  if [ "$MODE" = "split" ]; then
    laeuft "$LM_PORT" || abbruch "Sprachmodell-Server konnte nicht rechtzeitig starten. Siehe data/ace-lm.log"
  fi

  echo "  ✓ Alle Server laufen!"
  oeffne_browser "http://localhost:$PORT"
  echo ""
  echo "═══════════════════════════════════════════════════════════"
  echo "  Music Generator geöffnet unter: http://localhost:$PORT"
  echo "  Zum Beenden:  $0 --stop"
  echo "  oder:         './Music Generator OFF.sh'"
  echo "═══════════════════════════════════════════════════════════"
}

# --- Haupt-Installation --------------------------------------------------------
installiere() {
  echo "═══════════════════════════════════════════════════════════"
  echo "  MUSIC GENERATOR · Arch Linux Setup"
  echo "═══════════════════════════════════════════════════════════"

  # 1. System und Hardware prüfen
  schritt "1. System und Hardware prüfen"
  if [ "$(uname -s)" != "Linux" ]; then
    abbruch "Dieses Skript ist für Linux ausgelegt (erkannt: $(uname -s))."
  fi

  if [ -f /etc/os-release ]; then
    # shellcheck source=/dev/null
    source /etc/os-release
    echo "  Distribution: $PRETTY_NAME"
    if [[ "$ID" != "arch" && "$ID_LIKE" != *"arch"* && "$ID" != "cachyos" && "$ID" != "manjaro" && "$ID" != "endeavouros" ]]; then
      echo "  Hinweis: Kein Standard-Arch-System erkannt ($ID). pacman-Paketverwaltung wird dennoch vorausgesetzt."
    fi
  fi

  ARCH=$(uname -m)
  echo "  Architektur:  $ARCH"
  if [ "$ARCH" != "x86_64" ]; then
    echo "  Warnung: Nicht-x86_64 Architektur. acestep.cpp wird möglicherweise nicht voll unterstützt."
  fi

  # RAM ermitteln
  local mem_kb
  mem_kb=$(awk '/MemTotal/ {print $2}' /proc/meminfo 2>/dev/null || echo 0)
  RAM_GB=$(( mem_kb / 1048576 ))
  echo "  Arbeitsspeicher: ${RAM_GB} GB"

  # Freier Speicherplatz
  local free_kb
  free_kb=$(df -k "$ROOT" | awk 'NR==2 {print $4}')
  FREE_GB=$(( free_kb / 1048576 ))
  echo "  Freier Speicherplatz: ${FREE_GB} GB"

  # GPU-Erkennung
  local GPU_INFO
  GPU_INFO=$(lspci 2>/dev/null | grep -iE 'vga|3d|display' || true)
  local HAS_NVIDIA=0
  local HAS_AMD=0
  local HAS_INTEL=0
  local VRAM_GB=0
  local AMD_GFX=""

  if echo "$GPU_INFO" | grep -iq 'nvidia'; then HAS_NVIDIA=1; fi
  if echo "$GPU_INFO" | grep -iqE 'amd|radeon|advanced micro devices'; then HAS_AMD=1; fi
  if echo "$GPU_INFO" | grep -iq 'intel'; then HAS_INTEL=1; fi

  if [ "$HAS_NVIDIA" = 1 ] && command -v nvidia-smi >/dev/null 2>&1; then
    local vram_mb
    vram_mb=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits 2>/dev/null | head -1 || echo 0)
    VRAM_GB=$(( (vram_mb + 512) / 1024 ))
  elif [ "$HAS_AMD" = 1 ]; then
    # VRAM via sysfs (/sys/class/drm) oder rocm-smi
    local vram_bytes=0
    for f in /sys/class/drm/card*/device/mem_info_vram_total; do
      if [ -r "$f" ]; then
        vram_bytes=$(cat "$f" 2>/dev/null || echo 0)
        [ "$vram_bytes" -gt 0 ] && break
      fi
    done
    if [ "$vram_bytes" -gt 0 ]; then
      VRAM_GB=$(( (vram_bytes / 1048576 + 512) / 1024 ))
    elif command -v rocm-smi >/dev/null 2>&1; then
      local vram_raw
      vram_raw=$(rocm-smi --showmeminfo vram 2>/dev/null | grep -o 'VRAM Total Memory (B): [0-9]*' | awk '{print $NF}' | head -1 || echo 0)
      [ "$vram_raw" -gt 0 ] && VRAM_GB=$(( (vram_raw / 1048576 + 512) / 1024 ))
    fi
    # AMD GFX Ziel-Architektur ermitteln (z.B. gfx1201 für RX 9070 XT)
    if command -v rocminfo >/dev/null 2>&1; then
      AMD_GFX=$(rocminfo 2>/dev/null | grep -m1 -o 'gfx[0-9a-z]*' || true)
    fi
  fi

  echo "  Grafikkarte(n):"
  if [ -n "$GPU_INFO" ]; then
    echo "$GPU_INFO" | sed 's/^/    /'
  else
    echo "    Keine dedizierte GPU gefunden"
  fi
  [ "$VRAM_GB" -gt 0 ] && echo "    Grafikspeicher (VRAM): ${VRAM_GB} GB"
  [ -n "$AMD_GFX" ] && echo "    AMD GPU-Architektur:  $AMD_GFX"

  # VAE Chunking nach VRAM festlegen
  VAE_CHUNK=""
  if [ "$VRAM_GB" -gt 0 ]; then
    if   [ "$VRAM_GB" -lt 8 ];  then VAE_CHUNK="128"
    elif [ "$VRAM_GB" -lt 12 ]; then VAE_CHUNK="256"
    elif [ "$VRAM_GB" -lt 16 ]; then VAE_CHUNK="512"
    fi
  elif [ "$HAS_AMD" = 1 ] || [ "$HAS_INTEL" = 1 ]; then
    # Bei AMD/Intel im Vulkan-Modus ist VAE_CHUNK=256 ein sicherer Standard für Karten bis 16GB
    VAE_CHUNK="256"
  fi

  # 2. Backend bestimmen
  schritt "2. Rechen-Backend bestimmen"
  local CHOSEN_BACKEND="$BACKEND"
  if [ "$CHOSEN_BACKEND" = "auto" ]; then
    if [ "$HAS_AMD" = 1 ] && { [ -d "/opt/rocm" ] || command -v hipcc >/dev/null 2>&1 || command -v rocminfo >/dev/null 2>&1; }; then
      CHOSEN_BACKEND="rocm"
    elif [ "$HAS_NVIDIA" = 1 ] && { command -v nvcc >/dev/null 2>&1 || [ -d "/opt/cuda" ]; }; then
      CHOSEN_BACKEND="cuda"
    elif [ "$HAS_AMD" = 1 ] || [ "$HAS_INTEL" = 1 ] || [ "$HAS_NVIDIA" = 1 ]; then
      CHOSEN_BACKEND="vulkan"
    else
      CHOSEN_BACKEND="cpu"
    fi
  fi
  [ "$CHOSEN_BACKEND" = "hip" ] && CHOSEN_BACKEND="rocm"
  echo "  Ausgewähltes Backend: $CHOSEN_BACKEND"

  # 3. Arch Linux Pakete prüfen und installieren
  schritt "3. Arch Linux Abhängigkeiten prüfen (pacman / yay)"
  local BASE_PKGS=(base-devel cmake git curl lsof xdg-utils)
  local GPU_PKGS=()

  if [ "$CHOSEN_BACKEND" = "rocm" ]; then
    # Für AMD ROCm / HIP
    if ! command -v hipcc >/dev/null 2>&1 && ! pacman -Q hip-runtime-amd >/dev/null 2>&1; then
      GPU_PKGS+=(rocm-hip-sdk)
    fi
  elif [ "$CHOSEN_BACKEND" = "vulkan" ]; then
    GPU_PKGS=(vulkan-headers spirv-headers shaderc vulkan-icd-loader)
    if [ "$HAS_AMD" = 1 ]; then
      GPU_PKGS+=(vulkan-radeon)
    elif [ "$HAS_INTEL" = 1 ]; then
      GPU_PKGS+=(vulkan-intel)
    elif [ "$HAS_NVIDIA" = 1 ]; then
      GPU_PKGS+=(nvidia-utils)
    fi
  elif [ "$CHOSEN_BACKEND" = "cuda" ]; then
    GPU_PKGS=(cuda)
  fi

  local NEEDED_PKGS=()
  for pkg in "${BASE_PKGS[@]}" "${GPU_PKGS[@]}"; do
    if ! pacman -Q "$pkg" >/dev/null 2>&1; then
      # Sonderfall base-devel (Meta-Paket)
      if [ "$pkg" = "base-devel" ] && pacman -Qg base-devel >/dev/null 2>&1; then
        continue
      fi
      # Sonderfall gcc
      if [ "$pkg" = "base-devel" ] && command -v gcc >/dev/null 2>&1 && command -v make >/dev/null 2>&1; then
        continue
      fi
      # Sonderfall lsof: ss oder fuser reicht aus
      if [ "$pkg" = "lsof" ] && { command -v ss >/dev/null 2>&1 || command -v fuser >/dev/null 2>&1; }; then
        continue
      fi
      # Sonderfall Vulkan Treiber: wenn bereits ein ICD installiert ist
      if [[ "$pkg" =~ vulkan-(radeon|intel) ]] && pacman -Q vulkan-icd-loader >/dev/null 2>&1 && [ -d /usr/share/vulkan/icd.d ] && [ "$(ls -A /usr/share/vulkan/icd.d 2>/dev/null)" ]; then
        continue
      fi
      NEEDED_PKGS+=("$pkg")
    fi
  done

  if [ ${#NEEDED_PKGS[@]} -gt 0 ]; then
    echo "  Folgende Arch-Pakete fehlen:"
    for p in "${NEEDED_PKGS[@]}"; do echo "    - $p"; done
    echo ""
    local METHOD=""
    if [ "$USE_YAY" = 1 ]; then
      if command -v yay >/dev/null 2>&1; then
        METHOD="yay"
      else
        echo "  Hinweis: yay wurde angefordert, ist aber nicht installiert. Fallback auf sudo pacman."
        METHOD="pacman"
      fi
    elif [ "$AUTO" = 1 ]; then
      METHOD="pacman"
    else
      if command -v yay >/dev/null 2>&1; then
        read -r -p "  Fehlende Pakete jetzt installieren mit [Enter = sudo pacman, y = yay, n = abbrechen]? " a
        if [[ "$a" =~ ^[yY] ]]; then
          METHOD="yay"
        elif [ -z "$a" ] || [[ "$a" =~ ^[jJpP] ]]; then
          METHOD="pacman"
        fi
      else
        if ja "  Fehlende Pakete jetzt mit 'sudo pacman -S --needed' installieren?"; then
          METHOD="pacman"
        fi
      fi
    fi

    local install_rc=0
    if [ "$METHOD" = "yay" ]; then
      echo "  Installiere Pakete mit 'yay -S --needed'..."
      yay -S --needed "${NEEDED_PKGS[@]}" || install_rc=$?
    elif [ "$METHOD" = "pacman" ]; then
      echo "  Installiere Pakete mit 'sudo pacman -S --needed'..."
      sudo pacman -S --needed "${NEEDED_PKGS[@]}" || install_rc=$?
    else
      install_rc=1
    fi

    if [ "$install_rc" -ne 0 ]; then
      echo "  Warnung: Installation einiger Pakete fehlgeschlagen oder abgebrochen."
      if [ "$CHOSEN_BACKEND" = "vulkan" ] || [ "$CHOSEN_BACKEND" = "rocm" ] || [ "$CHOSEN_BACKEND" = "cuda" ]; then
        echo "  Wechsle zu CPU-Backend als Fallback..."
        CHOSEN_BACKEND="cpu"
      fi
    fi
  else
    echo "  ✓ Alle benötigten Systempakete sind vorhanden."
  fi

  # uv (Python-Paketmanager)
  schritt "4. uv & Python-Umgebung einrichten"
  if ! command -v uv >/dev/null 2>&1; then
    echo "  uv ist nicht im PATH. Versuche Installation..."
    if [ "$USE_YAY" = 1 ] && command -v yay >/dev/null 2>&1; then
      yay -S --needed uv || true
    elif ja "  uv mit pacman installieren?"; then
      sudo pacman -S --needed uv || true
    fi
    if ! command -v uv >/dev/null 2>&1; then
      echo "  Installiere uv per offiziellem Astral-Installationsskript..."
      curl -LsSf https://astral.sh/uv/install.sh | sh || abbruch "uv konnte nicht installiert werden."
      export PATH="$HOME/.local/bin:$PATH"
    fi
  fi
  echo "  uv-Version: $(uv --version)"

  echo "  Installiere Python-Abhängigkeiten der Web-App (uv sync)..."
  uv sync --quiet || abbruch "Python-Abhängigkeiten konnten nicht installiert werden."
  echo "  ✓ Python-Umgebung bereit."

  # 5. Modellserver (acestep.cpp) bauen
  schritt "5. Modellserver acestep.cpp bauen ($CHOSEN_BACKEND)"
  mkdir -p "$ROOT/engine"
  if [ ! -d "$ENGINE/.git" ]; then
    echo "  Klone acestep.cpp..."
    git clone --quiet "$ACESTEP_REPO" "$ENGINE" || abbruch "Klonen von acestep.cpp fehlgeschlagen."
  fi

  ( cd "$ENGINE" \
    && { git cat-file -e "$ACESTEP_REV^{commit}" 2>/dev/null || git fetch --quiet origin; } \
    && git checkout --quiet "$ACESTEP_REV" \
    && git submodule update --init --recursive --quiet ) || abbruch "Konnte nicht auf Version ${ACESTEP_REV:0:7} wechseln."

  local BUILD_NEEDED=0
  if [ "$REBUILD" = 1 ]; then
    BUILD_NEEDED=1
  elif [ ! -x "$ENGINE/build/ace-server" ]; then
    BUILD_NEEDED=1
  elif ! "$ENGINE/build/ace-server" --help 2>&1 | head -1 | grep -q "${ACESTEP_REV:0:7}"; then
    BUILD_NEEDED=1
  fi

  if [ "$BUILD_NEEDED" = 1 ]; then
    echo "  Kompiliere acestep.cpp (Backend: $CHOSEN_BACKEND) ..."
    mkdir -p "$DATA"
    local CMAKE_FLAGS=()
    if [ "$CHOSEN_BACKEND" = "rocm" ]; then
      [ -d "/opt/rocm/bin" ] && export PATH="/opt/rocm/bin:$PATH"
      CMAKE_FLAGS=(-DGGML_HIP=ON)
      [ -n "$AMD_GFX" ] && CMAKE_FLAGS+=(-DAMDGPU_TARGETS="$AMD_GFX")
    elif [ "$CHOSEN_BACKEND" = "cuda" ]; then
      [ -d "/opt/cuda/bin" ] && export PATH="/opt/cuda/bin:$PATH"
      CMAKE_FLAGS=(-DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES=native)
    elif [ "$CHOSEN_BACKEND" = "vulkan" ]; then
      CMAKE_FLAGS=(-DGGML_VULKAN=ON)
    else
      CMAKE_FLAGS=(-DGGML_CPU_ALL_VARIANTS=ON)
    fi

    (
      cd "$ENGINE"
      rm -rf build
      cmake -S . -B build "${CMAKE_FLAGS[@]}"
      cmake --build build --config Release -j "$(nproc)"
    ) > "$DATA/build.log" 2>&1 || {
      if [ "$CHOSEN_BACKEND" != "cpu" ]; then
        echo "  Build mit $CHOSEN_BACKEND fehlgeschlagen. Details in data/build.log"
        if ja "  Soll ein Fallback-Build mit reinem CPU-Backend versucht werden?"; then
          CHOSEN_BACKEND="cpu"
          (
            cd "$ENGINE"
            rm -rf build
            cmake -S . -B build -DGGML_CPU_ALL_VARIANTS=ON
            cmake --build build --config Release -j "$(nproc)"
          ) > "$DATA/build.log" 2>&1 || abbruch "CPU-Build ebenfalls fehlgeschlagen. Siehe data/build.log"
        else
          abbruch "Build abgebrochen."
        fi
      else
        abbruch "Build fehlgeschlagen. Details in data/build.log"
      fi
    }
    echo "  ✓ Build erfolgreich abgeschlossen."
  else
    echo "  ✓ acestep.cpp ist bereits kompiliert (${ACESTEP_REV:0:7})."
  fi

  # 6. Modellgröße wählen
  schritt "6. Modellgröße auswählen"
  local EMPF="1.7B"
  if   [ "$RAM_GB" -lt 12 ]; then EMPF="0.6B"
  elif [ "$RAM_GB" -lt 24 ]; then EMPF="1.7B"
  else                            EMPF="4B"
  fi
  if [ "$VRAM_GB" -gt 0 ]; then
    if   [ "$VRAM_GB" -lt 6 ]; then EMPF="0.6B"
    elif [ "$VRAM_GB" -lt 8 ] && [ "$EMPF" = "4B" ]; then EMPF="1.7B"
    fi
  fi

  if [ -z "$LM_SIZE" ]; then
    LM_SIZE="$EMPF"
    if [ "$AUTO" != 1 ]; then
      echo "  Sprachmodell (plant Struktur, Tempo, Tonart; größer = bessere Songs, mehr RAM):"
      echo "    1) 0.6B – ca. 0.7 GB (für 8 GB RAM / kleine GPUs)"
      echo "    2) 1.7B – ca. 2.0 GB (für 16 GB RAM / 6–8 GB VRAM)"
      echo "    3) 4B   – ca. 4.5 GB (ab 24 GB RAM / ab 8 GB VRAM)"
      read -r -p "  Auswahl [Enter = Empfehlung $EMPF]: " a
      case "$a" in
        1) LM_SIZE="0.6B" ;;
        2) LM_SIZE="1.7B" ;;
        3) LM_SIZE="4B" ;;
      esac
    fi
  fi
  case "$LM_SIZE" in 0.6B|1.7B|4B) ;; *) abbruch "Unbekannte Modellgröße: $LM_SIZE (erlaubt: 0.6B, 1.7B, 4B)" ;; esac
  LM_FILE="acestep-5Hz-lm-${LM_SIZE}-Q8_0.gguf"
  echo "  → Sprachmodell: $LM_SIZE, Klangmodell: SFT (Standard, hohe Qualität)"

  # XL Modell optional
  XL_FILE="acestep-v15-xl-sft-Q8_0.gguf"
  [ -f "$MODELS/$XL_FILE" ] && XL=1
  if [ "$XL" != 1 ] && [ "$AUTO" != 1 ]; then
    echo ""
    echo "  XL-Klangmodell (optional, 5.3 GB): noch höhere Klangqualität, benötigt jedoch doppelte Rechenzeit."
    read -r -p "  XL-Klangmodell ebenfalls herunterladen? [Enter = nein, j = ja]: " a
    [[ "$a" =~ ^[jJyY] ]] && XL=1
  fi
  [ "$XL" = 1 ] && echo "  → Inklusive XL-Klangmodell"

  # 7. Modelle herunterladen
  schritt "7. Modelle herunterladen"
  mkdir -p "$MODELS"
  local FILES
  FILES=$(echo "$MODEL_FILES" | grep -v 'acestep-5Hz-lm-\|xl-sft' ; echo "$MODEL_FILES" | grep "^$LM_FILE:")
  [ "$XL" = 1 ] && FILES=$(echo "$FILES"; echo "$MODEL_FILES" | grep 'xl-sft')

  local NEED_MB=0
  for e in $FILES; do
    IFS=: read -r f mb _ <<< "$e"
    [ -f "$MODELS/$f" ] || NEED_MB=$(( NEED_MB + mb ))
  done
  local NEED_GB=$(( (NEED_MB + 1023) / 1024 + 3 ))
  echo "  Downloadvolumen: $(( (NEED_MB + 1023) / 1024 )) GB · Benötigter freier Platz: ca. $NEED_GB GB"
  [ "$FREE_GB" -ge "$NEED_GB" ] || abbruch "Zu wenig freier Festplattenspeicher: $FREE_GB GB frei, $NEED_GB GB benötigt."

  for e in $FILES; do
    IFS=: read -r f _ sha <<< "$e"
    if [ -f "$MODELS/$f" ]; then
      echo "  ✓ $f vorhanden"
      continue
    fi
    echo "  ↓ Lade $f herunter..."
    curl -L --fail -C - --progress-bar -o "$MODELS/$f.part" "$HF_BASE/$f" \
      || abbruch "Download von $f fehlgeschlagen. Erneuter Aufruf setzt den Download fort."
    echo "    Prüfe Prüfsumme (SHA-256)..."
    local calc_sha
    calc_sha=$(berechne_sha256 "$MODELS/$f.part")
    if [ "$calc_sha" != "$sha" ]; then
      rm -f "$MODELS/$f.part"
      abbruch "Prüfsumme von $f stimmt nicht überein. Datei wurde gelöscht; bitte erneut ausführen."
    fi
    mv "$MODELS/$f.part" "$MODELS/$f"
    echo "    ✓ Prüfsumme in Ordnung"
  done

  # 8. Selbsttest & Konfiguration speichern
  schritt "8. Grafik-Selbsttest & Konfiguration"
  mkdir -p "$DATA"
  local OLD_MODE=""
  local OLD_LM=""
  local OLD_REV=""
  [ -f "$CONF" ] && {
    OLD_MODE=$(sed -n 's/^MODE=//p' "$CONF" || true)
    OLD_LM=$(sed -n 's/^LM_FILE=//p' "$CONF" || true)
    OLD_REV=$(sed -n 's/^TESTED_REV=//p' "$CONF" || true)
  }

  local MODE="single"
  if [ -n "$OLD_MODE" ] && [ "$OLD_LM" = "$LM_FILE" ] && [ "${OLD_REV:0:7}" = "${ACESTEP_REV:0:7}" ]; then
    MODE="$OLD_MODE"
    echo "  Bereits getestet: Modus $MODE"
  else
    local TEST_PORT=8099
    if laeuft "$TEST_PORT"; then
      echo "  Test-Port $TEST_PORT belegt, überspringe Probelauf. Modus: single."
      MODE="single"
    else
      echo "  Starte Testlauf des Modells..."
      local TEST_ARGS=(--host 127.0.0.1 --port "$TEST_PORT" --models "$MODELS" --max-batch 1)
      [ -n "$VAE_CHUNK" ] && TEST_ARGS+=(--vae-chunk "$VAE_CHUNK")
      "$ENGINE/build/ace-server" "${TEST_ARGS[@]}" > "$DATA/selftest.log" 2>&1 &
      local TEST_PID=$!
      for _ in $(seq 1 120); do curl -s "http://127.0.0.1:$TEST_PORT/health" >/dev/null && break; sleep 0.5; done
      if uv run python scripts/metal_selftest.py "$TEST_PORT" "$LM_FILE" >/dev/null 2>&1; then
        MODE="single"
      else
        MODE="split"
      fi
      kill "$TEST_PID" 2>/dev/null || true
      wait "$TEST_PID" 2>/dev/null || true
      echo "  → Ermittelter Modus: $MODE"
    fi
  fi

  cat > "$CONF" <<EOF
MODE=$MODE
LM_FILE=$LM_FILE
TESTED_REV=$ACESTEP_REV
VAE_CHUNK=$VAE_CHUNK
BACKEND=$CHOSEN_BACKEND
EOF
  echo "  ✓ Konfiguration in data/engine.conf gespeichert."

  echo ""
  echo "═══════════════════════════════════════════════════════════"
  echo "  Installation unter Arch Linux erfolgreich abgeschlossen!"
  echo "═══════════════════════════════════════════════════════════"
}

# --- Ausführung nach gewählter Aktion ------------------------------------------
case "$ACTION" in
  stop)
    stoppe_server
    ;;
  status)
    zeige_status
    ;;
  install_only)
    installiere
    ;;
  start)
    starte_server
    ;;
  run)
    if [ ! -x "$ENGINE/build/ace-server" ] || [ ! -f "$CONF" ]; then
      installiere
    fi
    starte_server
    ;;
esac
