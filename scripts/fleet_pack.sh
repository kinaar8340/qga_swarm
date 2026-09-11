#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FLEET="${FLEET:-$HOME/Playground/bin/fleet}"
HOSTS=(bud2 bud3 bud4 bud5 bud6 bud7 bud8 bud9)
MAP=(s2 t2 k2 p2 s2 t2 k2 p2)

local_pack() {
  mkdir -p "$ROOT/results/local"
  for s in s2 t2 k2 p2 helicoid catenoid; do
    python3 "$ROOT/workers/${s}.py" --out "$ROOT/results/local/${s}_edges.bin"
  done
  for spec in theta0:0 theta25:0.25 theta50:0.5; do
    s="${spec%%:*}"
    ph="${spec##*:}"
    python3 "$ROOT/workers/theta.py" --theta "$ph" --out "$ROOT/results/local/${s}_edges.bin"
  done
}

remote_pack() {
  "$FLEET" copy "$ROOT/workers/" /tmp/qga_swarm_workers/
  i=0
  for h in "${HOSTS[@]}"; do
    s="${MAP[$i]}"
    "$FLEET" run --hosts "$h" -- \
      "python3 /tmp/qga_swarm_workers/${s}.py --out /tmp/${s}_edges.bin"
    mkdir -p "$ROOT/results/$h"
    rsync -az "$h:/tmp/${s}_edges.bin" "$ROOT/results/$h/${s}_edges.bin"
    i=$((i + 1))
  done
  # H/C after homology. MAP stays bud2/6 s2 … bud5/9 p2. Theta stays --local.
  "$FLEET" run --hosts bud6 -- \
    "python3 /tmp/qga_swarm_workers/helicoid.py --out /tmp/helicoid_edges.bin"
  mkdir -p "$ROOT/results/bud6"
  rsync -az bud6:/tmp/helicoid_edges.bin "$ROOT/results/bud6/helicoid_edges.bin"
  "$FLEET" run --hosts bud7 -- \
    "python3 /tmp/qga_swarm_workers/catenoid.py --out /tmp/catenoid_edges.bin"
  mkdir -p "$ROOT/results/bud7"
  rsync -az bud7:/tmp/catenoid_edges.bin "$ROOT/results/bud7/catenoid_edges.bin"
}

if [[ "${1:-}" == "--local" ]]; then
  local_pack
else
  remote_pack
fi
