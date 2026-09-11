//! Last mile on bud. One QGAE, four-bin paint, one BGRA.
//! Occupant = cards + hubs. Motes are flux, not the catalog.

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use qga_gpu::{
    hud_quad, hud_text, print_claim_banner, Camera, GpuContext, GpuHub, GpuParticle, HudVert,
    LineStyle, LineVert, Renderer, VisualState,
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
const U_STAR: f32 = 0.30;
const V_STAR: f32 = 0.0;
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
    u: f32,
    v: f32,
    pos: Vec3,
    vel: Vec3,
    hue: f32,
    leak: bool,
}

fn parse_args() -> Result<Args> {
    let mut frames = 1u32;
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
            "--capture" => capture = Some(PathBuf::from(it.next().context("--capture DIR")?)),
            "--beat" => beat = Some(it.next().context("--beat NAME")?),
            "--field" => field = Some(PathBuf::from(it.next().context("--field DIR")?)),
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
        jobs,
        capture,
        beat,
        field,
        compare,
    })
}

struct Args {
    frames: u32,
    jobs: Vec<Job>,
    capture: Option<PathBuf>,
    beat: Option<Beat>,
    field: Option<PathBuf>,
    compare: Option<PathBuf>,
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

fn catalog_to_verts(segs: &[CatalogSeg]) -> Vec<LineVert> {
    segs.iter()
        .flat_map(|s| {
            [
                LineVert {
                    pos: s.a,
                    pad: 0.0,
                    color: s.color,
                },
                LineVert {
                    pos: s.b,
                    pad: 0.0,
                    color: s.color,
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

fn stamp_hubs(renderer: &mut Renderer, cat: &Catalog, hex_alpha: f32, mid: Option<(Vec3, f32)>) {
    for h in &cat.pent {
        let m = Mat4::from_translation(Vec3::from(h.pos)) * Mat4::from_scale(Vec3::splat(h.radius));
        renderer.draw_geodesic_orb_alpha(m, CYAN, 1.0);
    }
    if hex_alpha > 1e-4 {
        for h in &cat.hex {
            let m =
                Mat4::from_translation(Vec3::from(h.pos)) * Mat4::from_scale(Vec3::splat(h.radius));
            renderer.draw_geodesic_orb_alpha(m, ORANGE, hex_alpha.clamp(0.02, 1.0));
        }
    }
    if let Some((p, a)) = mid {
        let m = Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.07));
        renderer.draw_geodesic_orb_alpha(m, GOLD, a.clamp(0.02, 1.0));
    }
}

fn upload_breath(renderer: &mut Renderer, gpu: &GpuContext, cat: &Catalog) -> Result<()> {
    let hubs: Vec<GpuHub> = cat
        .pent
        .iter()
        .map(|h| GpuHub::new(Vec3::from(h.pos), h.radius, CYAN))
        .collect();
    renderer.upload_hubs(gpu, &hubs)?;
    Ok(())
}

fn seed_motes(cat: &Catalog) -> Vec<Mote> {
    let mut motes = Vec::new();
    for k in 0..8 {
        let v = -std::f32::consts::PI + 2.0 * std::f32::consts::PI * k as f32 / 8.0;
        for i in 0..32 {
            if motes.len() >= MOTE_CAP {
                return motes;
            }
            let u = -1.0 + 2.0 * i as f32 / 31.0;
            let pos = helicoid(u, v);
            let bits = nearest_section(&cat.cents, &cat.faces, pos.to_array());
            motes.push(Mote {
                u,
                v,
                pos,
                vel: Vec3::ZERO,
                hue: SECTION_HUE[bits],
                leak: bits == 2,
            });
        }
    }
    motes
}

fn step_motes(motes: &mut [Mote], cat: &Catalog, theta: f32, dtheta: f32) {
    for m in motes.iter_mut() {
        if m.leak {
            m.u = (m.u + 0.01 * dtheta.signum()).clamp(-1.4, 1.4);
        } else {
            m.u *= 0.985;
        }
        let next = associate(m.u, m.v, theta);
        m.vel = (next - m.pos) / 0.016_f32.max(1e-4);
        m.pos = next;
        let bits = nearest_section(&cat.cents, &cat.faces, m.pos.to_array());
        m.hue = SECTION_HUE[bits];
        m.leak = bits == 2;
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
    let mut gpu = GpuContext::init_headless().context("init_headless")?;
    let mut renderer = Renderer::new(&gpu)?;
    let camera = Camera::orbit(Vec3::ZERO, 4.2);
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
    let field = args
        .field
        .as_deref()
        .context("--beat caterpillar needs --field DIR")?;
    let cat = load_catalog(field)?;
    let occ = match args.compare.as_deref() {
        Some(p) => Some(load_occupancy(p).map_err(|e| anyhow::anyhow!("{e}"))?),
        None => None,
    };

    let s2 = job_edges(&args.jobs, Bin::S2).unwrap_or_default();
    let t2 = job_edges(&args.jobs, Bin::T2).unwrap_or_default();
    let k2 = job_edges(&args.jobs, Bin::K2).unwrap_or_default();
    let p2 = job_edges(&args.jobs, Bin::P2).unwrap_or_default();
    let hel = job_edges(&args.jobs, Bin::Helicoid).unwrap_or_default();
    let catn = job_edges(&args.jobs, Bin::Catenoid).unwrap_or_default();
    let hyp = job_edges(&args.jobs, Bin::Hyperboloid).context("--beat needs --hyperboloid")?;

    let mut gpu = GpuContext::init_headless().context("init_headless")?;
    let mut renderer = Renderer::new(&gpu)?;
    let camera = Camera::orbit(Vec3::ZERO, 4.2);
    let mut vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        zener: 2.4,
        ..VisualState::default()
    };

    let frames = args.frames.max(6);
    let beat_len = (frames / 6).max(1);
    let mut motes = seed_motes(&cat);
    let mut last_theta = 0.0f32;
    let mut family = 0usize;
    let mut cage_t = -1.0f32;
    let mut cage_dir = 1.0f32;

    let capture_dir = args.capture.as_deref();
    if let Some(dir) = capture_dir {
        std::fs::create_dir_all(dir)?;
    }

    for i in 0..frames {
        let beat_i = (i / beat_len).min(5);
        let local = i - beat_i * beat_len;
        let frac = local as f32 / beat_len as f32;
        let time = i as f32 * 0.016;
        vis.pulse = 0.2 + 0.25 * (time * 1.1).sin().abs();
        vis.zener = 2.4 + 0.15 * (time * 0.7).sin();

        let (beat_name, theta, hex_a, show_mid, show_motes, freeze) = match beat_i {
            0 => ("GENERATORS", 0.0, 0.0, false, false, false),
            1 => ("CATALOG", 0.0, 0.0, false, false, false),
            2 => (
                "LENS",
                frac * std::f32::consts::FRAC_PI_2,
                0.0,
                true,
                false,
                false,
            ),
            3 => ("CAGE", std::f32::consts::FRAC_PI_2, 0.0, true, false, false),
            4 => (
                "LIFE",
                frac * std::f32::consts::FRAC_PI_2,
                POLY_ALPHA,
                true,
                true,
                false,
            ),
            _ => (
                "REFUSE",
                std::f32::consts::FRAC_PI_2,
                POLY_ALPHA,
                true,
                true,
                true,
            ),
        };

        let mut verts: Vec<LineVert> = Vec::new();
        match beat_i {
            0 => {
                let nshow = 1 + (frac * 4.0).floor() as usize;
                if nshow >= 1 {
                    verts.extend(edges_to_verts(&s2, Bin::S2.rgb()));
                }
                if nshow >= 2 {
                    verts.extend(edges_to_verts(&t2, Bin::T2.rgb()));
                }
                if nshow >= 3 {
                    verts.extend(edges_to_verts(&k2, Bin::K2.rgb()));
                }
                if nshow >= 4 {
                    verts.extend(edges_to_verts(&p2, Bin::P2.rgb()));
                }
            }
            1 => {
                verts.extend(edges_to_verts(&s2, Bin::S2.rgb()));
                verts.extend(catalog_to_verts(&cat.segs));
            }
            2 => {
                verts.extend(edges_to_verts(&hel, Bin::Helicoid.rgb()));
                verts.extend(edges_to_verts(&catn, Bin::Catenoid.rgb()));
            }
            3 => {
                verts.extend(edges_to_verts(&hyp, Bin::Hyperboloid.rgb()));
                verts.extend(catalog_to_verts(&cat.segs));
            }
            _ => {
                verts.extend(catalog_to_verts(&cat.segs));
                verts.extend(edges_to_verts(&hel, Bin::Helicoid.rgb()));
                verts.extend(edges_to_verts(&catn, Bin::Catenoid.rgb()));
            }
        }
        renderer.update_line_verts(&gpu, &verts);

        let dtheta = theta - last_theta;
        last_theta = theta;
        let dp = d_associate(U_STAR, V_STAR, theta);
        let mid_a = if show_mid {
            (dp.length() * 2.0 + 0.15).clamp(0.02, 1.0)
        } else {
            0.0
        };
        let mid = if show_mid {
            Some((associate(U_STAR, V_STAR, theta), mid_a))
        } else {
            None
        };

        if beat_i == 3 {
            cage_t += cage_dir * 0.08;
            if cage_t.abs() > 1.15 {
                cage_dir *= -1.0;
                family = 1 - family;
            }
            let p = hyperboloid_point(family, 0.4, cage_t);
            renderer.draw_geodesic_orb_alpha(
                Mat4::from_translation(p) * Mat4::from_scale(Vec3::splat(0.05)),
                GOLD,
                0.9,
            );
        }

        if beat_i >= 1 {
            stamp_hubs(&mut renderer, &cat, hex_a, mid);
            upload_breath(&mut renderer, &gpu, &cat)?;
        } else {
            draw_stub_hubs(&mut renderer);
        }

        if show_motes {
            if !freeze {
                step_motes(&mut motes, &cat, theta, dtheta);
            }
            renderer.write_particles(&gpu, &gpu_motes(&motes))?;
        } else {
            renderer.write_particles(&gpu, &[])?;
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
