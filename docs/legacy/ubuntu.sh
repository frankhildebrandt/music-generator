#!/usr/bin/env bash
# Music Generator Ubuntu installer. Add to repository root beside arch_linux.sh.
set -Eeuo pipefail
SOURCE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -f "$SOURCE_DIR/scripts/common.sh" ]]; then echo 'ERROR: Put ubuntu.sh in the music-generator repository root.' >&2; exit 1; fi
# shellcheck source=scripts/common.sh
source "$SOURCE_DIR/scripts/common.sh"
cd "$ROOT"
ACTION=install; BACKEND=auto; LM_SIZE=1.7B; XL=0; YES=0; DRY=0; REBUILD=0; BROWSER=1; CPU_FALLBACK=0
usage() { cat <<'HELP'
Music Generator Ubuntu installer
  ./ubuntu.sh [--install-only] [--yes] [--backend auto|cuda|cpu]
              [--lm 0.6B|1.7B|4B] [--xl] [--rebuild] [--cpu-fallback]
              [--dry-run] [--start|--stop|--status] [--no-browser] [--help]
--dry-run plans operations without installing, downloading, deleting or starting anything.
CUDA toolkit/driver must be installed separately (no automatic driver replacement).
HELP
}
while (($#)); do
 case "$1" in
  --yes|-y) YES=1;; --dry-run) DRY=1;; --install-only|--no-start) ACTION=install;;
  --start) ACTION=start;; --stop) ACTION=stop;; --status) ACTION=status;;
  --backend|--lm) [[ $# -ge 2 ]] || { echo "Missing value for $1" >&2; exit 2; }; if [[ $1 == --backend ]]; then BACKEND=$2; else LM_SIZE=$2; fi; shift;;
  --xl) XL=1;; --rebuild) REBUILD=1;; --cpu-fallback) CPU_FALLBACK=1;; --no-browser) BROWSER=0;;
  --help|-h) usage; exit 0;; *) echo "Unknown option: $1" >&2; usage; exit 2;;
 esac; shift
done
[[ $BACKEND == auto || $BACKEND == cuda || $BACKEND == cpu ]] || { echo "Invalid backend: $BACKEND" >&2; exit 2; }
[[ $LM_SIZE == 0.6B || $LM_SIZE == 1.7B || $LM_SIZE == 4B ]] || { echo "Invalid LM size: $LM_SIZE" >&2; exit 2; }
if [[ $ACTION != install ]]; then
 [[ $DRY == 0 ]] || { echo "DRY RUN: would delegate $ACTION to arch_linux.sh (no installation)."; exit 0; }
 opts=("--$ACTION"); [[ $BROWSER == 0 ]] && opts+=(--no-browser)
 # --start is safe after a completed install; otherwise it triggers the Arch installer.
 if [[ $ACTION == start && ( ! -x "$ENGINE/build/ace-server" || ! -f "$CONF" ) ]]; then echo 'Install with ./ubuntu.sh --install-only first.' >&2; exit 1; fi
 exec bash "$ROOT/arch_linux.sh" "${opts[@]}"
fi
[[ $(uname -s) == Linux ]] || { echo 'Linux required.' >&2; exit 1; }
# Permit Ubuntu and Ubuntu derivatives; reject Arch to prevent wrong package manager.
. /etc/os-release
[[ ${ID:-} == ubuntu || " ${ID_LIKE:-} " == *' ubuntu '* || " ${ID_LIKE:-} " == *' debian '* ]] || { echo "Ubuntu or Ubuntu-derived OS required: ${ID:-unknown}" >&2; exit 1; }
command -v apt-get >/dev/null || { echo 'apt-get is missing.' >&2; exit 1; }
[[ $(uname -m) == x86_64 ]] || { echo 'This installer supports x86_64 only.' >&2; exit 1; }
export PATH="/usr/local/cuda/bin:$PATH"
if [[ $BACKEND == auto ]]; then
 if command -v nvcc >/dev/null 2>&1 && command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi -L >/dev/null 2>&1; then BACKEND=cuda; else BACKEND=cpu; fi
