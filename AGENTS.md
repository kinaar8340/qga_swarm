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

- qga_gpu@90aa7fc (`update_line_verts`; old `b9c9994`)
- qga_engine@7e7866b
- flux_hopf_lib 0.3.1
- shellscan recipes: read-only dumps under docs/recipe-scores/

Pin 90aa7fc exists for update_line_verts (mixed LineVert color). Old sha b9c9994
already had update_line_segments / draw_geodesic_orb / write_particles / upload_hubs.
Do not float main. No v0.1.0 tag.

A new qga_gpu version is allowed only if the plan names:
  old sha, new sha, why update_line_verts / draw_geodesic_orb need it.
inner_cone + qga_engine stay on b9c9994; this sidecar does not move them.
Cargo.lock is not the contract. Workspace rev is.

## Last mile (bud only)

Allowed GPU calls:

- Renderer::update_line_segments — S² T² K² P² wires, helicoid / catenoid rulings
- Renderer::update_line_verts — mixed four-bin catalog edges (pin 90aa7fc)
- Renderer::draw_geodesic_orb / _alpha — hubs only (catalog mesh; α = LOD or fade)
- Renderer::upload_hubs — Clock B breath on pentavalent sites, not a second net
- Renderer::write_hud — cards + schematic Venn. Plate sentence, never a capture proof
- Renderer::write_particles — CPU ruling motes constructed on bud. Cap 512.
  pad = SPEC four-bin hue (0.55/0.10/0.30/0.80). Life/flux, not the occupant
- Renderer::render(..., capture=true) — frames for ffmpeg

Forbidden:

- write_particles from recipe fields or qga_pixel dumps (do not alias the 32 B chart cut)
- the 65 536-particle ocean demo path
- retain_static_fibers as a place to hide occupancy paint
- binding any output to γ(s) / flux_trajectoid faceplate
- make scan (frozen)
- occupancy-as-capsomer shading
- a fifth hue
- calling inner_cone as if it ran in the film
- treating lod as geodesic curvature
- empty update_orb_instances as a disappearing occupant (that writes identity)

## Record split (same width, different algebra)

- qga_pixel 32 B  — chart cut (shellscan). Workers may emit these as files.
- GpuParticle 32 B — pos, mass, vel, pad. Do not alias qga_pixel onto this.
- GpuOrbInstance 32 B — hubs.
- FrameUniforms 256 B — written on bud per frame. Never shipped over SSH.

Conversion happens on bud in this repo (convert.rs or scripts/convert.py).
Workers never open a GpuContext.

## Claims

- Wires + hubs through qga_gpu: Software fact of the upload path.
- T=3 / T=7 / larva paints: Model + Software fact of shellscan dumps (read only).
- Occupancy scores (1.0 vs ~1/6, setal vs band): Hypothesis on the HUD card.
- Cylinder isoline: Hypothesis (bounded). Do not restage it here.
- Helicoid / catenoid lens: Model (arena split). Surfaces illustrate;
  they are not the configuration space. Mid-plane orb is ∂p/∂t on the
  workers/theta.py chart, not a geodesic ODE in qga_gpu.
- One-sheet hyperboloid cage: Model. Not a third minimal surface.
- CPU motes along rulings: Model. Display only. Not the occupant.
- S² T² K² P² prologue: homology legend, not a remesh onto Goldberg faces.
- Goldberg nets in lepidopteran development: not a claim. Recipe ≠ morphogenesis.

Binaries print claims=Software fact.

## Plan → approve → build

Grok Build must start in plan mode for every non-trivial change.
Write tools stay off until the human types APPROVE (or presses `a`
in the plan viewer). Headless `grok -p` on bud is for facts and
status, not for editing this tree. Workers are signed in.
`bin/fleet grok` is Track 3 (facts/status). Do not mix grok into
`fleet_pack.sh`. Last mile stays on bud.

See BUILD.md.
