//! Last mile on bud. One QGAE, four-bin paint, one BGRA.
//! Occupant = cards + hubs. Motes are flux, not the catalog.

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use qga_gpu::{
    hud_quad, hud_text, print_claim_banner, Camera, GpuContext, GpuParticle, HudVert, LineStyle,
    LineVert, Renderer, VisualState,
};
use qga_swarm_convert::{
    catalog_line_verts, face_centroids, glam_edges, hexavalent_hubs, load_chaetotaxy,
    load_group_row, load_net_json, load_occupancy, load_qga_pixel_field, load_qgae,
    load_setal_sites, nearest_section, pentavalent_hubs, CatalogSeg, ChaetaSite, Chaetotaxy,
    GroupRms, Hub, OccupancyCard, PixelFace, SetalSite, SECTION_HUE, SECTION_RGBA, SPECIES_RGBA,
};
use std::path::{Path, PathBuf};

const CYAN: Vec3 = Vec3::new(0.20, 0.60, 1.00);
const GOLD: Vec3 = Vec3::new(1.00, 0.75, 0.20);
const ORANGE: Vec3 = Vec3::new(1.00, 0.40, 0.20);
const MAGENTA: Vec3 = Vec3::new(1.00, 0.20, 0.80);

const C_ASSOC: f32 = 0.35;
const MOTE_CAP: usize = 512;
const POLY_ALPHA: f32 = 1.0 / 6.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bin {
    S2,
    T2,
    K2,
    P2,
    Helicoid,
    Catenoid,
    Theta,
    Hyperboloid,
}

impl Bin {
    fn rgb(self) -> Vec3 {
        match self {
            Bin::S2 | Bin::Helicoid => CYAN,
            Bin::T2 | Bin::Theta => GOLD,
            Bin::K2 | Bin::Catenoid => ORANGE,
            Bin::P2 => MAGENTA,
            Bin::Hyperboloid => GOLD,
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Bin::S2 => "s2",
            Bin::T2 => "t2",
            Bin::K2 => "k2",
            Bin::P2 => "p2",
            Bin::Helicoid => "helicoid",
            Bin::Catenoid => "catenoid",
            Bin::Theta => "theta",
            Bin::Hyperboloid => "hyperboloid",
        }
    }

    fn from_stem(name: &str) -> Option<Self> {
        let s = name.to_ascii_lowercase();
        if s.starts_with("theta") {
            Some(Bin::Theta)
        } else if s.starts_with("helicoid") {
            Some(Bin::Helicoid)
        } else if s.starts_with("catenoid") {
            Some(Bin::Catenoid)
        } else if s.starts_with("hyperboloid") {
            Some(Bin::Hyperboloid)
        } else if s.starts_with("s2") {
            Some(Bin::S2)
        } else if s.starts_with("t2") {
            Some(Bin::T2)
        } else if s.starts_with("k2") {
            Some(Bin::K2)
        } else if s.starts_with("p2") {
            Some(Bin::P2)
        } else {
            None
        }
    }
}

struct Job {
    path: PathBuf,
    bin: Bin,
}

struct Catalog {
    segs: Vec<CatalogSeg>,
    pent: Vec<Hub>,
    hex: Vec<Hub>,
    cents: Vec<[f32; 3]>,
    faces: Vec<PixelFace>,
}

struct Mote {
    ruling: usize,
    s: f32,
    dir: f32,
    pos: Vec3,
    vel: Vec3,
    hue: f32,
}

fn parse_args() -> Result<Args> {
    let mut frames = 1u32;
    let mut width = 1920u32;
    let mut height = 1080u32;
    let mut named: Vec<Job> = Vec::new();
    let mut lines: Vec<PathBuf> = Vec::new();
    let mut capture: Option<PathBuf> = None;
    let mut headless = false;
    let mut beat: Option<String> = None;
    let mut field: Option<PathBuf> = None;
    let mut larva: Option<PathBuf> = None;
    let mut chaeta: Option<PathBuf> = None;
    let mut groups: Option<PathBuf> = None;
    let mut compare: Option<PathBuf> = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--headless" => headless = true,
            "--frames" => frames = it.next().context("--frames N")?.parse()?,
            "--width" => width = it.next().context("--width N")?.parse()?,
            "--height" => height = it.next().context("--height N")?.parse()?,
            "--capture" => capture = Some(PathBuf::from(it.next().context("--capture DIR")?)),
            "--beat" => beat = Some(it.next().context("--beat NAME")?),
            "--field" => field = Some(PathBuf::from(it.next().context("--field PATH")?)),
            "--larva" => larva = Some(PathBuf::from(it.next().context("--larva PATH")?)),
            "--chaeta" => chaeta = Some(PathBuf::from(it.next().context("--chaeta PATH")?)),
            "--groups" => groups = Some(PathBuf::from(it.next().context("--groups PATH")?)),
            "--compare" => compare = Some(PathBuf::from(it.next().context("--compare PATH")?)),
            "--s2" => named.push(Job {
                path: PathBuf::from(it.next().context("--s2 PATH")?),
                bin: Bin::S2,
            }),
            "--t2" => named.push(Job {
                path: PathBuf::from(it.next().context("--t2 PATH")?),
                bin: Bin::T2,
            }),
            "--k2" => named.push(Job {
                path: PathBuf::from(it.next().context("--k2 PATH")?),
                bin: Bin::K2,
            }),
            "--p2" => named.push(Job {
                path: PathBuf::from(it.next().context("--p2 PATH")?),
                bin: Bin::P2,
            }),
            "--helicoid" => named.push(Job {
                path: PathBuf::from(it.next().context("--helicoid PATH")?),
                bin: Bin::Helicoid,
            }),
            "--catenoid" => named.push(Job {
                path: PathBuf::from(it.next().context("--catenoid PATH")?),
                bin: Bin::Catenoid,
            }),
            "--hyperboloid" => named.push(Job {
                path: PathBuf::from(it.next().context("--hyperboloid PATH")?),
                bin: Bin::Hyperboloid,
            }),
            "--theta" => named.push(Job {
                path: PathBuf::from(it.next().context("--theta PATH")?),
                bin: Bin::Theta,
            }),
            "--lines" => lines.push(PathBuf::from(it.next().context("--lines FILE")?)),
            other => bail!("unknown arg {other}"),
        }
    }
    if !headless {
        bail!("pass-2 is --headless only");
    }
    if !named.is_empty() && !lines.is_empty() {
        bail!("do not mix --lines with named flags");
    }
    let mut jobs = named;
    if jobs.is_empty() {
        for p in lines {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let bin = Bin::from_stem(stem).unwrap_or(Bin::S2);
            jobs.push(Job { path: p, bin });
        }
    }
    let beat = match beat.as_deref() {
        None => None,
        Some("caterpillar") => Some(Beat::Caterpillar),
        Some("caterpillar-grow") => Some(Beat::CaterpillarGrow),
        Some("caterpillar-skin") => Some(Beat::CaterpillarSkin),
        Some("caterpillar-chaeta") => Some(Beat::CaterpillarChaeta),
        Some("hang-chrysalis") => Some(Beat::HangChrysalis),
        Some(other) => bail!(
            "unknown --beat {other}; expected caterpillar, caterpillar-grow, caterpillar-skin, caterpillar-chaeta, or hang-chrysalis"
        ),
    };
    if larva.is_none() {
        larva = field.clone();
    }
    if beat.is_none() && jobs.is_empty() {
        bail!("need --lines FILE or --s2/--t2/--k2/--p2/--helicoid/--catenoid/--theta");
    }
    if matches!(
        beat,
        Some(
            Beat::CaterpillarGrow
                | Beat::CaterpillarSkin
                | Beat::Caterpillar
                | Beat::CaterpillarChaeta
                | Beat::HangChrysalis,
        )
    ) && larva.is_none()
    {
        bail!("--beat needs --larva PATH or --field PATH");
    }
    jobs.sort_by_key(|j| match j.bin {
        Bin::S2 => 0,
        Bin::T2 => 1,
        Bin::K2 => 2,
        Bin::P2 => 3,
        Bin::Helicoid => 4,
        Bin::Catenoid => 5,
        Bin::Hyperboloid => 6,
        Bin::Theta => 7,
    });
    Ok(Args {
        frames: frames.max(1),
        width: width.max(1),
        height: height.max(1),
        jobs,
        capture,
        beat,
        field,
        larva,
        chaeta,
        groups,
        compare,
    })
}

struct Args {
    frames: u32,
    width: u32,
    height: u32,
    jobs: Vec<Job>,
    capture: Option<PathBuf>,
    beat: Option<Beat>,
    field: Option<PathBuf>,
    larva: Option<PathBuf>,
    chaeta: Option<PathBuf>,
    groups: Option<PathBuf>,
    compare: Option<PathBuf>,
}

