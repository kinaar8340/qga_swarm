//! Last mile on bud. Lines + hubs. No particles. No ocean.

use anyhow::{bail, Context, Result};
use glam::{Mat4, Vec3};
use qga_gpu::{
    print_claim_banner, Camera, GpuContext, LineStyle, Renderer, VisualState,
};
use qga_swarm_convert::{glam_edges, load_qgae};
use std::path::{Path, PathBuf};

fn cyan() -> Vec3 {
    Vec3::new(0.2, 0.6, 1.0)
}

fn parse_args() -> Result<(u32, PathBuf, Option<PathBuf>)> {
    let mut frames = 1u32;
    let mut lines: Option<PathBuf> = None;
    let mut capture: Option<PathBuf> = None;
    let mut headless = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--headless" => headless = true,
            "--frames" => {
                frames = it.next().context("--frames N")?.parse()?;
            }
            "--lines" => {
                lines = Some(PathBuf::from(it.next().context("--lines FILE")?));
            }
            "--capture" => {
                capture = Some(PathBuf::from(it.next().context("--capture DIR")?));
            }
            other => bail!("unknown arg {other}"),
        }
    }
    if !headless {
        bail!("pass-1 is --headless only");
    }
    Ok((
        frames.max(1),
        lines.context("--lines FILE")?,
        capture,
    ))
}

fn write_bgra(dir: &Path, bgra: &[u8], w: u32, h: u32) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("last.bgra");
    std::fs::write(&path, bgra)?;
    std::fs::write(
        dir.join("last.txt"),
        format!("bgra {w}x{h} bytes={}\n", bgra.len()),
    )?;
    Ok(())
}

fn main() -> Result<()> {
    print_claim_banner("homology remesh / inner_cone film");
    let (frames, lines, capture) = parse_args()?;
    let parcel = load_qgae(&lines).map_err(|e| anyhow::anyhow!("{e}"))?;
    let edges = glam_edges(&parcel);

    let mut gpu = GpuContext::init_headless().context("init_headless")?;
    let mut renderer = Renderer::new(&gpu)?;
    let camera = Camera::orbit(Vec3::ZERO, 4.2);
    let vis = VisualState {
        glow: 0.4,
        pulse: 0.2,
        tube_radius: 0.02,
        ..VisualState::default()
    };

    renderer.update_line_segments(
        &gpu,
        &edges,
        LineStyle {
            color: cyan(),
            opacity: 1.0,
            width: 1.0,
            depth_bias: 0.0,
        },
    );
    renderer.write_hud(&gpu, &[])?;

    let mut last: Option<(Vec<u8>, u32, u32)> = None;
    for i in 0..frames {
        renderer.draw_geodesic_orb(Mat4::from_scale(Vec3::splat(0.06)), cyan(), 1);
        renderer.draw_geodesic_orb(
            Mat4::from_translation(Vec3::Y * 0.9) * Mat4::from_scale(Vec3::splat(0.04)),
            cyan(),
            1,
        );
        let grab = capture.is_some() && i + 1 == frames;
        if let Some(frame) = renderer.render(&mut gpu, &camera, &vis, i as f32 * 0.016, grab)? {
            last = Some((frame.bgra, frame.width, frame.height));
        }
    }

    if let (Some(dir), Some((bgra, w, h))) = (capture.as_deref(), last) {
        write_bgra(dir, &bgra, w, h)?;
    }
    Ok(())
}
