//! Last mile on bud. One QGAE, four-bin paint, one BGRA.
//! Occupant = cards + hubs. Motes are flux, not the catalog.

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use qga_gpu::{
    hud_quad, hud_text, print_claim_banner, Camera, GpuContext, GpuParticle, HudVert, LineStyle,
    LineVert, Renderer, VisualState,
};
use qga_swarm_convert::{
    catalog_line_verts, face_centroids, glam_edges, hexavalent_hubs, load_net_json, load_occupancy,
    load_qga_pixel_field, load_qgae, nearest_section, pentavalent_hubs, CatalogSeg, Hub,
    OccupancyCard, PixelFace, SECTION_HUE,
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
            "--field" | "--larva" => {
                if field.is_some() {
                    bail!("do not mix --field and --larva");
                }
                field = Some(PathBuf::from(it.next().context("--field/--larva PATH")?));
            }
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
        Some(other) => bail!("unknown --beat {other}; expected caterpillar"),
    };
    if beat.is_none() && jobs.is_empty() {
        bail!("need --lines FILE or --s2/--t2/--k2/--p2/--helicoid/--catenoid/--theta");
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
        if dir.join("net.json").is_file() && dir.join("qga_pixel_field.bin").is_file() {
            return Ok(dir);
        }
    }
    bail!(
        "catalog dump not found at {} (need net.json + qga_pixel_field.bin)",
        raw.display()
    )
}

fn init_gpu(width: u32, height: u32) -> Result<GpuContext> {
    GpuContext::init_headless_extent(width, height).context("init_headless")
}

#[derive(Clone, Copy)]
enum Beat {
    Caterpillar,
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
    plate_hud("CATALOG", None, "GENERATORS", 0.0)
}

fn plate_hud(
    left_title: &str,
    occ: Option<&OccupancyCard>,
    beat: &str,
    theta: f32,
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

    hud_quad(&mut v, 0.38, 0.50, 0.96, 0.94, PANEL);
    hud_text(&mut v, 0.40, 0.90, 0.016, "REFUSE", GOLD_A);
    hud_text(&mut v, 0.40, 0.84, 0.014, "HYPOTHESIS / MODEL", INK);
    hud_text(&mut v, 0.40, 0.78, 0.014, "FACEPLATE UNUSED", INK);
    hud_text(&mut v, 0.40, 0.72, 0.014, "CATALOG CANNOT", INK);
    hud_text(&mut v, 0.40, 0.66, 0.014, "PROVE OCCUPANT", INK);

    hud_quad(&mut v, -0.22, -0.92, 0.08, -0.72, VENN_A);
    hud_quad(&mut v, -0.08, -0.92, 0.22, -0.72, VENN_B);
    hud_text(&mut v, -0.20, -0.70, 0.012, "CATALOG", INK);
    hud_text(&mut v, 0.02, -0.70, 0.012, "LIFE", INK);

    hud_text(&mut v, -0.20, 0.46, 0.014, beat, GOLD_A);
    let tline = format!("THETA {:.2}  CHI=2", theta);
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
        .field
        .as_deref()
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

        renderer.write_hud(&gpu, &plate_hud("CATALOG", occ.as_ref(), beat_name, theta))?;

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

fn main() -> Result<()> {
    print_claim_banner("homology remesh / inner_cone film");
    let args = parse_args()?;
    match args.beat {
        Some(Beat::Caterpillar) => run_caterpillar(args),
        None => run_stills(args),
    }
}