/// `--larva` may be a recipe dir or `net.json`. `../shellscan/...` from
/// Playground is the Projects sibling, not a Playground checkout.
fn resolve_catalog_dir(raw: &Path) -> Result<PathBuf> {
    let mut tries = vec![raw.to_path_buf()];
    if let Ok(home) = std::env::var("HOME") {
        let s = raw.to_string_lossy().replace('\\', "/");
        let rest = s
            .strip_prefix("../shellscan/")
            .or_else(|| s.strip_prefix("~/Projects/shellscan/"));
        if let Some(rest) = rest {
            tries.push(PathBuf::from(&home).join("Projects/shellscan").join(rest));
        }
        if raw.file_name().and_then(|n| n.to_str()) == Some("net.json") {
            if let Some(name) = raw.parent().and_then(|p| p.file_name()) {
                tries.push(
                    PathBuf::from(&home)
                        .join("Projects/shellscan/output/recipe")
                        .join(name)
                        .join("net.json"),
                );
            }
        }
    }
    for t in &tries {
        let dir = if t.is_file() {
            t.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| t.clone())
        } else {
            t.clone()
        };
        if dir.join("net.json").is_file() {
            return Ok(dir);
        }
    }
    bail!(
        "catalog dump not found at {} (need net.json)",
        raw.display()
    )
}

fn resolve_pixel_bin(field: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    let mut tries = Vec::new();
    if let Some(p) = field {
        tries.push(p.to_path_buf());
        if p.is_dir() {
            tries.push(p.join("qga_pixel_field.bin"));
        }
        if let Ok(home) = std::env::var("HOME") {
            let s = p.to_string_lossy().replace('\\', "/");
            if let Some(rest) = s.strip_prefix("../shellscan/") {
                tries.push(PathBuf::from(home).join("Projects/shellscan").join(rest));
            }
        }
    }
    tries.push(dir.join("qga_pixel_field.bin"));
    tries.into_iter().find(|p| p.is_file())
}

fn resolve_chaeta_path(raw: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    let mut tries = Vec::new();
    if let Some(p) = raw {
        tries.push(p.to_path_buf());
        if let Ok(home) = std::env::var("HOME") {
            let s = p.to_string_lossy().replace('\\', "/");
            if let Some(rest) = s.strip_prefix("../shellscan/") {
                tries.push(PathBuf::from(home).join("Projects/shellscan").join(rest));
            }
        }
    }
    tries.push(dir.join("chaetotaxy.json"));
    tries.into_iter().find(|p| p.is_file())
}

fn resolve_groups_path(raw: Option<&Path>, dir: &Path) -> Option<PathBuf> {
    let mut tries = Vec::new();
    if let Some(p) = raw {
        tries.push(p.to_path_buf());
    }
    tries.push(dir.join("compare_groups.json"));
    if let Some(parent) = dir.parent() {
        tries.push(parent.join("compare_groups.json"));
    }
    if let Ok(home) = std::env::var("HOME") {
        let root = PathBuf::from(home).join("Projects/shellscan");
        tries.push(root.join("output/recipe/compare_groups.json"));
        tries.push(root.join("docs/recipe-scores/compare_groups.json"));
    }
    tries.into_iter().find(|p| p.is_file())
}

fn init_gpu(width: u32, height: u32) -> Result<GpuContext> {
    GpuContext::init_headless_extent(width, height).context("init_headless")
}

#[derive(Clone, Copy)]
enum Beat {
    Caterpillar,
    CaterpillarGrow,
    CaterpillarSkin,
    CaterpillarChaeta,
    HangChrysalis,
}

fn theta_capture_stem(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("theta");
    stem.strip_suffix("_edges").unwrap_or(stem).to_string()
}

fn write_bgra(dir: &Path, name: &str, bgra: &[u8], w: u32, h: u32) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(format!("{name}.bgra")), bgra)?;
    std::fs::write(
        dir.join(format!("{name}.txt")),
        format!("bgra {w}x{h} bytes={}\n", bgra.len()),
    )?;
    Ok(())
}

fn style(color: Vec3) -> LineStyle {
    LineStyle {
        color,
        opacity: 1.0,
        width: 1.0,
        depth_bias: 0.0,
    }
}

fn edges_to_verts(edges: &[[Vec3; 2]], color: Vec3) -> Vec<LineVert> {
    let col = [color.x, color.y, color.z, 1.0];
    edges
        .iter()
        .flat_map(|[a, b]| {
            [
                LineVert {
                    pos: (*a).into(),
                    pad: 0.0,
                    color: col,
                },
                LineVert {
                    pos: (*b).into(),
                    pad: 0.0,
                    color: col,
                },
            ]
        })
        .collect()
}

fn net_bone_verts(net: &qga_swarm_convert::Net, alpha: f32) -> Vec<LineVert> {
    let a = alpha.clamp(0.0, 1.0);
    let col = [BONE.x * a, BONE.y * a, BONE.z * a, 1.0];
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for ring in &net.faces {
        if ring.len() < 2 {
            continue;
        }
        for k in 0..ring.len() {
            let i = ring[k];
            let j = ring[(k + 1) % ring.len()];
            let key = if i <= j { (i, j) } else { (j, i) };
            if !seen.insert(key) {
                continue;
            }
            let pa = net.verts.get(i as usize).copied().unwrap_or([0.0; 3]);
            let pb = net.verts.get(j as usize).copied().unwrap_or([0.0; 3]);
            out.push(LineVert {
                pos: pa,
                pad: 0.0,
                color: col,
            });
            out.push(LineVert {
                pos: pb,
                pad: 0.0,
                color: col,
            });
        }
    }
    out
}

fn catalog_to_verts(segs: &[CatalogSeg], alpha: f32) -> Vec<LineVert> {
    let a = alpha.clamp(0.0, 1.0);
    segs.iter()
        .flat_map(|s| {
            let col = [s.color[0] * a, s.color[1] * a, s.color[2] * a, 1.0];
            [
                LineVert {
                    pos: s.a,
                    pad: 0.0,
                    color: col,
                },
                LineVert {
                    pos: s.b,
                    pad: 0.0,
                    color: col,
                },
            ]
        })
        .collect()
}

fn draw_stub_hubs(renderer: &mut Renderer) {
    renderer.draw_geodesic_orb(Mat4::from_scale(Vec3::splat(0.06)), CYAN, 1);
    renderer.draw_geodesic_orb(
        Mat4::from_translation(Vec3::Y * 0.9) * Mat4::from_scale(Vec3::splat(0.04)),
        CYAN,
        1,
    );
}

fn catalog_and_refuse() -> Vec<HudVert> {
    plate_hud("CATALOG", None, "GENERATORS", 0.0, false)
}

fn plate_hud(
    left_title: &str,
    occ: Option<&OccupancyCard>,
    beat: &str,
    theta: f32,
    open: bool,
) -> Vec<HudVert> {
    const PANEL: [f32; 4] = [0.02, 0.04, 0.08, 0.72];
    const INK: [f32; 4] = [0.92, 0.95, 1.00, 0.92];
    const GOLD_A: [f32; 4] = [1.00, 0.78, 0.38, 0.95];
    const VENN_A: [f32; 4] = [0.20, 0.60, 1.00, 0.28];
    const VENN_B: [f32; 4] = [1.00, 0.40, 0.20, 0.28];
    let mut v = Vec::new();
    hud_quad(&mut v, -0.96, 0.50, -0.38, 0.94, PANEL);
    hud_text(&mut v, -0.94, 0.90, 0.016, left_title, GOLD_A);
    hud_text(&mut v, -0.94, 0.84, 0.014, "CARDS + HUBS", INK);
    hud_text(&mut v, -0.94, 0.78, 0.014, "NOT OCEAN", INK);
    if open {
        hud_text(&mut v, -0.94, 0.72, 0.014, "SHELLSCAN DUMP", INK);
        hud_text(&mut v, -0.94, 0.66, 0.014, "READ ONLY", INK);
        if let Some(o) = occ {
            hud_text(
                &mut v,
                -0.94,
                0.60,
                0.012,
                &format!("AGREE {:.2}", o.section_agree),
                GOLD_A,
            );
        }
    } else {
        hud_text(&mut v, -0.94, 0.72, 0.014, "P22 1.0", INK);
        hud_text(&mut v, -0.94, 0.66, 0.014, "POLYOMA 1/6", INK);
        if let Some(o) = occ {
            let line = format!("AGREE {:.2}", o.section_agree);
            hud_text(&mut v, -0.94, 0.60, 0.014, &line, GOLD_A);
            hud_text(&mut v, -0.94, 0.54, 0.012, "HYPOTHESIS", INK);
        } else {
            hud_text(&mut v, -0.94, 0.60, 0.014, "SHELLSCAN DUMP", INK);
            hud_text(&mut v, -0.94, 0.54, 0.014, "READ ONLY", INK);
        }
    }

    hud_quad(&mut v, 0.38, 0.50, 0.96, 0.94, PANEL);
    hud_text(&mut v, 0.40, 0.90, 0.016, "REFUSE", GOLD_A);
    hud_text(&mut v, 0.40, 0.84, 0.014, "HYPOTHESIS / MODEL", INK);
    hud_text(&mut v, 0.40, 0.78, 0.014, "FACEPLATE UNUSED", INK);
    hud_text(&mut v, 0.40, 0.72, 0.014, "CATALOG CANNOT", INK);
    hud_text(&mut v, 0.40, 0.66, 0.014, "PROVE OCCUPANT", INK);
    if open {
        hud_text(&mut v, 0.40, 0.60, 0.012, "CLOSED NET SCORES", GOLD_A);
        hud_text(&mut v, 0.40, 0.54, 0.012, "P22 1.0", INK);
        hud_text(&mut v, 0.40, 0.48, 0.012, "POLYOMA 1/6", INK);
        hud_text(&mut v, 0.40, 0.42, 0.012, "NOT THIS DUMP", INK);
    }

    hud_quad(&mut v, -0.22, -0.92, 0.08, -0.72, VENN_A);
    hud_quad(&mut v, -0.08, -0.92, 0.22, -0.72, VENN_B);
    hud_text(&mut v, -0.20, -0.70, 0.012, "CATALOG", INK);
    hud_text(&mut v, 0.02, -0.70, 0.012, "LIFE", INK);

    hud_text(&mut v, -0.20, 0.46, 0.014, beat, GOLD_A);
    let tline = if open {
        format!("THETA {:.2}  OPEN", theta)
    } else {
        format!("THETA {:.2}  CHI=2", theta)
    };
    hud_text(&mut v, -0.20, 0.40, 0.012, &tline, INK);
    v
}

