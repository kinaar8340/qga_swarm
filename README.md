# qga_swarm

Assemble homology wires and catalog dumps into one captured MP4.
Last mile is qga_gpu on bud. Workers emit files.

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

## Run

    # workers
    ~/Playground/bin/fleet ping
    ./scripts/fleet_pack.sh

    # bud
    cargo run -p qga-swarm-preview --release -- --capture output/mp4/
