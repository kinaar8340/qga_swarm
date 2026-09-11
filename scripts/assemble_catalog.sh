#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FFMPEG="/usr/bin/ffmpeg"
STEMS=(s2 t2 k2 p2 helicoid catenoid theta0 theta25 theta50)
OUT="$ROOT/output/mp4/assemble_catalog.mp4"
HOLD_S=2
FPS=24

usage() {
  echo "usage: assemble_catalog.sh [--out PATH] DIR [DIR...]" >&2
  exit 2
}

if [[ ! -x "$FFMPEG" ]]; then
  echo "missing $FFMPEG" >&2
  exit 1
fi

dirs=()
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
    --)
      shift
      dirs+=("$@")
      break
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

if [[ ${#dirs[@]} -eq 0 ]]; then
  usage
fi

echo "claims=Software fact  not_a_proof_of=homology remesh / inner_cone film"
echo "catalog cannot prove it captured the occupant"

find_bgra() {
  local stem="$1" d
  for d in "${dirs[@]}"; do
    if [[ -f "$d/${stem}.bgra" ]]; then
      printf '%s\n' "$d/${stem}.bgra"
      return 0
    fi
  done
  return 1
}

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

tmp="$(mktemp -d "${TMPDIR:-/tmp}/qga_swarm_assemble.XXXXXX")"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

declare -a pngs
width=""
height=""
i=0
for stem in "${STEMS[@]}"; do
  bgra="$(find_bgra "$stem" || true)"
  if [[ -z "$bgra" ]]; then
    echo "missing stem $stem.bgra in: ${dirs[*]}" >&2
    exit 1
  fi
  txt="${bgra%.bgra}.txt"
  wh="$(parse_txt "$txt")"
  w="${wh%% *}"
  h="${wh##* }"
  if [[ -z "$width" ]]; then
    width="$w"
    height="$h"
  elif [[ "$w" != "$width" || "$h" != "$height" ]]; then
    echo "size mismatch $stem: ${w}x${h} vs ${width}x${height}" >&2
    exit 1
  fi
  png="$tmp/$(printf '%02d' "$i")_${stem}.png"
  "$FFMPEG" -y -hide_banner -loglevel error \
    -f rawvideo -pix_fmt bgra -s "${w}x${h}" -i "$bgra" \
    -frames:v 1 "$png"
  pngs+=("$png")
  i=$((i + 1))
done

n="${#pngs[@]}"
inputs=()
labels=""
idx=0
for png in "${pngs[@]}"; do
  inputs+=(-framerate "$FPS" -loop 1 -t "$HOLD_S" -i "$png")
  labels+="[${idx}:v]"
  idx=$((idx + 1))
done

mkdir -p "$(dirname "$OUT")"
"$FFMPEG" -y -hide_banner -loglevel error \
  "${inputs[@]}" \
  -filter_complex "${labels}concat=n=${n}:v=1:a=0,fps=${FPS},format=yuv420p" \
  -c:v libx264 \
  "$OUT"