fn helicoid(u: f32, v: f32) -> Vec3 {
    Vec3::new(u * v.cos(), u * v.sin(), C_ASSOC * v)
}

fn catenoid(u: f32, v: f32) -> Vec3 {
    let r = C_ASSOC * (u / C_ASSOC).cosh();
    Vec3::new(r * v.cos(), r * v.sin(), u)
}

fn associate(u: f32, v: f32, theta: f32) -> Vec3 {
    catenoid(u, v) * theta.cos() + helicoid(u, v) * theta.sin()
}

fn d_associate(u: f32, v: f32, theta: f32) -> Vec3 {
    -catenoid(u, v) * theta.sin() + helicoid(u, v) * theta.cos()
}

fn theta_of(tau: f32) -> f32 {
    std::f32::consts::FRAC_PI_2 * tau
}

/// 48-frame sheet scaled to `--frames`. Lens is the long beat.
fn sheet(i: u32, frames: u32) -> (u8, f32, f32) {
    let n = frames.max(1);
    let tau = if n <= 1 {
        0.0
    } else {
        i as f32 / (n - 1) as f32
    };
    let x = i as f32 * 48.0 / n as f32;
    let (beat, start, len) = if x < 8.0 {
        (0u8, 0.0, 8.0)
    } else if x < 16.0 {
        (1, 8.0, 8.0)
    } else if x < 28.0 {
        (2, 16.0, 12.0)
    } else if x < 36.0 {
        (3, 28.0, 8.0)
    } else if x < 44.0 {
        (4, 36.0, 8.0)
    } else {
        (5, 44.0, 4.0)
    };
    let frac = ((x - start) / len).clamp(0.0, 1.0);
    (beat, frac, tau)
}

fn polyline_segs(pts: &[Vec3]) -> Vec<[Vec3; 2]> {
    pts.windows(2).map(|w| [w[0], w[1]]).collect()
}

/// Allowed: `update_line_verts`. Rebuild associate-family rulings. Do not touch the mesh.
fn associate_polylines(theta: f32, n: usize) -> Vec<Vec<Vec3>> {
    let n = n.max(2);
    let mut lines = Vec::with_capacity(16);
    for k in 0..8 {
        let v = -std::f32::consts::PI + 2.0 * std::f32::consts::PI * k as f32 / 8.0;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let u = -1.0 + 2.0 * i as f32 / (n - 1) as f32;
            pts.push(associate(u, v, theta));
        }
        lines.push(pts);
    }
    for k in 0..8 {
        let u = -1.0 + 2.0 * k as f32 / 7.0;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let v = -std::f32::consts::PI + 2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32;
            pts.push(associate(u, v, theta));
        }
        lines.push(pts);
    }
    lines
}

fn associate_edges(theta: f32, n: usize) -> Vec<[Vec3; 2]> {
    associate_polylines(theta, n)
        .iter()
        .flat_map(|pts| polyline_segs(pts))
        .collect()
}

/// Allowed: `draw_geodesic_orb_alpha`. Waist v=0. scale and α follow |∂p/∂t|.
fn midplane_orb(theta: f32) -> (Mat4, f32) {
    let p = associate(0.0, 0.0, theta);
    let speed = d_associate(0.0, 0.0, theta).length() / 0.35;
    let speed = speed.clamp(0.0, 1.0);
    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05 + 0.04 * speed));
    (m, (0.25 + 0.75 * speed).clamp(0.02, 1.0))
}

fn hyperboloid_point(family: usize, alpha: f32, t: f32) -> Vec3 {
    let a = 0.70f32;
    let c = 0.55f32;
    let (ca, sa) = (alpha.cos(), alpha.sin());
    if family == 0 {
        Vec3::new(a * (ca - t * sa), a * (sa + t * ca), c * t)
    } else {
        Vec3::new(a * (ca + t * sa), a * (sa - t * ca), c * t)
    }
}

fn hyperboloid_edges(n: usize) -> Vec<[Vec3; 2]> {
    let n = n.max(2);
    let mut segs = Vec::new();
    for k in 0..8 {
        let th = 2.0 * std::f32::consts::PI * k as f32 / 8.0;
        for family in 0..2 {
            let mut pts = Vec::with_capacity(n);
            for i in 0..n {
                let t = -1.0 + 2.0 * i as f32 / (n - 1) as f32;
                pts.push(hyperboloid_point(family, th, t));
            }
            segs.extend(polyline_segs(&pts));
        }
    }
    segs
}

fn associate_line_verts(theta: f32, n: usize) -> Vec<LineVert> {
    let w = theta.sin().abs();
    let col = ORANGE * (1.0 - w) + CYAN * w;
    edges_to_verts(&associate_edges(theta, n), col)
}

fn load_catalog(dir: &Path) -> Result<Catalog> {
    let net = load_net_json(&dir.join("net.json")).map_err(|e| anyhow::anyhow!("{e}"))?;
    let faces = load_qga_pixel_field(&dir.join("qga_pixel_field.bin"))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let segs = catalog_line_verts(&net, &faces).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Catalog {
        pent: pentavalent_hubs(&net),
        hex: hexavalent_hubs(&net),
        cents: face_centroids(&net),
        segs,
        faces,
    })
}

fn stamp_hubs(
    renderer: &mut Renderer,
    cat: &Catalog,
    pent_alpha: f32,
    hex_alpha: f32,
    mid: Option<(Mat4, f32)>,
) {
    let pa = pent_alpha.clamp(0.02, 1.0);
    for h in &cat.pent {
        let m = Mat4::from_translation(Vec3::from(h.pos)) * Mat4::from_scale(Vec3::splat(h.radius));
        renderer.draw_geodesic_orb_alpha(m, CYAN, pa);
    }
    let ha = hex_alpha.clamp(0.02, 1.0);
    if hex_alpha > 1e-4 {
        for h in &cat.hex {
            let m =
                Mat4::from_translation(Vec3::from(h.pos)) * Mat4::from_scale(Vec3::splat(h.radius));
            renderer.draw_geodesic_orb_alpha(m, ORANGE, ha);
        }
    }
    if let Some((m, a)) = mid {
        renderer.draw_geodesic_orb_alpha(m, GOLD, a.clamp(0.02, 1.0));
    }
}

fn seed_motes(lines: &[Vec<Vec3>], cat: &Catalog) -> Vec<Mote> {
    let mut motes = Vec::new();
    let rulings = lines.iter().take(8).collect::<Vec<_>>();
    for (ri, pts) in rulings.iter().enumerate() {
        if pts.len() < 2 {
            continue;
        }
        for k in 0..32 {
            if motes.len() >= MOTE_CAP {
                return motes;
            }
            let s = k as f32 / 31.0;
            let mut m = Mote {
                ruling: ri,
                s,
                dir: if k % 2 == 0 { 1.0 } else { -1.0 },
                pos: Vec3::ZERO,
                vel: Vec3::ZERO,
                hue: SECTION_HUE[0],
            };
            place_mote(&mut m, pts);
            let bits = nearest_section(&cat.cents, &cat.faces, m.pos.to_array());
            m.hue = SECTION_HUE[bits];
            motes.push(m);
        }
    }
    motes
}

fn place_mote(m: &mut Mote, pts: &[Vec3]) {
    let nseg = (pts.len() - 1) as f32;
    let f = (m.s.clamp(0.0, 1.0) * nseg).min(nseg - 1e-4);
    let i = f.floor() as usize;
    let t = f - i as f32;
    let a = pts[i];
    let b = pts[i + 1];
    m.pos = a.lerp(b, t);
    m.vel = (b - a).normalize_or_zero() * m.dir;
}

/// Allowed: `write_particles`, N ≤ 512. CPU Euler along the ruling. Bounce at rims.
fn advect(motes: &mut [Mote], lines: &[Vec<Vec3>], dt: f32, cat: &Catalog) {
    for m in motes.iter_mut() {
        let Some(pts) = lines.get(m.ruling) else {
            continue;
        };
        if pts.len() < 2 {
            continue;
        }
        m.s += m.dir * 0.35 * dt;
        if m.s > 1.0 {
            m.s = 2.0 - m.s;
            m.dir = -m.dir;
        }
        if m.s < 0.0 {
            m.s = -m.s;
            m.dir = -m.dir;
        }
        m.s = m.s.clamp(0.0, 1.0);
        place_mote(m, pts);
        let bits = nearest_section(&cat.cents, &cat.faces, m.pos.to_array());
        m.hue = SECTION_HUE[bits];
    }
}