fi
if [[ $BACKEND == cuda ]]; then
 command -v nvcc >/dev/null 2>&1 || { echo 'CUDA toolkit (nvcc) missing. Install a compatible toolkit first or use --backend cpu.' >&2; exit 1; }
 command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi -L >/dev/null 2>&1 || { echo 'NVIDIA driver/GPU unavailable.' >&2; exit 1; }
fi
LM_FILE="acestep-5Hz-lm-${LM_SIZE}-Q8_0.gguf"
echo "Music Generator Ubuntu setup | ${PRETTY_NAME:-$ID} | backend=$BACKEND | LM=$LM_SIZE"
packages=(build-essential cmake ninja-build git curl pkg-config libcurl4-openssl-dev lsof xdg-utils ca-certificates ccache)
if [[ $DRY == 1 ]]; then
 echo 'DRY RUN: no side effects.'
 printf 'PLAN apt-get install: '; printf '%s ' "${packages[@]}"; echo
 echo 'PLAN uv sync --locked'
 echo "PLAN git checkout $ACESTEP_REV and git submodule update --init --recursive"
 if [[ $BACKEND == cuda ]]; then echo 'PLAN CMake: -DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES=native -DGGML_CPU_ALL_VARIANTS=OFF'; else echo 'PLAN CMake: -DGGML_CPU_ALL_VARIANTS=OFF'; fi
 echo "PLAN download + SHA256 verify: VAE, Qwen embedding, SFT, $LM_FILE $([[ $XL == 1 ]] && echo 'and XL' || true)"
 echo 'PLAN run local inference self-test and atomically write data/engine.conf'
 exit 0
fi
if [[ $YES != 1 ]]; then read -r -p 'Install Ubuntu dependencies and continue? [Y/n] ' ans; [[ $ans == '' || $ans == [Yy]* ]] || exit 1; fi
sudo apt-get update
sudo apt-get install -y "${packages[@]}"
if ! command -v uv >/dev/null 2>&1; then
 echo 'Installing uv via official Astral installer (https://astral.sh/uv/install.sh).'
 uv_installer=$(mktemp)
 curl -fsSL --retry 3 https://astral.sh/uv/install.sh -o "$uv_installer"
 sh "$uv_installer"
 rm -f "$uv_installer"
 export PATH="$HOME/.local/bin:$PATH"
 command -v uv >/dev/null 2>&1 || { echo 'uv installation failed.' >&2; exit 1; }
fi
uv sync --locked
mkdir -p "$ROOT/engine" "$DATA"
if [[ ! -d "$ENGINE/.git" ]]; then
 [[ ! -e "$ENGINE" ]] || { echo "Engine directory exists but is not a git clone: $ENGINE" >&2; exit 1; }
 git clone "$ACESTEP_REPO" "$ENGINE"
fi
( cd "$ENGINE"; git cat-file -e "$ACESTEP_REV^{commit}" 2>/dev/null || git fetch origin; git checkout --detach "$ACESTEP_REV"; git submodule update --init --recursive )
BUILD=0
[[ $REBUILD == 1 || ! -x "$ENGINE/build/ace-server" ]] && BUILD=1
# Never reuse a previous CPU build when CUDA was selected (or vice versa).
if [[ -f "$ENGINE/build/CMakeCache.txt" ]]; then
 if [[ $BACKEND == cuda ]] && ! grep -qx "GGML_CUDA:BOOL=ON" "$ENGINE/build/CMakeCache.txt"; then BUILD=1; fi
 if [[ $BACKEND == cpu ]] && grep -qx "GGML_CUDA:BOOL=ON" "$ENGINE/build/CMakeCache.txt"; then BUILD=1; fi
else BUILD=1; fi
if [[ $BUILD == 1 ]]; then
 flags=(-DCMAKE_BUILD_TYPE=Release -DGGML_CPU_ALL_VARIANTS=OFF)
 if [[ $BACKEND == cuda ]]; then flags+=(-DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES=native); fi
 build_engine() {
  local target=$1; local -a f=(-DCMAKE_BUILD_TYPE=Release -DGGML_CPU_ALL_VARIANTS=OFF)
  if [[ $target == cuda ]]; then f+=(-DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES=native); fi
  ( cd "$ENGINE"; rm -rf build; cmake -S . -B build "${f[@]}"; cmake --build build --config Release -j "${BUILD_JOBS:-4}" ) > "$DATA/build.log" 2>&1
 }
 if ! build_engine "$BACKEND"; then
  if [[ $BACKEND == cuda && $CPU_FALLBACK == 1 ]]; then echo 'CUDA build failed; explicit CPU fallback requested.'; BACKEND=cpu; build_engine cpu || { echo 'CPU build failed: data/build.log' >&2; exit 1; }
  else echo 'Build failed: data/build.log (no silent CUDA-to-CPU fallback).' >&2; exit 1; fi
 fi
