# Plan first. Build after APPROVE.

CLI on this box is Grok 1.0.5. Complex work uses Plan Mode.
https://x.ai/docs/build/cli/reference
https://x.ai/news/grok-build-cli

## Session on bud

    cd ~/Playground/qga_swarm
    grok inspect          # confirm AGENTS.md + skills picked up
    /plan                 # or Shift+Tab onto Plan
    # then the goal, one paragraph

or one shot:

    grok plan "…"

Plan mode may write only the session plan file. Edits to any other
path must fail until approval.

## Plan quality bar (reject otherwise)

- Files touched as paths, not vibes
- Order of operations (what breaks if reordered)
- Risks + rollback (git commit first)
- Test command that will actually run
- Out of scope named
- Which hosts run which step (bud vs budN)
- Whether qga_gpu pin stays b9c9994 or a new sha is required

## First approved goal (when you say APPROVE)

Scaffold only:

    qga_swarm/
      AGENTS.md BUILD.md README.md
      crates/qga-swarm-convert/   # pixel/edge files → GpuHub + line pairs
      crates/qga-swarm-preview/   # bud binary: line_segments + orbs + capture
      workers/s2.py t2.py k2.py p2.py   # CPU polylines, no GPU
      scripts/fleet_pack.sh       # bin/fleet run + rsync results/
      output/mp4/                 # gitignored
      pins.toml                   # qga_gpu / qga_engine shas

Out of scope for pass 1: ocean demo, make scan, γ(s), inner_cone
binary, worker Grok login, bumping qga_gpu.

## After APPROVE

    grok apply            # if you used `grok plan`
    # or press `a` in the TUI plan viewer

Every change is a diff. Do not --always-approve on this repo.
Test command:

    cargo check -p qga-swarm-preview
    python3 workers/s2.py --out /tmp/s2_edges.bin
    # GPU smoke on bud only:
    cargo run -p qga-swarm-preview -- --headless --frames 8 \
        --lines /tmp/s2_edges.bin

## Fleet pack (model B, no Grok on workers)

    bin/fleet ping
    ./scripts/fleet_pack.sh     # SSH generators, rsync results/$host/
    # then preview on bud

## If the last mile needs a newer qga_gpu

New plan. Must list the methods you need that b9c9994 already has
(update_line_segments, draw_geodesic_orb) versus a real gap.
If b9c9994 already exposes them, do not bump. If you bump:

    pins.toml qga_gpu = <new>
    document inner_cone + qga_engine pin move
    no v0.1.0 tag invented from a visuals sha