fn gpu_motes(motes: &[Mote]) -> Vec<GpuParticle> {
    motes
        .iter()
        .map(|m| GpuParticle::new(m.pos, m.vel, 0.55).with_hue(m.hue))
        .collect()
}

fn job_edges(jobs: &[Job], want: Bin) -> Result<Vec<[Vec3; 2]>> {
    for j in jobs {
        if j.bin == want {
            let parcel = load_qgae(&j.path).map_err(|e| anyhow::anyhow!("{e}"))?;
            return Ok(glam_edges(&parcel));
        }
    }
    bail!("missing {} parcel", want.stem());
}

fn run_stills(args: Args) -> Result<()> {
    let multi = args.jobs.len() > 1;
    let mut gpu = init_gpu(args.width, args.height)?;
    let mut renderer = Renderer::new(&gpu)?;
    let mut camera = Camera::orbit(Vec3::ZERO, 4.2);
    camera.aspect = args.width as f32 / args.height as f32;
    let vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        ..VisualState::default()
    };

    for job in &args.jobs {
        let parcel = load_qgae(&job.path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let edges = glam_edges(&parcel);
        renderer.update_line_segments(&gpu, &edges, style(job.bin.rgb()));
        renderer.write_hud(&gpu, &catalog_and_refuse())?;

        let mut last: Option<(Vec<u8>, u32, u32)> = None;
        for i in 0..args.frames {
            draw_stub_hubs(&mut renderer);
            let grab = args.capture.is_some() && i + 1 == args.frames;
            if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, i as f32 * 0.016, grab)? {
                last = Some((frame.bgra, frame.width, frame.height));
            }
        }
        if let (Some(dir), Some((bgra, w, h))) = (args.capture.as_deref(), last) {
            let name = if !multi {
                "last".to_string()
            } else if matches!(job.bin, Bin::Theta) {
                theta_capture_stem(&job.path)
            } else {
                job.bin.stem().to_string()
            };
            write_bgra(dir, &name, &bgra, w, h)?;
        }
    }
    Ok(())
}

fn run_caterpillar(args: Args) -> Result<()> {
    let field_raw = args
        .larva
        .as_deref()
        .or(args.field.as_deref())
        .context("--beat caterpillar needs --field DIR or --larva PATH")?;
    let field = resolve_catalog_dir(field_raw)?;
    let cat = load_catalog(&field)?;
    let occ = match args.compare.as_deref() {
        Some(p) => Some(load_occupancy(p).map_err(|e| anyhow::anyhow!("{e}"))?),
        None => None,
    };

    let s2 = job_edges(&args.jobs, Bin::S2).unwrap_or_default();
    let hyp = job_edges(&args.jobs, Bin::Hyperboloid).unwrap_or_default();

    let mut gpu = init_gpu(args.width, args.height)?;
    let mut renderer = Renderer::new(&gpu)?;
    let mut camera = Camera::orbit(Vec3::ZERO, 4.2);
    camera.aspect = args.width as f32 / args.height as f32;
    let mut vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        zener: 2.4,
        ..VisualState::default()
    };

    let frames = args.frames.max(1);
    let cage = if hyp.is_empty() {
        hyperboloid_edges(32)
    } else {
        hyp
    };

    let mut motes: Vec<Mote> = Vec::new();
    let mut motes_seeded = false;
    let mut family = 0usize;
    let mut frozen_mid: Option<(Mat4, f32)> = None;
    let mut frozen_pent = 0.02f32;
    let mut frozen_hex = 0.02f32;

    let capture_dir = args.capture.as_deref();
    if let Some(dir) = capture_dir {
        std::fs::create_dir_all(dir)?;
    }

    for i in 0..frames {
        let (beat_i, frac, _tau) = sheet(i, frames);
        let time = i as f32 / 24.0;
        vis.pulse = 0.5 + 0.5 * time.sin();
        vis.zener = 2.4;

        let beat_name = match beat_i {
            0 => "GENERATORS",
            1 => "CATALOG",
            2 => "LENS",
            3 => "CAGE",
            4 => "LIFE",
            _ => "REFUSE",
        };

        let freeze = beat_i == 5;
        let theta = match beat_i {
            2 => theta_of(frac),
            0 | 1 => 0.0,
            _ => theta_of(1.0),
        };

        let pent_a;
        let hex_a;
        let mut verts: Vec<LineVert> = Vec::new();
        let mut mid: Option<(Mat4, f32)> = None;
        let mut show_motes = false;

        if freeze {
            pent_a = frozen_pent;
            hex_a = frozen_hex;
            mid = frozen_mid;
            stamp_hubs(&mut renderer, &cat, pent_a, hex_a, mid);
        } else {
            match beat_i {
                0 => {
                    pent_a = 0.02;
                    hex_a = 0.02;
                    verts.extend(edges_to_verts(&s2, Bin::S2.rgb()));
                }
                1 => {
                    pent_a = 0.02 + 0.98 * frac;
                    hex_a = POLY_ALPHA;
                    verts.extend(catalog_to_verts(&cat.segs, pent_a));
                }
                2 => {
                    pent_a = 1.0;
                    hex_a = POLY_ALPHA;
                    verts.extend(catalog_to_verts(&cat.segs, 1.0));
                    verts.extend(associate_line_verts(theta, 48));
                    mid = Some(midplane_orb(theta));
                }
                3 => {
                    pent_a = 1.0;
                    hex_a = POLY_ALPHA;
                    verts.extend(catalog_to_verts(&cat.segs, 1.0));
                    verts.extend(edges_to_verts(&cage, GOLD));
                    let s = (frac * 2.0 - 1.0).clamp(-1.0, 1.0);
                    if frac > 0.5 {
                        family = 1;
                    }
                    let p = hyperboloid_point(family, 0.4, s);
                    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.06));
                    mid = Some((m, 0.9));
                }
                _ => {
                    pent_a = 1.0;
                    hex_a = POLY_ALPHA;
                    verts.extend(catalog_to_verts(&cat.segs, 1.0));
                    let lines = associate_polylines(theta, 48);
                    verts.extend(associate_line_verts(theta, 48));
                    mid = Some(midplane_orb(theta));
                    show_motes = true;
                    if !motes_seeded {
                        motes = seed_motes(&lines, &cat);
                        motes_seeded = true;
                    }
                    advect(&mut motes, &lines, 1.0 / 24.0, &cat);
                }
            }

            renderer.update_line_verts(&gpu, &verts);
            stamp_hubs(&mut renderer, &cat, pent_a, hex_a, mid);
            let particles = if show_motes {
                gpu_motes(&motes)
            } else {
                Vec::new()
            };
            renderer.write_particles(&gpu, &particles)?;
            frozen_mid = mid;
            frozen_pent = pent_a;
            frozen_hex = hex_a;
        }

        renderer.write_hud(
            &gpu,
            &plate_hud("CATALOG", occ.as_ref(), beat_name, theta, false),
        )?;

        let grab = capture_dir.is_some();
        if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, time, grab)? {
            if let Some(dir) = capture_dir {
                write_bgra(
                    dir,
                    &format!("frame_{i:04}"),
                    &frame.bgra,
                    frame.width,
                    frame.height,
                )?;
            }
        }
    }
    Ok(())
}

const BONE: Vec3 = Vec3::new(0.72, 0.72, 0.68);
const R0: f32 = 0.11;
const SIGMA: [f32; 5] = [0.45, 0.60, 0.75, 0.90, 1.00];
const GROUP_FADE: [&str; 6] = ["XD", "D", "SD", "L", "SV", "V"];

/// rings (segments), length along s, twist radians at end of instar.
const INSTAR: [(u32, f32, f32); 5] = [
    (3, 0.35, 0.0),
    (5, 0.50, 0.0),
    (8, 0.70, 0.0),
    (11, 0.88, 0.25),
    (13, 1.00, 0.0),
];

fn grow_sheet(i: u32, frames: u32) -> (u8, f32, bool) {
    let n = frames.max(1);
    let x = i as f32 * 240.0 / n as f32;
    let (stage, start, len) = if x < 40.0 {
        (0u8, 0.0, 40.0)
    } else if x < 80.0 {
        (1, 40.0, 40.0)
    } else if x < 120.0 {
        (2, 80.0, 40.0)
    } else if x < 160.0 {
        (3, 120.0, 40.0)
    } else if x < 200.0 {
        (4, 160.0, 40.0)
    } else {
        (5, 200.0, 40.0)
    };
    let local = x - start;
    let frac = (local / len).clamp(0.0, 1.0);
    let flash = stage >= 1 && stage <= 4 && local < 2.0;
    (stage, frac, flash)
}

fn instar_cylinder(
    n_seg: u32,
    n_phi: u32,
    length: f32,
    twist: f32,
    radius: f32,
    height: f32,
) -> Vec<[Vec3; 2]> {
    let n_seg = n_seg.max(1);
    let n_phi = n_phi.max(8);
    let n_rings = n_seg + 1;
    let mut rings = vec![vec![Vec3::ZERO; n_phi as usize]; n_rings as usize];
    for i in 0..n_rings {
        let u = i as f32 / n_seg as f32;
        let z = 0.5 * height - length * height * u;
        for j in 0..n_phi {
            let phi = std::f32::consts::TAU * j as f32 / n_phi as f32;
            let psi = phi + twist * u;
            rings[i as usize][j as usize] = Vec3::new(radius * psi.cos(), radius * psi.sin(), z);
        }
    }
    let mut segs = Vec::new();
    for i in 0..n_rings {
        for j in 0..n_phi {
            let j2 = (j + 1) % n_phi;
            segs.push([
                rings[i as usize][j as usize],
                rings[i as usize][j2 as usize],
            ]);
        }
    }
    for i in 0..n_seg {
        for j in 0..n_phi {
            segs.push([
                rings[i as usize][j as usize],
                rings[(i + 1) as usize][j as usize],
            ]);
        }
    }
    segs
}

