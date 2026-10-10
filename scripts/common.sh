# Gemeinsame Einstellungen für Install, Start und Stopp. Wird per "source" eingebunden.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE="$ROOT/engine/acestep.cpp"
MODELS="$ENGINE/models"
DATA="$ROOT/data"
CONF="$DATA/engine.conf"          # vom Installer geschrieben: MODE, LM_FILE

PORT=8765         # Web-App
SYNTH_PORT=8085   # ace-server Synthese (Metal)
LM_PORT=8086      # ace-server Sprachmodell (nur im Modus "split", auf CPU)

# Getestete acestep.cpp-Version (voller Commit-Hash); neuere könnten die API ändern.
ACESTEP_REPO="https://github.com/ServeurpersoCom/acestep.cpp"
ACESTEP_REV="b7ba6d9ce2f19ca7627fb8f340032c3ebe8ae253"
# Modelle: feste Revision des Hugging-Face-Repos, Dateien werden nach dem Download per SHA-256 geprüft.
HF_BASE="https://huggingface.co/Serveurperso/ACE-Step-1.5-GGUF/resolve/666ac70204440867d8c01ba4b119cc79c95b370a"
# Datei:Größe in MB:SHA-256
MODEL_FILES="vae-BF16.gguf:340:0599862ac5d15cd308e1d2e368373aea6c02e25ebd1737ad4a4562a0901b0ef8
Qwen3-Embedding-0.6B-Q8_0.gguf:780:972f23255e46adfe744a0eb9a0039f3c63988f65753b0968d776e8b27168c321
acestep-v15-sft-Q8_0.gguf:2550:17f1984e48aaab27b3eb8ccbf0b754a6656e677884c60ea7003845cfc0059b70
acestep-5Hz-lm-0.6B-Q8_0.gguf:710:bdaf9e292d4470f31c19cafeaca1b74936a114667e3a85e5d33b65247e9908ec
acestep-5Hz-lm-1.7B-Q8_0.gguf:1980:726f99a82f050b32ebc5c5e36acaa9d7acd06bfe5e579a62b00e0d0d6d0b7ec6
acestep-5Hz-lm-4B-Q8_0.gguf:4460:972f91147a167f0c041f1b158d67985a82c0f6a852e68cdf70e46030cf08b1bc"

export PATH="$HOME/.local/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

laeuft() { lsof -i :"$1" -sTCP:LISTEN -t >/dev/null 2>&1; }

# MODE=split  -> Sprachmodell auf CPU (Port 8086), Synthese auf Metal (Port 8085)
# MODE=single -> alles auf einem Metal-Server (Port 8085)
lade_conf() {   # engine.conf als key=value lesen, nicht ausführen
  MODE=split
  LM_FILE=""
  [ -f "$CONF" ] || return 0
  MODE=$(sed -n 's/^MODE=//p' "$CONF" | tail -1)
  LM_FILE=$(sed -n 's/^LM_FILE=//p' "$CONF" | tail -1)
  [ -n "$MODE" ] || MODE=split
}

# Terminal-Fenster nach getaner Arbeit selbst schließen (wie die übrigen APPS-Skripte).
fenster_schliessen() {
  local tty_now
  tty_now=$(tty 2>/dev/null) || return 0
  (sleep 1; osascript -e "tell application \"Terminal\" to close (first window whose tty is \"$tty_now\")" >/dev/null 2>&1) &
  disown
}
