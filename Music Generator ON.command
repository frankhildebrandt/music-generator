#!/bin/bash
# Startet Modellserver + Web-App und öffnet den Browser. Fenster schließt sich danach selbst.
source "$(dirname "$0")/scripts/common.sh"
cd "$ROOT" || exit 1
lade_conf
mkdir -p "$DATA"

if [ ! -x "$ENGINE/build/ace-server" ]; then
  echo "Not installed yet. Please run 'Install' first."
  read -n 1 -s -r -p "Press any key to close..."
  exit 1
fi

fehler() {
  echo "$1 Log file ($2):"
  tail -n 30 "$2"
  echo ""
  read -n 1 -s -r -p "Press any key to close..."
  exit 1
}

# Zeitstempel der neusten Quelldatei — daran erkennt man veralteten Code.
neuste_quelle() {
  find app static -type f \( -name '*.py' -o -name '*.js' -o -name '*.html' -o -name '*.css' \) \
    -exec stat -f '%m' {} \; 2>/dev/null | sort -rn | head -1
}

# --- Modellserver -----------------------------------------------------------
# Kachelgröße des VAE-Decoders: schreibt der Installer nach Arbeitsspeicher in engine.conf. Ältere Installationen,
# die nur aktualisiert wurden, haben die Zeile nicht; dann hier nach derselben Regel ableiten (8 GB: Rauschen ab 30 s).
if ! grep -q '^VAE_CHUNK=' "$CONF" 2>/dev/null; then
  RAM_GB=$(( $(sysctl -n hw.memsize) / 1073741824 ))
  if   [ "$RAM_GB" -lt 12 ]; then VAE_CHUNK="256"
  elif [ "$RAM_GB" -lt 16 ]; then VAE_CHUNK="512"
  fi
  [ -n "$VAE_CHUNK" ] && echo "VAE_CHUNK=$VAE_CHUNK" >> "$CONF"
fi
SERVER_ARGS=(--models "$MODELS" --max-batch 1)
[ -n "$VAE_CHUNK" ] && SERVER_ARGS+=(--vae-chunk "$VAE_CHUNK")

# Läuft der Synthese-Server noch mit einer anderen Kachelgröße (z. B. nach einem Update), neu starten
if [ -n "$VAE_CHUNK" ] && laeuft "$SYNTH_PORT"; then
  for pid in $(port_pids "$SYNTH_PORT"); do
    if ! ps -o args= -p "$pid" 2>/dev/null | grep -q -- "--vae-chunk $VAE_CHUNK"; then
      echo "Synthesis server runs with the old tile size, restarting."
      kill "$pid" 2>/dev/null
    fi
  done
  for _ in $(seq 1 20); do laeuft "$SYNTH_PORT" || break; sleep 0.5; done
fi

if [ "$MODE" = "split" ]; then
  if laeuft "$LM_PORT"; then
    echo "Language model server is already running (port $LM_PORT)."
  else
    echo "Starting language model server (CPU)..."
    GGML_BACKEND=CPU nohup "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$LM_PORT" \
      "${SERVER_ARGS[@]}" > "$DATA/ace-lm.log" 2>&1 &
    disown
  fi
fi

if laeuft "$SYNTH_PORT"; then
  echo "Synthesis server is already running (port $SYNTH_PORT)."
else
  echo "Starting synthesis server (Metal)..."
  nohup "$ENGINE/build/ace-server" --host 127.0.0.1 --port "$SYNTH_PORT" \
    "${SERVER_ARGS[@]}" > "$DATA/ace-server.log" 2>&1 &
  disown
fi

# --- Web-App ----------------------------------------------------------------
PID=$(lsof -i :"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -1)
if [ -n "$PID" ]; then
  START=$(date -j -f "%a %b %e %T %Y" "$(ps -p "$PID" -o lstart=)" +%s 2>/dev/null)
  NEUSTE=$(neuste_quelle)
  if [ -n "$START" ] && [ -n "$NEUSTE" ] && [ "$NEUSTE" -gt "$START" ]; then
    echo "The code is newer than the running web app, restarting."
    kill $(lsof -i :"$PORT" -sTCP:LISTEN -t) 2>/dev/null
    for _ in $(seq 1 20); do laeuft "$PORT" || break; sleep 0.5; done
    PID=""
  else
    echo "Web app is already running on port $PORT."
  fi
fi
if [ -z "$PID" ]; then
  echo "Starting web app..."
  nohup uv run uvicorn app.main:app --host 127.0.0.1 --port "$PORT" > "$DATA/server.log" 2>&1 &
  disown
fi

# --- Warten, bis alles antwortet --------------------------------------------
for _ in $(seq 1 60); do
  laeuft "$PORT" && laeuft "$SYNTH_PORT" && { [ "$MODE" != "split" ] || laeuft "$LM_PORT"; } && break
  sleep 0.5
done
laeuft "$PORT"       || fehler "The web app did not start in time." "$DATA/server.log"
laeuft "$SYNTH_PORT" || fehler "The synthesis server did not start in time." "$DATA/ace-server.log"
if [ "$MODE" = "split" ]; then
  laeuft "$LM_PORT" || fehler "The language model server did not start in time." "$DATA/ace-lm.log"
fi
echo "All servers are running."

open "http://localhost:$PORT"
echo ""
echo "Music Generator is open at http://localhost:$PORT"
echo "To stop: use 'Music Generator OFF'."
fenster_schliessen
exit 0