fn site_pos(
    site: &SetalSite,
    n_seg: u32,
    length: f32,
    twist: f32,
    radius: f32,
    height: f32,
    mirror: bool,
) -> Vec3 {
    let u = (site.segment_index as f32 + 0.5) / n_seg.max(1) as f32;
    let z = 0.5 * height - length * height * u;
    let mut phi = site.phi_deg.to_radians();
    if mirror {
        phi = -phi;
    }
    let psi = phi + twist * u;
    let r = radius * 1.06;
    Vec3::new(r * psi.cos(), r * psi.sin(), z)
}

fn stamp_setal(
    renderer: &mut Renderer,
    sites: &[SetalSite],
    n_seg: u32,
    length: f32,
    twist: f32,
    radius: f32,
    height: f32,
    tentacles_only: bool,
    amp_scale: f32,
    alpha: f32,
) {
    let a = alpha.clamp(0.02, 1.0);
    for site in sites {
        if site.segment_index < 0 || site.segment_index as u32 >= n_seg {
            continue;
        }
        if tentacles_only && !site.tentacle {
            continue;
        }
        let col = Vec3::new(
            SECTION_RGBA[site.section][0],
            SECTION_RGBA[site.section][1],
            SECTION_RGBA[site.section][2],
        );
        let scale = if site.tentacle {
            0.045 + 0.04 * site.amplitude * amp_scale
        } else {
            0.018 + 0.012 * site.amplitude * amp_scale
        };
        for mirror in [false, true] {
            let p = site_pos(site, n_seg, length, twist, radius, height, mirror);
            let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(scale));
            renderer.draw_geodesic_orb_alpha(m, col, a);
        }
    }
}

fn cylinder_point(s: f32, phi_deg: f32, radius: f32, height: f32) -> Vec3 {
    let z = 0.5 * height - s.clamp(0.0, 1.0) * height;
    let psi = phi_deg.to_radians();
    Vec3::new(radius * psi.cos(), radius * psi.sin(), z)
}

fn chart_point(s: f32, phi_deg: f32, height: f32) -> Vec3 {
    let phi = phi_deg.abs().min(180.0);
    Vec3::new(
        2.35 + phi / 180.0 * 1.55,
        0.0,
        0.5 * height - s.clamp(0.0, 1.0) * height,
    )
}

fn group_index(group: &str) -> usize {
    GROUP_FADE.iter().position(|g| *g == group).unwrap_or(5)
}

fn site_color(site: &ChaetaSite) -> Vec3 {
    let c = SECTION_RGBA[site.section()];
    Vec3::new(c[0], c[1], c[2])
}

fn stamp_one_orb(renderer: &mut Renderer, p: Vec3, r: f32, col: Vec3, alpha: f32) {
    let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(r.max(0.008)));
    renderer.draw_geodesic_orb_alpha(m, col, alpha.clamp(0.02, 1.0));
}

fn shortest_dphi(a: f32, b: f32) -> f32 {
    let mut d = b - a;
    while d > 180.0 {
        d -= 360.0;
    }
    while d < -180.0 {
        d += 360.0;
    }
    d
}

fn radial_tick(p: Vec3, phi_deg: f32, length: f32, n: usize) -> Vec<[Vec3; 2]> {
    let n = n.max(2);
    let radial = Vec3::new(phi_deg.to_radians().cos(), phi_deg.to_radians().sin(), 0.0);
    let mut segs = Vec::with_capacity(n);
    let mut prev = p;
    for i in 1..=n {
        let q = p + radial * length * (i as f32 / n as f32);
        segs.push([prev, q]);
        prev = q;
    }
    segs
}

fn azimuthal_chord(s: f32, phi0: f32, phi1: f32, radius: f32, height: f32) -> Vec<[Vec3; 2]> {
    let d = shortest_dphi(phi0, phi1);
    let n = 6;
    let mut segs = Vec::with_capacity(n);
    let mut prev = cylinder_point(s, phi0, radius, height);
    for i in 1..=n {
        let phi = phi0 + d * (i as f32 / n as f32);
        let q = cylinder_point(s, phi, radius, height);
        segs.push([prev, q]);
        prev = q;
    }
    segs
}

fn push_col(out: &mut Vec<LineVert>, segs: &[[Vec3; 2]], col: Vec3) {
    let c = [col.x, col.y, col.z, 1.0];
    for [a, b] in segs {
        out.push(LineVert {
            pos: (*a).into(),
            pad: 0.0,
            color: c,
        });
        out.push(LineVert {
            pos: (*b).into(),
            pad: 0.0,
            color: c,
        });
    }
}

/// Occupancy orbs + amp ticks + optional Δφ chords. Skeleton stays uploaded.
fn stamp_atlas(
    renderer: &mut Renderer,
    atlas: &Chaetotaxy,
    instar: u8,
    sigma: f32,
    radius: f32,
    height: f32,
    alpha_scale: f32,
    group_step: Option<(usize, f32)>,
    allow_primary: bool,
    allow_sub: bool,
    allow_tentacle: bool,
    allow_spiracle: bool,
    tentacle_frac: f32,
    ticks: bool,
    chart: bool,
    amp_ticks: bool,
) -> (Vec<LineVert>, bool) {
    let mut extra = Vec::new();
    let mut cell_jump = false;
    let cell = 360.0 / atlas.n_phi.max(1) as f32;
    let cyan = CYAN;
    let gold = GOLD;
    for site in &atlas.sites {
        if site.instar > instar {
            continue;
        }
        let mut alpha = alpha_scale;
        match site.kind.as_str() {
            "tentacle" if !allow_tentacle => continue,
            "spiracle" if !allow_spiracle => continue,
            "seta" => {
                if site.primary() && !allow_primary {
                    continue;
                }
                if site.subprimary() && !allow_sub {
                    continue;
                }
                if !site.primary() && !site.subprimary() && !(allow_primary && allow_sub) {
                    continue;
                }
            }
            _ => {}
        }
        if let Some((step, frac)) = group_step {
            if site.kind == "seta" && site.primary() {
                let gi = group_index(&site.group);
                if gi > step {
                    continue;
                }
                if gi == step {
                    alpha *= frac;
                }
            }
        }
        let r = R0 * site.amp * sigma;
        let phis: Vec<f32> = if site.mirror() {
            vec![site.phi_deg, -site.phi_deg]
        } else {
            vec![site.phi_deg]
        };
        let col = site_color(site);
        for &phi in &phis {
            let p = cylinder_point(site.s, phi, radius, height);
            let tentacle = site.kind == "tentacle";
            if tentacle {
                stamp_one_orb(renderer, p, R0 * 0.32 * sigma, gold, alpha);
            } else if ticks {
                if let Some(h) = site.phi_hinton {
                    let h_signed = if phi < 0.0 { -h } else { h };
                    let hp = cylinder_point(site.s, h_signed, radius, height);
                    stamp_one_orb(renderer, hp, r * 0.75, cyan, alpha);
                    let dphi = shortest_dphi(h_signed, phi).abs();
                    if dphi > cell {
                        cell_jump = true;
                    }
                    let chord = azimuthal_chord(site.s, h_signed, phi, radius, height);
                    push_col(&mut extra, &chord, gold * 0.85 + cyan * 0.15);
                    if chart {
                        let q0 = chart_point(site.s, h.abs(), height);
                        let q1 = chart_point(site.s, phi.abs(), height);
                        push_col(&mut extra, &[[q0, q1]], gold);
                    }
                }
                stamp_one_orb(renderer, p, r, gold, alpha);
            } else {
                stamp_one_orb(renderer, p, r, col, alpha);
            }
            if chart && !tentacle {
                let q = chart_point(site.s, phi.abs(), height);
                stamp_one_orb(renderer, q, r * 0.7, if ticks { gold } else { col }, alpha);
            }
            if tentacle && tentacle_frac > 1e-4 {
                let len = site.amp * sigma * tentacle_frac.clamp(0.0, 1.0) * 0.55;
                push_col(&mut extra, &radial_tick(p, phi, len, 3), gold);
                if chart {
                    let q = chart_point(site.s, phi.abs(), height);
                    stamp_one_orb(renderer, q, R0 * 0.22 * sigma, gold, alpha);
                }
            } else if amp_ticks && !tentacle {
                let cell_len = std::f32::consts::TAU * radius / atlas.n_phi.max(1) as f32;
                let len = site.amp * sigma * (2.2 * cell_len);
                if len > 1e-4 {
                    push_col(&mut extra, &radial_tick(p, phi, len, 5), col);
                }
            }
        }
    }
    (extra, cell_jump)
}

