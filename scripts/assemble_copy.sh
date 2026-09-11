#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FFMPEG="/usr/bin/ffmpeg"
OUT="$ROOT/output/mp4/caterpillar_copy.mp4"
FPS=24
HOLD_S=1

usage() {
  echo "usage: assemble_copy.sh [--out PATH] GROW_DIR SKIN_DIR" >&2
  exit 2
}

if [[ ! -x "$FFMPEG" ]]; then
  echo "missing $FFMPEG" >&2
  exit 1
fi

OUT_SET=0
dirs=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      [[ $# -ge 2 ]] || usage
      OUT="$2"
      OUT_SET=1
      shift 2
      ;;
    -h|--help)
      usage
      ;;
    -*)
      echo "unknown arg $1" >&2
      usage
      ;;
    *)
      dirs+=("$1")
      shift
      ;;
  esac
done

if [[ ${#dirs[@]} -ne 2 ]]; then
  usage
fi

GROW="${dirs[0]}"
SKIN="${dirs[1]}"

echo "claims=Software fact  not_a_proof_of=homology remesh / inner_cone film"
echo "catalog cannot prove it captured the occupant"
echo "skeleton then skin; paint does not grow"

parse_txt() {
  local txt="$1"
  local line w h n
  [[ -f "$txt" ]] || { echo "missing $txt" >&2; return 1; }
  line="$(tr -d '\r' <"$txt" | head -n 1)"
  if [[ "$line" =~ ^bgra[[:space:]]+([0-9]+)x([0-9]+)[[:space:]]+bytes=([0-9]+) ]]; then
    w="${BASH_REMATCH[1]}"
    h="${BASH_REMATCH[2]}"
    n="${BASH_REMATCH[3]}"
  else
    echo "bad txt $txt: $line" >&2
    return 1
  fi
  if [[ "$n" -ne $((w * h * 4)) ]]; then
    echo "stride mismatch $txt: bytes=$n want=$((w * h * 4))" >&2
    return 1
  fi
  printf '%s %s\n' "$w" "$h"
}

collect() {
  local dir="$1"
  shopt -s nullglob
  local frames=("$dir"/frame_*.bgra)
  if [[ ${#frames[@]} -eq 0 ]]; then
    echo "no frame_*.bgra in $dir" >&2
    exit 1
  fi
  IFS=$'\n' frames=($(printf '%s\n' "${frames[@]}" | sort))
  unset IFS
  printf '%s\n' "${frames[@]}"
}

grow_frames=()
while IFS= read -r line; do
  grow_frames+=("$line")
done < <(collect "$GROW")
skin_frames=()
while IFS= read -r line; do
  skin_frames+=("$line")
done < <(collect "$SKIN")

wh="$(parse_txt "${grow_frames[0]%.bgra}.txt")"
width="${wh%% *}"
height="${wh##* }"

tmp="$(mktemp -d "${TMPDIR:-/tmp}/qga_swarm_copy.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

i=0
dump() {
  local bgra="$1"
  local t="${bgra%.bgra}.txt"
  parse_txt "$t" >/dev/null
  local png="$tmp/$(printf '%05d' "$i").png"
  "$FFMPEG" -y -hide_banner -loglevel error \
    -f rawvideo -pix_fmt bgra -s "${width}x${height}" -i "$bgra" \
    -frames:v 1 "$png"
  i=$((i + 1))
}

for bgra in "${grow_frames[@]}"; do
  dump "$bgra"
done
hold=$((HOLD_S * FPS))
last="${grow_frames[-1]}"
for _ in $(seq 1 "$hold"); do
  dump "$last"
done
for bgra in "${skin_frames[@]}"; do
  dump "$bgra"
done

mkdir -p "$(dirname "$OUT")"
"$FFMPEG" -y -hide_banner -loglevel error \
  -framerate "$FPS" -i "$tmp/%05d.png" \
  -c:v libx264 -pix_fmt yuv420p \
  "$OUT"

echo "wrote $OUT  frames=$i  ${width}x${height}  hold=${HOLD_S}s"
