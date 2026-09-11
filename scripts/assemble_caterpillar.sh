#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FFMPEG="/usr/bin/ffmpeg"
OUT="$ROOT/output/mp4/caterpillar_topology.mp4"
FPS=24

usage() {
  echo "usage: assemble_caterpillar.sh [--out PATH] DIR" >&2
  exit 2
}

if [[ ! -x "$FFMPEG" ]]; then
  echo "missing $FFMPEG" >&2
  exit 1
fi

DIR=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      [[ $# -ge 2 ]] || usage
      OUT="$2"
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
      DIR="$1"
      shift
      ;;
  esac
done

if [[ -z "$DIR" ]]; then
  usage
fi

echo "claims=Software fact  not_a_proof_of=homology remesh / inner_cone film"
echo "catalog cannot prove it captured the occupant"

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

shopt -s nullglob
frames=("$DIR"/frame_*.bgra)
if [[ ${#frames[@]} -eq 0 ]]; then
  echo "no frame_*.bgra in $DIR" >&2
  exit 1
fi
IFS=$'\n' frames=($(printf '%s\n' "${frames[@]}" | sort))
unset IFS

first="${frames[0]}"
txt="${first%.bgra}.txt"
wh="$(parse_txt "$txt")"
width="${wh%% *}"
height="${wh##* }"

for bgra in "${frames[@]}"; do
  t="${bgra%.bgra}.txt"
  wh="$(parse_txt "$t")"
  w="${wh%% *}"
  h="${wh##* }"
  if [[ "$w" != "$width" || "$h" != "$height" ]]; then
    echo "size mismatch $bgra: ${w}x${h} vs ${width}x${height}" >&2
    exit 1
  fi
done

mkdir -p "$(dirname "$OUT")"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/qga_swarm_caterpillar.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

i=0
for bgra in "${frames[@]}"; do
  png="$tmp/$(printf '%04d' "$i").png"
  "$FFMPEG" -y -hide_banner -loglevel error \
    -f rawvideo -pix_fmt bgra -s "${width}x${height}" -i "$bgra" \
    -frames:v 1 "$png"
  i=$((i + 1))
done

"$FFMPEG" -y -hide_banner -loglevel error \
  -framerate "$FPS" -i "$tmp/%04d.png" \
  -c:v libx264 -pix_fmt yuv420p \
  "$OUT"

echo "wrote $OUT  frames=$i  ${width}x${height}"