fn chart_frame(height: f32) -> Vec<[Vec3; 2]> {
    let z0 = 0.5 * height;
    let z1 = -0.5 * height;
    let x0 = 2.35;
    let x1 = 2.35 + 1.55;
    vec![
        [Vec3::new(x0, 0.0, z0), Vec3::new(x1, 0.0, z0)],
        [Vec3::new(x0, 0.0, z1), Vec3::new(x1, 0.0, z1)],
        [Vec3::new(x0, 0.0, z0), Vec3::new(x0, 0.0, z1)],
        [Vec3::new(x1, 0.0, z0), Vec3::new(x1, 0.0, z1)],
        [Vec3::new(x0, 0.0, z0), Vec3::new(x0, 0.0, z1)],
    ]
}

fn chaeta_sheet(i: u32, frames: u32) -> (u8, f32) {
    let n = frames.max(1);
    if n <= 24 {
        return (6, 1.0);
    }
    let x = i as f32 * 96.0 / n as f32;
    if x < 12.0 {
        (0, (x / 12.0).clamp(0.0, 1.0))
    } else if x < 36.0 {
        (1, ((x - 12.0) / 24.0).clamp(0.0, 1.0))
    } else if x < 48.0 {
        (2, ((x - 36.0) / 12.0).clamp(0.0, 1.0))
    } else if x < 60.0 {
        (3, ((x - 48.0) / 12.0).clamp(0.0, 1.0))
    } else if x < 72.0 {
        (4, ((x - 60.0) / 12.0).clamp(0.0, 1.0))
    } else if x < 84.0 {
        (5, ((x - 72.0) / 12.0).clamp(0.0, 1.0))
    } else {
        (6, 1.0)
    }
}

fn species_label(src: &str) -> &'static str {
    let s = src.to_ascii_lowercase();
    if s.contains("gilippus") {
        "GILIPPUS"
    } else if s.contains("melpomene") {
        "MELPOMENE"
    } else if s.contains("polyxenes") {
        "POLYXENES"
    } else if s.contains("hinton") {
        "HINTON"
    } else if s.contains("plexippus") {
        "PLEXIPPUS"
    } else {
        "CHAETA"
    }
}

fn chaeta_hud(
    source: &str,
    n: usize,
    rms: f32,
    order_ok: bool,
    cell_jump: bool,
    beat: &str,
    groups: &[GroupRms],
) -> Vec<HudVert> {
    let mut v = plate_hud(species_label(source), None, beat, 0.0, true);
    const INK: [f32; 4] = [0.92, 0.95, 1.00, 0.92];
    const GOLD_A: [f32; 4] = [1.00, 0.78, 0.38, 0.95];
    hud_text(&mut v, -0.94, 0.48, 0.012, &format!("N {n}"), INK);
    hud_text(&mut v, -0.94, 0.42, 0.012, "ORDER XD-D-SD-L-SV-V", INK);
    hud_text(
        &mut v,
        -0.94,
        0.36,
        0.012,
        &format!("DPHI RMS {:.1}", rms),
        GOLD_A,
    );
    let mut y = 0.24;
    for g in groups.iter().take(6) {
        let line = format!("{} {:.1}", g.group, g.rms);
        hud_text(
            &mut v,
            -0.94,
            y,
            0.011,
            &line,
            if g.cell_jump { GOLD_A } else { INK },
        );
        y -= 0.055;
    }
    if !order_ok || cell_jump {
        hud_text(&mut v, -0.20, 0.34, 0.016, "REFUSE", GOLD_A);
        if cell_jump {
            hud_text(&mut v, -0.20, 0.28, 0.012, "SNAP CELL", GOLD_A);
        }
    }
    hud_text(&mut v, -0.20, 0.16, 0.012, "OPEN CYLINDER", INK);
    hud_text(&mut v, -0.20, 0.10, 0.012, "NOT CHI=2", INK);
    hud_text(&mut v, 0.38, -0.58, 0.012, "MIDDORSAL", INK);
    hud_text(&mut v, 0.62, -0.58, 0.012, "MIDVENTRAL", INK);
    hud_text(&mut v, 0.48, 0.18, 0.012, "S ANT", INK);
    hud_text(&mut v, 0.48, -0.96, 0.012, "S POST", INK);
    if order_ok {
        hud_text(&mut v, -0.94, 0.30, 0.012, "PHI ORDER OK", GOLD_A);
    }
    v
}

fn grow_hud(instar: &str, rings: u32, flash: bool) -> Vec<HudVert> {
    let beat = if flash {
        format!("{instar} MOLT")
    } else {
        format!("{instar} RINGS {rings}")
    };
    plate_hud("SKELETON", None, &beat, 0.0, true)
}

fn run_grow(args: Args) -> Result<()> {
    let raw = args
        .larva
        .as_deref()
        .or(args.field.as_deref())
        .context("--beat caterpillar-grow needs --larva")?;
    let dir = resolve_catalog_dir(raw)?;
    let net = load_net_json(&dir.join("net.json")).map_err(|e| anyhow::anyhow!("{e}"))?;
    let atlas =
        resolve_chaeta_path(args.chaeta.as_deref(), &dir).and_then(|p| load_chaetotaxy(&p).ok());
    let sites = load_setal_sites(&dir).unwrap_or_default();
    let n_phi = if net.n_phi == 0 { 36 } else { net.n_phi };
    let radius = if net.radius <= 0.0 { 1.0 } else { net.radius };
    let height = if net.height <= 0.0 { 2.0 } else { net.height };

    let mut gpu = init_gpu(args.width, args.height)?;
    let mut renderer = Renderer::new(&gpu)?;
    let mut camera = Camera::orbit(Vec3::ZERO, 4.6);
    camera.aspect = args.width as f32 / args.height as f32;
    let mut vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        zener: 2.4,
        ..VisualState::default()
    };

    let frames = args.frames.max(1);
    let capture_dir = args.capture.as_deref();
    if let Some(d) = capture_dir {
        std::fs::create_dir_all(d)?;
    }

    let mut frozen_n = 3u32;
    let mut frozen_len = 0.35f32;
    let mut frozen_twist = 0.0f32;
    let mut frozen_amp = 0.55f32;
    let mut frozen_tent_only = true;

    for i in 0..frames {
        let (stage, frac, flash) = grow_sheet(i, frames);
        let time = i as f32 / 24.0;
        vis.pulse = 0.5 + 0.5 * time.sin();

        let refuse = stage == 5;
        let (n_seg, length, twist, tent_only, amp, name, hub_alpha) = if refuse {
            (
                frozen_n,
                frozen_len,
                frozen_twist,
                frozen_tent_only,
                frozen_amp,
                "L5 HOLD",
                1.0f32,
            )
        } else {
            let (rings, len, tw_end) = INSTAR[stage as usize];
            let twist = if stage == 3 { tw_end * frac } else { tw_end };
            let tent_only = stage <= 1;
            let amp = match stage {
                0 => 0.55,
                1 => 0.7,
                2 => 0.85,
                3 => 0.7 + 0.3 * frac,
                _ => 1.0,
            };
            let name = match stage {
                0 => "L1",
                1 => "L2",
                2 => "L3",
                3 => "L4",
                _ => "L5",
            };
            let hub_alpha = if stage == 0 { 0.02 + 0.98 * frac } else { 1.0 };
            (rings, len, twist, tent_only, amp, name, hub_alpha)
        };

        let segs = instar_cylinder(n_seg, n_phi, length, twist, radius, height);
        if !refuse {
            renderer.update_line_verts(&gpu, &edges_to_verts(&segs, BONE));
            frozen_n = n_seg;
            frozen_len = length;
            frozen_twist = twist;
            frozen_amp = amp;
            frozen_tent_only = tent_only;
        }
        if !flash {
            let instar = (stage + 1).clamp(1, 5);
            let sigma = SIGMA[(instar as usize).saturating_sub(1)];
            if let Some(atlas) = atlas.as_ref() {
                let _ = stamp_atlas(
                    &mut renderer,
                    atlas,
                    instar,
                    sigma,
                    radius,
                    height,
                    hub_alpha,
                    None,
                    true,
                    true,
                    true,
                    true,
                    1.0,
                    false,
                    false,
                    false,
                );
            } else {
                stamp_setal(
                    &mut renderer,
                    &sites,
                    n_seg,
                    length,
                    twist,
                    radius,
                    height,
                    tent_only,
                    amp,
                    hub_alpha,
                );
            }
        }
        renderer.write_particles(&gpu, &[])?;
        renderer.write_hud(
            &gpu,
            &grow_hud(if refuse { "REFUSE" } else { name }, n_seg, flash),
        )?;
        let grab = capture_dir.is_some();
        if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, time, grab)? {
            if let Some(d) = capture_dir {
                write_bgra(
                    d,
                    &format!("frame_{i:04}"),
                    &frame.bgra,
                    frame.width,
                    frame.height,
                )?;
            }
        }
    }
    Ok(())
}

fn mix4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.0,
    ]
}

