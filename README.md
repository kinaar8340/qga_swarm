# qga_swarm

Assemble homology wires and catalog dumps into one captured MP4.
Last mile is qga_gpu on bud. Workers emit files.

Playground tracks: 6 (GPU parent / CPU workers) + 0 (bin/fleet) + 3 (headless grok; workers have auth).
Not Track 9.

Life on the helicoid, catalog on the catenoid. The catalog cannot
prove it captured the occupant. This repo does not prove it either.

## What you already shipped (do not rebuild)

- shellscan make film          → t3_field_strip.mp4
- shellscan make film-cylinder → cylinder_isoline.mp4
- inner_cone owns the observer
- qga_gpu owns the frame (pin 90aa7fc)

This repo consumes those dumps and four CPU wire generators.

## Beat sheet (assemble_catalog.mp4 stills)

1. Generators — S² T² K² P² wires (line_segments)
2. Catalog    — S² remains; occupant is cards + hubs, not ocean particles
3. Lens       — helicoid / catenoid rulings, mid-plane orb as ∂p/∂t
4. Refuse     — Hypothesis/Model card. Faceplate unused.

## Beat sheet (caterpillar_topology.mp4)

Same last mile. Occupant retargeted at larva dumps. 48 frames, one Renderer.

1. Generators — four homology wires appear
2. Catalog    — `banded-larva` net; 12 pentavalent geodesic hubs; hexavalent empty
3. Lens       — helicoid / catenoid, mid-plane orb as ∂p/∂t on `workers/theta.py`
4. Cage       — one-sheet hyperboloid rulings (Model, not a third minimal surface)
5. Life       — ≤512 CPU motes along rulings; `write_particles` is display
6. Refuse     — HUD Venn + occupancy card. Catalog cannot prove it captured the occupant

`--beat caterpillar` captures every frame as `frame_XXXX.bgra`. Without `--beat`
the old per-stem still path is unchanged (`assemble_catalog.sh`).

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
      /tmp/qga_swarm_cat /tmp/qga_swarm_theta

    python3 workers/hyperboloid.py --out results/local/hyperboloid_edges.bin
    cargo run -p qga-swarm-preview -- --headless --frames 48 \
      --beat caterpillar \
      --s2 results/local/s2_edges.bin \
      --t2 results/local/t2_edges.bin \
      --k2 results/local/k2_edges.bin \
      --p2 results/local/p2_edges.bin \
      --helicoid results/local/helicoid_edges.bin \
      --catenoid results/local/catenoid_edges.bin \
      --hyperboloid results/local/hyperboloid_edges.bin \
      --width 1920 --height 1080 \
      --field "$HOME/Projects/shellscan/output/recipe/banded-larva" \
      --compare "$HOME/Projects/shellscan/output/recipe/compare_capsid-t7-p22_capsid-t7-polyoma.json" \
      --capture output/mp4/caterpillar_topology

`--larva PATH` is an alias for `--field`. PATH may be the recipe dir or `net.json`.
`--width` / `--height` default 1920×1080. Homology / hyperboloid parcels are optional;
missing bins stay empty instead of failing closed.

`--frames 240` is 10 s at 24 fps (lens ≈ 2.5 s of associate-family rebuild).
48 frames is two seconds; the same loop, not a different film.
    ./scripts/assemble_caterpillar.sh --out output/mp4/caterpillar_topology.mp4 \
      output/mp4/caterpillar_topology

    python3 workers/theta.py --theta 0 --out /tmp/theta0_edges.bin
    python3 workers/theta.py --theta 0.25 --out /tmp/theta25_edges.bin
    python3 workers/theta.py --theta 0.5 --out /tmp/theta50_edges.bin
    cargo run -p qga-swarm-preview -- --headless --frames 8 \
      --theta results/local/theta0_edges.bin \
      --theta results/local/theta25_edges.bin \
      --theta results/local/theta50_edges.bin \
      --capture /tmp/qga_swarm_theta

    ./scripts/fleet_pack.sh

Remote homology MAP is unchanged (bud2/6 s2, bud3/7 t2, bud4/8 k2, bud5/9 p2). Remote H/C: bud6 `helicoid_edges.bin`, bud7 `catenoid_edges.bin`. Theta stays `--local`. `fleet_pack.sh` never calls grok.

    ~/Playground/bin/fleet grok --hosts bud2 --no-subagents -p 'reply with only the hostname'

Track 3 is `bin/fleet grok` (facts/status). Last mile stays on bud.

θ family is Model; conjugate helicoid is not `helicoid.py`. Catalog stems are `s2 t2 k2 p2 helicoid catenoid theta0 theta25 theta50` (fail-closed). HUD is catalog + refuse cards (`write_hud`), not ocean particles and not occupant proof. Pin is `pins.toml` (`qga_gpu@90aa7fc`; old `b9c9994` lacked mixed `update_line_verts`). inner_cone / qga_engine stay on `b9c9994`. Never path-dep `~/Projects/qga_gpu`. Hyperboloid cage is Model. Recipe ≠ morphogenesis.