fi
[[ -x "$ENGINE/build/ace-server" ]] || { echo 'ace-server executable missing after build.' >&2; exit 1; }
mkdir -p "$MODELS"
files=(vae-BF16.gguf Qwen3-Embedding-0.6B-Q8_0.gguf acestep-v15-sft-Q8_0.gguf "$LM_FILE")
[[ $XL == 1 ]] && files+=(acestep-v15-xl-sft-Q8_0.gguf)
for f in "${files[@]}"; do
 record=$(printf '%s\n' "$MODEL_FILES" | grep -F "$f:" | head -n 1)
 [[ -n $record ]] || { echo "No pinned checksum for $f" >&2; exit 1; }
 IFS=: read -r _ mb sha <<< "$record"
 if [[ -f "$MODELS/$f" ]]; then
  if printf '%s  %s\n' "$sha" "$MODELS/$f" | sha256sum -c --status; then echo "Verified existing $f"; continue; fi
  echo "Existing $f has invalid SHA256. Not overwriting; move it aside manually." >&2; exit 1
 fi
 echo "Downloading $f ($mb MB)"
 curl -fL -C - --retry 3 -o "$MODELS/$f.part" "$HF_BASE/$f"
 if ! printf '%s  %s\n' "$sha" "$MODELS/$f.part" | sha256sum -c --status; then rm -f "$MODELS/$f.part"; echo "Checksum mismatch: $f" >&2; exit 1; fi
 mv "$MODELS/$f.part" "$MODELS/$f"
done
# Keep original project's single/split mode decision via its own self-test.
MODE=single
TEST_PORT=8099
if laeuft "$TEST_PORT"; then echo 'Self-test port 8099 occupied; refusing to overwrite configuration.' >&2; exit 1; fi
"$ENGINE/build/ace-server" --host 127.0.0.1 --port "$TEST_PORT" --models "$MODELS" --max-batch 1 > "$DATA/selftest.log" 2>&1 & test_pid=$!
cleanup() { kill "$test_pid" 2>/dev/null || true; wait "$test_pid" 2>/dev/null || true; }
trap cleanup EXIT
ready=0
for ((i=0;i<120;i++)); do if ! kill -0 "$test_pid" 2>/dev/null; then break; fi; if curl -fsS --max-time 1 "http://127.0.0.1:$TEST_PORT/health" >/dev/null 2>&1; then ready=1; break; fi; sleep .5; done
[[ $ready == 1 ]] || { echo 'Server health check failed; see data/selftest.log' >&2; exit 1; }
if ! uv run python scripts/metal_selftest.py "$TEST_PORT" "$LM_FILE"; then MODE=split; echo 'Single-mode inference failed; selected split mode.'; fi
cleanup; trap - EXIT
OLD_DEVICE=""; [[ -f "$CONF" ]] && OLD_DEVICE=$(sed -n 's/^LM_DEVICE=//p' "$CONF" | tail -1)   # Wahl aus den Einstellungen (Sprachmodell nur auf der CPU) bleibt erhalten
conf_tmp=$(mktemp "$DATA/engine.conf.XXXXXX")
printf 'MODE=%s\nLM_FILE=%s\nTESTED_REV=%s\nVAE_CHUNK=\nBACKEND=%s\n' "$MODE" "$LM_FILE" "$ACESTEP_REV" "$BACKEND" > "$conf_tmp"
[[ "$OLD_DEVICE" == cpu ]] && echo "LM_DEVICE=cpu" >> "$conf_tmp"
mv "$conf_tmp" "$CONF"
echo "Installation finished: backend=$BACKEND mode=$MODE. Start using ./ubuntu.sh --start"