fn run_skin(args: Args) -> Result<()> {
    let raw = args
        .larva
        .as_deref()
        .or(args.field.as_deref())
        .context("--beat caterpillar-skin needs --larva")?;
    let dir = resolve_catalog_dir(raw)?;
    let net = load_net_json(&dir.join("net.json")).map_err(|e| anyhow::anyhow!("{e}"))?;
    let bin = resolve_pixel_bin(args.field.as_deref(), &dir)
        .context("caterpillar-skin needs qga_pixel_field.bin")?;
    let _field = load_qga_pixel_field(&bin).map_err(|e| anyhow::anyhow!("{e}"))?;
    let sites = load_setal_sites(&dir).unwrap_or_default();
    let occ = match args.compare.as_deref() {
        Some(p) => Some(load_occupancy(p).map_err(|e| anyhow::anyhow!("{e}"))?),
        None => None,
    };

    let n_phi = if net.n_phi == 0 { 36 } else { net.n_phi };
    let n_seg = if net.n_segments == 0 {
        13
    } else {
        net.n_segments
    };
    let radius = if net.radius <= 0.0 { 1.0 } else { net.radius };
    let height = if net.height <= 0.0 { 2.0 } else { net.height };

    let mut gpu = init_gpu(args.width, args.height)?;
    let mut renderer = Renderer::new(&gpu)?;
    let mut camera = Camera::orbit(Vec3::ZERO, 4.6);
    camera.aspect = args.width as f32 / args.height as f32;
    let mut vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        zener: 2.4,
        ..VisualState::default()
    };

    let frames = args.frames.max(1);
    let capture_dir = args.capture.as_deref();
    if let Some(d) = capture_dir {
        std::fs::create_dir_all(d)?;
    }

    let bone = [BONE.x, BONE.y, BONE.z, 1.0];
    let rulings = instar_cylinder(n_seg, n_phi, 1.0, 0.0, radius, height);

    for i in 0..frames {
        let t = if frames <= 1 {
            1.0
        } else {
            i as f32 / (frames - 1) as f32
        };
        let time = i as f32 / 24.0;
        vis.pulse = 0.5 + 0.5 * time.sin();

        let n_ring_edges = ((n_seg + 1) * n_phi) as usize;
        let mut verts = Vec::new();
        for (k, [a, b]) in rulings.iter().enumerate() {
            let seg_i = if k < n_ring_edges {
                (k as u32 / n_phi).min(n_seg.saturating_sub(1))
            } else {
                ((k - n_ring_edges) as u32 / n_phi).min(n_seg.saturating_sub(1))
            };
            let col = mix4(bone, SPECIES_RGBA[(seg_i % 3) as usize], t);
            verts.push(LineVert {
                pos: (*a).into(),
                pad: 0.0,
                color: col,
            });
            verts.push(LineVert {
                pos: (*b).into(),
                pad: 0.0,
                color: col,
            });
        }
        renderer.update_line_verts(&gpu, &verts);
        stamp_setal(
            &mut renderer,
            &sites,
            n_seg,
            1.0,
            0.0,
            radius,
            height,
            false,
            1.0,
            1.0,
        );
        if t > 0.75 {
            let n_ring_edges = ((n_seg + 1) * n_phi) as usize;
            let mut motes = Vec::new();
            for (k, [a, b]) in rulings.iter().skip(n_ring_edges).enumerate() {
                if motes.len() >= MOTE_CAP {
                    break;
                }
                let phase = (k as f32 * 0.17 + time * 0.35).rem_euclid(1.0);
                let pos = *a + (*b - *a) * phase;
                motes.push(GpuParticle::new(pos, *b - *a, 0.4).with_hue(SECTION_HUE[2]));
            }
            renderer.write_particles(&gpu, &motes)?;
        } else {
            renderer.write_particles(&gpu, &[])?;
        }
        renderer.write_hud(
            &gpu,
            &plate_hud("SKIN ON FROZEN NET", occ.as_ref(), "L5 WRAP", 0.0, true),
        )?;
        let grab = capture_dir.is_some();
        if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, time, grab)? {
            if let Some(d) = capture_dir {
                write_bgra(
                    d,
                    &format!("frame_{i:04}"),
                    &frame.bgra,
                    frame.width,
                    frame.height,
                )?;
            }
        }
    }
    Ok(())
}

fn run_chaeta(args: Args) -> Result<()> {
    let raw = args
        .larva
        .as_deref()
        .or(args.field.as_deref())
        .context("--beat caterpillar-chaeta needs --larva")?;
    let dir = resolve_catalog_dir(raw)?;
    let net = load_net_json(&dir.join("net.json")).map_err(|e| anyhow::anyhow!("{e}"))?;
    let chaeta_path = resolve_chaeta_path(args.chaeta.as_deref(), &dir)
        .context("caterpillar-chaeta needs chaetotaxy.json")?;
    let atlas = load_chaetotaxy(&chaeta_path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let group_rows = resolve_groups_path(args.groups.as_deref(), &dir)
        .map(|p| load_group_row(&p, &atlas.source))
        .unwrap_or_default();
    let n_phi = if net.n_phi == 0 { 36 } else { net.n_phi };
    let n_seg = if net.n_segments == 0 {
        13
    } else {
        net.n_segments
    };
    let radius = if net.radius <= 0.0 { 1.0 } else { net.radius };
    let height = if net.height <= 0.0 { 2.0 } else { net.height };

    let mut gpu = init_gpu(args.width, args.height)?;
    let mut renderer = Renderer::new(&gpu)?;
    let mut camera = Camera::orbit(Vec3::new(1.05, 0.0, 0.0), 5.4);
    camera.yaw = 0.22;
    camera.pitch = 0.28;
    camera.aspect = args.width as f32 / args.height as f32;
    let mut vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        zener: 2.4,
        ..VisualState::default()
    };

    let frames = args.frames.max(1);
    let capture_dir = args.capture.as_deref();
    if let Some(d) = capture_dir {
        std::fs::create_dir_all(d)?;
    }

    let bone = [BONE.x, BONE.y, BONE.z, 1.0];
    let skeleton = instar_cylinder(n_seg, n_phi, 1.0, 0.0, radius, height);
    let n_ring_edges = ((n_seg + 1) * n_phi) as usize;
    let mut skin_verts = Vec::new();
    for (k, [a, b]) in skeleton.iter().enumerate() {
        let seg_i = if k < n_ring_edges {
            (k as u32 / n_phi).min(n_seg.saturating_sub(1))
        } else {
            ((k - n_ring_edges) as u32 / n_phi).min(n_seg.saturating_sub(1))
        };
        let col = mix4(bone, SPECIES_RGBA[(seg_i % 3) as usize], 1.0);
        skin_verts.push(LineVert {
            pos: (*a).into(),
            pad: 0.0,
            color: col,
        });
        skin_verts.push(LineVert {
            pos: (*b).into(),
            pad: 0.0,
            color: col,
        });
    }
    for [a, b] in chart_frame(height) {
        let col = [0.45, 0.45, 0.48, 1.0];
        skin_verts.push(LineVert {
            pos: a.into(),
            pad: 0.0,
            color: col,
        });
        skin_verts.push(LineVert {
            pos: b.into(),
            pad: 0.0,
            color: col,
        });
    }
    renderer.update_line_verts(&gpu, &skin_verts);

    let n_sites = atlas.sites.len();
    let refuse_order = !atlas.phi_order_ok;

    for i in 0..frames {
        let (phase, frac) = chaeta_sheet(i, frames);
        let time = i as f32 / 24.0;
        vis.pulse = 0.5 + 0.5 * time.sin();

        let (prim, sub, tent, spir, step, tfrac, ticks, amp_ticks, beat) =
            if refuse_order && phase >= 1 {
                (
                    true,
                    false,
                    false,
                    false,
                    Some((0usize, 1.0)),
                    0.0,
                    false,
                    true,
                    "REFUSE",
                )
            } else {
                match phase {
                    0 => (
                        false,
                        false,
                        false,
                        false,
                        None,
                        0.0,
                        false,
                        false,
                        "SKIN HOLD",
                    ),
                    1 => {
                        let g = (frac * 6.0).floor() as usize;
                        let f = (frac * 6.0).fract().max(0.15);
                        (
                            true,
                            false,
                            false,
                            false,
                            Some((g.min(5), f)),
                            0.0,
                            false,
                            true,
                            "PRIMARIES",
                        )
                    }
                    2 => (
                        true,
                        true,
                        false,
                        false,
                        None,
                        0.0,
                        false,
                        true,
                        "SUBPRIMARY",
                    ),
                    3 => (true, true, true, false, None, frac, false, true, "TENTACLE"),
                    4 => (true, true, true, true, None, 1.0, false, true, "SPIRACLE"),
                    5 => (true, true, true, true, None, 1.0, true, true, "DPHI TICK"),
                    _ => (true, true, true, true, None, 1.0, true, true, "HOLD"),
                }
            };

        let (extra, cell_jump) = stamp_atlas(
            &mut renderer,
            &atlas,
            5,
            1.0,
            radius,
            height,
            1.0,
            step,
            prim,
            sub,
            tent,
            spir,
            tfrac,
            ticks,
            true,
            amp_ticks,
        );
        let mut verts = skin_verts.clone();
        verts.extend(extra);
        renderer.update_line_verts(&gpu, &verts);
        renderer.write_particles(&gpu, &[])?;
        renderer.write_hud(
            &gpu,
            &chaeta_hud(
                &atlas.source,
                n_sites,
                atlas.dphi_rms,
                atlas.phi_order_ok,
                cell_jump,
                beat,
                &group_rows,
            ),
        )?;
        let grab = capture_dir.is_some();
        if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, time, grab)? {
            if let Some(d) = capture_dir {
                write_bgra(
                    d,
                    &format!("frame_{i:04}"),
                    &frame.bgra,
                    frame.width,
                    frame.height,
                )?;
            }
        }
    }
    Ok(())
}

