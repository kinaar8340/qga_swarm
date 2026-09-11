//! Last mile on bud. One QGAE, one hue, one BGRA. No particles. No ocean.

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use qga_gpu::{print_claim_banner, Camera, GpuContext, LineStyle, Renderer, VisualState};
use qga_swarm_convert::{glam_edges, load_qgae};
use std::path::{Path, PathBuf};

const CYAN: Vec3 = Vec3::new(0.20, 0.60, 1.00);
const GOLD: Vec3 = Vec3::new(1.00, 0.78, 0.38);
const ORANGE: Vec3 = Vec3::new(1.00, 0.40, 0.20);
const MAGENTA: Vec3 = Vec3::new(0.85, 0.25, 0.80);

#[derive(Clone, Copy)]
enum Bin {
    S2,
    T2,
    K2,
    P2,
}

impl Bin {
    fn rgb(self) -> Vec3 {
        match self {
            Bin::S2 => CYAN,
            Bin::T2 => GOLD,
            Bin::K2 => ORANGE,
            Bin::P2 => MAGENTA,
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Bin::S2 => "s2",
            Bin::T2 => "t2",
            Bin::K2 => "k2",
            Bin::P2 => "p2",
        }
    }

    fn from_stem(name: &str) -> Option<Self> {
        let s = name.to_ascii_lowercase();
        if s.starts_with("s2") {
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

fn parse_args() -> Result<(u32, Vec<Job>, Option<PathBuf>)> {
    let mut frames = 1u32;
    let mut named: Vec<Job> = Vec::new();
    let mut lines: Vec<PathBuf> = Vec::new();
    let mut capture: Option<PathBuf> = None;
    let mut headless = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--headless" => headless = true,
            "--frames" => frames = it.next().context("--frames N")?.parse()?,
            "--capture" => capture = Some(PathBuf::from(it.next().context("--capture DIR")?)),
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
            "--lines" => lines.push(PathBuf::from(it.next().context("--lines FILE")?)),
            other => bail!("unknown arg {other}"),
        }
    }
    if !headless {
        bail!("pass-2 is --headless only");
    }
    if !named.is_empty() && !lines.is_empty() {
        bail!("do not mix --lines with --s2/--t2/--k2/--p2");
    }
    let mut jobs = named;
    if jobs.is_empty() {
        for p in lines {
            let stem = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let bin = Bin::from_stem(stem).unwrap_or(Bin::S2);
            jobs.push(Job { path: p, bin });
        }
    }
    if jobs.is_empty() {
        bail!("need --lines FILE or --s2/--t2/--k2/--p2");
    }
    jobs.sort_by_key(|j| match j.bin {
        Bin::S2 => 0,
        Bin::T2 => 1,
        Bin::K2 => 2,
        Bin::P2 => 3,
    });
    Ok((frames.max(1), jobs, capture))
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

fn draw_hubs(renderer: &mut Renderer) {
    renderer.draw_geodesic_orb(Mat4::from_scale(Vec3::splat(0.06)), CYAN, 1);
    renderer.draw_geodesic_orb(
        Mat4::from_translation(Vec3::Y * 0.9) * Mat4::from_scale(Vec3::splat(0.04)),
        CYAN,
        1,
    );
}

fn main() -> Result<()> {
    print_claim_banner("homology remesh / inner_cone film");
    let (frames, jobs, capture) = parse_args()?;
    let multi = jobs.len() > 1;

    let mut gpu = GpuContext::init_headless().context("init_headless")?;
    let mut renderer = Renderer::new(&gpu)?;
    let camera = Camera::orbit(Vec3::ZERO, 4.2);
    let vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        ..VisualState::default()
    };

    for job in &jobs {
        let parcel = load_qgae(&job.path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let edges = glam_edges(&parcel);
        renderer.update_line_segments(&gpu, &edges, style(job.bin.rgb()));
        renderer.write_hud(&gpu, &[])?;

        let mut last: Option<(Vec<u8>, u32, u32)> = None;
        for i in 0..frames {
            draw_hubs(&mut renderer);
            let grab = capture.is_some() && i + 1 == frames;
            if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, i as f32 * 0.016, grab)? {
                last = Some((frame.bgra, frame.width, frame.height));
            }
        }
        if let (Some(dir), Some((bgra, w, h))) = (capture.as_deref(), last) {
            let name = if multi { job.bin.stem() } else { "last" };
            write_bgra(dir, name, &bgra, w, h)?;
        }
    }
    Ok(())
}
