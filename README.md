# qga_swarm

Assemble homology wires and catalog dumps into one captured MP4.
Last mile is qga_gpu on bud. Workers emit files.

Playground tracks: 6 (GPU parent / CPU workers) + 0 (bin/fleet).
Not Track 3 until workers have auth. Not Track 9.

Life on the helicoid, catalog on the catenoid. The catalog cannot
prove it captured the occupant. This repo does not prove it either.

## What you already shipped (do not rebuild)

- shellscan make film          → t3_field_strip.mp4
- shellscan make film-cylinder → cylinder_isoline.mp4
- inner_cone owns the observer
- qga_gpu owns the frame (pin b9c9994)

This repo consumes those dumps and four CPU wire generators.

## Beat sheet (assemble_catalog.mp4)

1. Generators — S² T² K² P² wires (line_segments)
2. Catalog    — S² remains; T=3 occupant triangle is cards + hubs,
                not ocean particles
3. Lens       — helicoid / catenoid rulings, mid-plane orb as ∂p/∂t
4. Refuse     — Hypothesis/Model card. Faceplate unused.

## Layout

    pins.toml
    crates/qga-swarm-convert/
    crates/qga-swarm-preview/
    workers/{edges,s2,t2,k2,p2}.py
    scripts/fleet_pack.sh
    output/mp4/

## Run

    python3 workers/s2.py --out /tmp/s2_edges.bin
    ./scripts/fleet_pack.sh --local
    cargo test -p qga-swarm-convert
    cargo check -p qga-swarm-preview
    cargo run -p qga-swarm-preview -- --headless --frames 8 --lines results/local/s2_edges.bin
    cargo run -p qga-swarm-preview -- --headless --frames 8 \
      --s2 results/local/s2_edges.bin \
      --t2 results/local/t2_edges.bin \
      --k2 results/local/k2_edges.bin \
      --p2 results/local/p2_edges.bin \
      --capture /tmp/qga_swarm_four
    python3 workers/helicoid.py --out /tmp/helicoid_edges.bin
    python3 workers/catenoid.py --out /tmp/catenoid_edges.bin
    cargo run -p qga-swarm-preview -- --headless --frames 8 \
      --helicoid results/local/helicoid_edges.bin \
      --catenoid results/local/catenoid_edges.bin \
      --capture /tmp/qga_swarm_hc

Four named flags → `{s2,t2,k2,p2}.bgra` under `--capture`. `--helicoid`/`--catenoid` → `{helicoid,catenoid}.bgra`. `--lines` alone → `last.bgra` (cyan unless the stem is s2|t2|k2|p2|helicoid|catenoid). `--local` writes homology plus H/C QGAE.

    cargo run -p qga-swarm-preview -- --headless --frames 8 \
      --s2 results/local/s2_edges.bin \
      --t2 results/local/t2_edges.bin \
      --k2 results/local/k2_edges.bin \
      --p2 results/local/p2_edges.bin \
      --helicoid results/local/helicoid_edges.bin \
      --catenoid results/local/catenoid_edges.bin \
      --capture /tmp/qga_swarm_cat
    ./scripts/assemble_catalog.sh --out output/mp4/assemble_catalog.mp4 \
      /tmp/qga_swarm_cat

    python3 workers/theta.py --theta 0 --out /tmp/theta0_edges.bin
    python3 workers/theta.py --theta 0.25 --out /tmp/theta25_edges.bin
    python3 workers/theta.py --theta 0.5 --out /tmp/theta50_edges.bin
    cargo run -p qga-swarm-preview -- --headless --frames 8 \
      --theta results/local/theta0_edges.bin \
      --theta results/local/theta25_edges.bin \
      --theta results/local/theta50_edges.bin \
      --capture /tmp/qga_swarm_theta

    ./scripts/fleet_pack.sh

Remote homology MAP is unchanged (bud2/6 s2, bud3/7 t2, bud4/8 k2, bud5/9 p2). Remote H/C: bud6 `helicoid_edges.bin`, bud7 `catenoid_edges.bin`. Theta stays `--local`. Never `bin/fleet grok`.

θ family is Model; conjugate helicoid is not `helicoid.py`. Do not add theta stems to `assemble_catalog.sh` this pass. Sidecar MP4 is not occupant proof. Pin is `pins.toml` (`qga_gpu@b9c9994`). Never path-dep `~/Projects/qga_gpu`.