fn hang_sheet(i: u32, frames: u32) -> (u8, f32) {
    let n = frames.max(1);
    let x = i as f32 * 120.0 / n as f32;
    if x < 12.0 {
        (0, (x / 12.0).clamp(0.0, 1.0))
    } else if x < 36.0 {
        (1, ((x - 12.0) / 24.0).clamp(0.0, 1.0))
    } else if x < 72.0 {
        (2, ((x - 36.0) / 36.0).clamp(0.0, 1.0))
    } else if x < 96.0 {
        (3, ((x - 72.0) / 24.0).clamp(0.0, 1.0))
    } else if x < 108.0 {
        (4, ((x - 96.0) / 12.0).clamp(0.0, 1.0))
    } else {
        (5, 1.0)
    }
}

fn hang_hud(beat: &str, chi2: bool) -> Vec<HudVert> {
    const PANEL: [f32; 4] = [0.02, 0.04, 0.08, 0.72];
    const INK: [f32; 4] = [0.92, 0.95, 1.00, 0.92];
    const GOLD_A: [f32; 4] = [1.00, 0.78, 0.38, 0.95];
    let mut v = Vec::new();
    hud_quad(&mut v, -0.96, 0.50, -0.38, 0.94, PANEL);
    hud_text(&mut v, -0.94, 0.90, 0.016, "HANG CHRYSALIS", GOLD_A);
    hud_text(&mut v, -0.94, 0.84, 0.014, "MODEL", INK);
    hud_text(&mut v, -0.94, 0.78, 0.014, "PAINT NONE", INK);
    hud_text(&mut v, -0.94, 0.72, 0.014, "NOT OCEAN", INK);
    hud_text(&mut v, -0.94, 0.66, 0.014, "FACEPLATE UNUSED", INK);
    if chi2 {
        hud_text(&mut v, -0.94, 0.60, 0.014, "CHI=2", GOLD_A);
        hud_text(&mut v, -0.94, 0.54, 0.012, "T=9 GOLDBERG", INK);
    } else {
        hud_text(&mut v, -0.94, 0.60, 0.014, "OPEN", GOLD_A);
        hud_text(&mut v, -0.94, 0.54, 0.012, "NOT CHI=2", INK);
    }
    hud_quad(&mut v, 0.38, 0.50, 0.96, 0.94, PANEL);
    hud_text(&mut v, 0.40, 0.90, 0.016, "REFUSE", GOLD_A);
    hud_text(&mut v, 0.40, 0.84, 0.014, "HYPOTHESIS / MODEL", INK);
    hud_text(&mut v, 0.40, 0.78, 0.014, "PAINT NONE", INK);
    hud_text(&mut v, 0.40, 0.72, 0.014, "CATALOG CANNOT", INK);
    hud_text(&mut v, 0.40, 0.66, 0.014, "PROVE OCCUPANT", INK);
    hud_text(&mut v, 0.40, 0.60, 0.012, "NOT MORPHOGENESIS", INK);
    hud_text(&mut v, -0.20, 0.46, 0.014, beat, GOLD_A);
    v
}

fn run_hang(args: Args) -> Result<()> {
    let raw = args
        .larva
        .as_deref()
        .or(args.field.as_deref())
        .context("--beat hang-chrysalis needs --larva")?;
    let dir = resolve_catalog_dir(raw)?;
    let net = load_net_json(&dir.join("net.json")).map_err(|e| anyhow::anyhow!("{e}"))?;
    let pent = pentavalent_hubs(&net);
    let goldberg = net_bone_verts(&net, 1.0);
    let hel = job_edges(&args.jobs, Bin::Helicoid).unwrap_or_default();
    let cage = job_edges(&args.jobs, Bin::Hyperboloid).unwrap_or_else(|_| hyperboloid_edges(32));

    let atlas = resolve_chaeta_path(args.chaeta.as_deref(), &dir)
        .or_else(|| {
            if let Ok(home) = std::env::var("HOME") {
                let p = PathBuf::from(home).join(
                    "Projects/shellscan/output/recipe/setal-plexippus-cylinder/chaetotaxy.json",
                );
                p.is_file().then_some(p)
            } else {
                None
            }
        })
        .and_then(|p| load_chaetotaxy(&p).ok());

    let mut gpu = init_gpu(args.width, args.height)?;
    let mut renderer = Renderer::new(&gpu)?;
    let mut camera = Camera::orbit(Vec3::ZERO, 4.8);
    camera.aspect = args.width as f32 / args.height as f32;
    let mut vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        zener: 2.4,
        ..VisualState::default()
    };

    let frames = args.frames.max(1);
    let capture_dir = args.capture.as_deref();
    if let Some(d) = capture_dir {
        std::fs::create_dir_all(d)?;
    }

    let cylinder = instar_cylinder(13, 36, 1.0, 0.0, 1.0, 2.0);
    let mut frozen_verts: Vec<LineVert> = Vec::new();
    let mut frozen_chi2 = false;
    let mut frozen_beat = "HOLD";

    for i in 0..frames {
        let (phase, frac) = hang_sheet(i, frames);
        let time = i as f32 / 24.0;
        vis.pulse = 0.5 + 0.5 * time.sin();
        let refuse = phase == 5;

        let (theta, cyl_a, hel_a, gold_on, hubs_on, cage_on, site_a, chi2, beat) = if refuse {
            (
                std::f32::consts::FRAC_PI_2,
                0.05,
                0.0,
                true,
                true,
                true,
                0.0,
                frozen_chi2,
                frozen_beat,
            )
        } else {
            match phase {
                0 => (0.0, 1.0, 0.0, false, false, false, 1.0, false, "L5 OPEN"),
                1 => (0.0, 1.0, frac, false, false, false, 1.0, false, "HELICOID"),
                2 => (
                    theta_of(frac),
                    (1.0 - frac).max(0.05),
                    1.0,
                    false,
                    false,
                    false,
                    (1.0 - frac).max(0.0),
                    false,
                    "ASSOCIATE",
                ),
                3 => (
                    theta_of(1.0),
                    0.05,
                    0.0,
                    true,
                    true,
                    false,
                    0.0,
                    true,
                    "T=9 CLOSE",
                ),
                _ => (
                    theta_of(1.0),
                    0.05,
                    0.0,
                    true,
                    true,
                    true,
                    0.0,
                    true,
                    "CAGE",
                ),
            }
        };

        let mut verts: Vec<LineVert> = Vec::new();
        if cyl_a > 1e-4 {
            verts.extend(edges_to_verts(&cylinder, BONE * cyl_a));
        }
        if hel_a > 1e-4 {
            if hel.is_empty() {
                verts.extend(associate_line_verts(theta, 48));
            } else if phase == 2 {
                verts.extend(associate_line_verts(theta, 48));
            } else {
                let col = CYAN * hel_a + BONE * (1.0 - hel_a);
                verts.extend(edges_to_verts(&hel, col));
            }
        }
        if gold_on {
            verts.extend(goldberg.clone());
        }
        if cage_on {
            verts.extend(edges_to_verts(&cage, GOLD * 0.55));
        }

        if refuse {
            renderer.update_line_verts(&gpu, &frozen_verts);
        } else {
            renderer.update_line_verts(&gpu, &verts);
            frozen_verts = verts;
            frozen_chi2 = chi2;
            frozen_beat = beat;
        }

        if hubs_on {
            for h in &pent {
                let m = Mat4::from_translation(glam::Vec3::from(h.pos))
                    * Mat4::from_scale(Vec3::splat(h.radius.max(0.04)));
                renderer.draw_geodesic_orb_alpha(m, CYAN, 1.0);
            }
        }
        if phase == 2 || (refuse && frozen_beat == "ASSOCIATE") {
            let (m, a) = midplane_orb(theta);
            renderer.draw_geodesic_orb_alpha(m, GOLD, a);
        }

        if let Some(atlas) = atlas.as_ref() {
            if site_a > 0.02 {
                let _ = stamp_atlas(
                    &mut renderer,
                    atlas,
                    5,
                    1.0,
                    1.0,
                    2.0,
                    site_a,
                    None,
                    true,
                    true,
                    true,
                    false,
                    site_a,
                    false,
                    false,
                    false,
                );
            }
        }

        renderer.write_particles(&gpu, &[])?;
        renderer.write_hud(&gpu, &hang_hud(beat, chi2))?;
        let grab = capture_dir.is_some();
        if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, time, grab)? {
            if let Some(d) = capture_dir {
                write_bgra(
                    d,
                    &format!("frame_{i:04}"),
                    &frame.bgra,
                    frame.width,
                    frame.height,
                )?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    print_claim_banner("homology remesh / inner_cone film");
    let args = parse_args()?;
    match args.beat {
        Some(Beat::Caterpillar) => run_caterpillar(args),
        Some(Beat::CaterpillarGrow) => run_grow(args),
        Some(Beat::CaterpillarSkin) => run_skin(args),
        Some(Beat::CaterpillarChaeta) => run_chaeta(args),
        Some(Beat::HangChrysalis) => run_hang(args),
        None => run_stills(args),
    }
}
