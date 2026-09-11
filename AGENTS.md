# qga_swarm

Fleet sidecar that feeds wire parcels into qga_gpu on bud.
Not a second engine. Not Animation A. Not a Renderer trait.

## Topology

- Parent: bud (4090). Only host allowed to construct Renderer.
- Workers: bud2–bud9. CPU parcels only. No wgpu, no CUDA, no qga_gpu binary.
- Driver: ~/Playground/bin/fleet (model B). Do not spawn local subagents
  that SSH. Do not mix model A with fleet jobs.
Playground tracks: 6 + 0 + 3 (worker auth present). Not 9.

## Pins (do not float main)

- qga_gpu@b9c9994 until a planned bump is APPROVED
- qga_engine@7e7866b
- flux_hopf_lib 0.3.1
- shellscan recipes: read-only dumps under docs/recipe-scores/

A new qga_gpu version is allowed only if the plan names:
  old sha, new sha, why update_line_segments / draw_geodesic_orb need it,
  and that inner_cone + qga_engine pins move together.
Cargo.lock is not the contract. Workspace rev is.

## Last mile (bud only)

Allowed GPU calls:

- Renderer::update_line_segments  — S² T² K² P² wires, helicoid / catenoid rulings
- Renderer::draw_geodesic_orb / _alpha — hubs only
- Renderer::write_hud — cards
- Renderer::render(..., capture=true) — frames for ffmpeg

Forbidden:

- write_particles from recipe fields or qga_pixel dumps
- the 65 536-particle ocean demo path
- retain_static_fibers as a place to hide occupancy paint
- binding any output to γ(s) / flux_trajectoid faceplate
- make scan (frozen)
- occupancy-as-capsomer shading
- a fifth hue
- calling inner_cone as if it ran in the film

## Record split (same width, different algebra)

- qga_pixel 32 B  — chart cut (shellscan). Workers may emit these as files.
- GpuParticle 32 B — pos, mass, vel, pad. Do not alias qga_pixel onto this.
- GpuOrbInstance 32 B — hubs.
- FrameUniforms 256 B — written on bud per frame. Never shipped over SSH.

Conversion happens on bud in this repo (convert.rs or scripts/convert.py).
Workers never open a GpuContext.

## Claims

- Wires + hubs through qga_gpu: Software fact of the upload path.
- T=3 / T=7 paints: Model + Software fact of shellscan dumps (read only).
- Cylinder isoline: Hypothesis (bounded). Do not restage it here.
- Helicoid / catenoid lens: Model (arena split). Surfaces illustrate;
  they are not the configuration space.
- S² T² K² P² prologue: homology legend, not a remesh onto Goldberg faces.

Binaries print claims=Software fact.

## Plan → approve → build

Grok Build must start in plan mode for every non-trivial change.
Write tools stay off until the human types APPROVE (or presses `a`
in the plan viewer). Headless `grok -p` on bud is for facts and
status, not for editing this tree. Workers are signed in.
`bin/fleet grok` is Track 3 (facts/status). Do not mix grok into
`fleet_pack.sh`. Last mile stays on bud.

See BUILD.md.
