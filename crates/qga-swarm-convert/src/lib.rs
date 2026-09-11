//! QGAE edge files + shellscan dumps → wire parcels. No qga-gpu. No wgpu.
//!
//! Do not alias qga_pixel onto GpuParticle. Occupant is hubs + cards.

use std::path::Path;

pub const MAGIC: &[u8; 4] = b"QGAE";
pub const VERSION: u32 = 1;

pub const SECTION_NAMES: [&str; 4] = ["elliptic", "parabolic", "hyperbolic", "flat-pockets"];

/// wrap.py witness RGB + alpha. Four-bin. Not a fifth hue.
pub const SECTION_RGBA: [[f32; 4]; 4] = [
    [0.20, 0.60, 1.00, 1.00],
    [1.00, 0.75, 0.20, 1.00],
    [1.00, 0.40, 0.20, 1.00],
    [1.00, 0.20, 0.80, 1.00],
];

/// SPEC.md GpuParticle.pad hues.
pub const SECTION_HUE: [f32; 4] = [0.55, 0.10, 0.30, 0.80];

/// Species preview witness (monarch black / yellow / white / magenta).
/// Same four bins as SECTION_RGBA. Not a fifth hue. Field dump still stores section.
pub const SPECIES_RGBA: [[f32; 4]; 4] = [
    [0.08, 0.08, 0.08, 1.00],
    [1.00, 0.75, 0.20, 1.00],
    [0.92, 0.92, 0.90, 1.00],
    [1.00, 0.20, 0.80, 1.00],
];

pub const SEGMENTS: [&str; 13] = [
    "T1", "T2", "T3", "A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "A10",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hub {
    pub pos: [f32; 3],
    pub radius: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WireParcel {
    pub edges: Vec<[[f32; 3]; 2]>,
    pub hubs: Vec<Hub>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PixelFace {
    pub theta: f32,
    pub phi: f32,
    pub psi: f32,
    pub offset: f32,
    pub amplitude: f32,
    pub shell_s: f32,
    pub persist: f32,
    pub packed: u32,
}

impl PixelFace {
    pub fn section_bits(self) -> usize {
        ((self.packed >> 1) & 3) as usize
    }

    pub fn section_name(self) -> &'static str {
        SECTION_NAMES[self.section_bits()]
    }

    pub fn rgba(self) -> [f32; 4] {
        SECTION_RGBA[self.section_bits()]
    }

    pub fn hue(self) -> f32 {
        SECTION_HUE[self.section_bits()]
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Net {
    pub kind: String,
    pub m: i32,
    pub n: i32,
    pub t: i32,
    pub verts: Vec<[f32; 3]>,
    pub faces: Vec<Vec<u32>>,
    pub n_phi: u32,
    pub n_segments: u32,
    pub twist: f32,
    pub radius: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetalSite {
    pub kind: String,
    pub segment: String,
    pub segment_index: i32,
    pub phi_deg: f32,
    pub section: usize,
    pub amplitude: f32,
    pub tentacle: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChaetaSite {
    pub id: String,
    pub segment: String,
    pub seta: String,
    pub s: f32,
    pub phi_deg: f32,
    pub amp: f32,
    pub group: String,
    pub kind: String,
    pub instar: u8,
    pub phi_hinton: Option<f32>,
    pub phi_measured: Option<f32>,
}

impl ChaetaSite {
    pub fn section(&self) -> usize {
        match self.group.as_str() {
            "XD" | "D" => 0,
            "SD" => 1,
            "L" => 2,
            "SV" | "V" => 3,
            _ => match self.kind.as_str() {
                "tentacle" | "spiracle" => 1,
                _ => 0,
            },
        }
    }

    pub fn primary(&self) -> bool {
        matches!(
            self.seta.as_str(),
            "XD1" | "XD2" | "D1" | "D2" | "SD1" | "L1" | "SV1" | "V1"
        ) && self.kind == "seta"
    }

    pub fn subprimary(&self) -> bool {
        matches!(self.seta.as_str(), "SD2" | "L2" | "L3" | "SV2" | "SV3")
    }

    pub fn mirror(&self) -> bool {
        !self.seta.starts_with('V') && self.group != "V"
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chaetotaxy {
    pub n_phi: u32,
    pub rings_l5: u32,
    pub dphi_rms: f32,
    pub phi_order_ok: bool,
    pub sites: Vec<ChaetaSite>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatalogSeg {
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OccupancyCard {
    pub section_agree: f32,
    pub a: String,
    pub b: String,
}

#[derive(Debug)]
pub enum ConvertError {
    Io(std::io::Error),
    NotQgae { got: [u8; 4] },
    PixelAlias,
    BadVersion(u32),
    Truncated,
    Json(String),
    BadNet(&'static str),
    FieldLen { bytes: usize },
    FaceMismatch { pixels: usize, faces: usize },
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertError::Io(e) => write!(f, "{e}"),
            ConvertError::NotQgae { got } => write!(f, "not QGAE magic: {got:?}"),
            ConvertError::PixelAlias => {
                write!(
                    f,
                    "32-byte file without QGAE; will not alias qga_pixel onto GpuParticle"
                )
            }
            ConvertError::BadVersion(v) => write!(f, "unsupported QGAE version {v}"),
            ConvertError::Truncated => write!(f, "truncated QGAE file"),
            ConvertError::Json(s) => write!(f, "json: {s}"),
            ConvertError::BadNet(s) => write!(f, "bad net.json: {s}"),
            ConvertError::FieldLen { bytes } => {
                write!(f, "qga_pixel_field length {bytes} not a multiple of 32")
            }
            ConvertError::FaceMismatch { pixels, faces } => {
                write!(f, "pixel faces {pixels} != net faces {faces}")
            }
        }
    }
}

impl std::error::Error for ConvertError {}
impl From<std::io::Error> for ConvertError {
    fn from(e: std::io::Error) -> Self {
        ConvertError::Io(e)
    }
}

pub fn load_qgae(path: &Path) -> Result<WireParcel, ConvertError> {
    parse_qgae(&std::fs::read(path)?)
}

pub fn parse_qgae(bytes: &[u8]) -> Result<WireParcel, ConvertError> {
    if bytes.len() == 32 && bytes.get(0..4) != Some(&MAGIC[..]) {
        return Err(ConvertError::PixelAlias);
    }
    if bytes.len() < 12 {
        return Err(ConvertError::Truncated);
    }
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if &magic != MAGIC {
        return Err(ConvertError::NotQgae { got: magic });
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != VERSION {
        return Err(ConvertError::BadVersion(version));
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let need = 12 + n * 24;
    if bytes.len() < need {
        return Err(ConvertError::Truncated);
    }
    let mut edges = Vec::with_capacity(n);
    let mut off = 12;
    for _ in 0..n {
        let mut pts = [[0f32; 3]; 2];
        for end in 0..2 {
            for k in 0..3 {
                pts[end][k] = f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
                off += 4;
            }
        }
        edges.push(pts);
    }
    Ok(WireParcel {
        edges,
        hubs: Vec::new(),
    })
}

pub fn glam_edges(parcel: &WireParcel) -> Vec<[glam::Vec3; 2]> {
    parcel
        .edges
        .iter()
        .map(|e| [glam::Vec3::from(e[0]), glam::Vec3::from(e[1])])
        .collect()
}

pub fn parse_qga_pixel_field(bytes: &[u8]) -> Result<Vec<PixelFace>, ConvertError> {
    if bytes.len() % 32 != 0 {
        return Err(ConvertError::FieldLen { bytes: bytes.len() });
    }
    let mut out = Vec::with_capacity(bytes.len() / 32);
    for chunk in bytes.chunks_exact(32) {
        out.push(PixelFace {
            theta: f32::from_le_bytes(chunk[0..4].try_into().unwrap()),
            phi: f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
            psi: f32::from_le_bytes(chunk[8..12].try_into().unwrap()),
            offset: f32::from_le_bytes(chunk[12..16].try_into().unwrap()),
            amplitude: f32::from_le_bytes(chunk[16..20].try_into().unwrap()),
            shell_s: f32::from_le_bytes(chunk[20..24].try_into().unwrap()),
            persist: f32::from_le_bytes(chunk[24..28].try_into().unwrap()),
            packed: u32::from_le_bytes(chunk[28..32].try_into().unwrap()),
        });
    }
    Ok(out)
}

pub fn load_qga_pixel_field(path: &Path) -> Result<Vec<PixelFace>, ConvertError> {
    parse_qga_pixel_field(&std::fs::read(path)?)
}

fn json_err(e: serde_json::Error) -> ConvertError {
    ConvertError::Json(e.to_string())
}

fn as_vec3(v: &serde_json::Value) -> Result<[f32; 3], ConvertError> {
    let a = v.as_array().ok_or(ConvertError::BadNet("vert not array"))?;
    if a.len() < 3 {
        return Err(ConvertError::BadNet("vert short"));
    }
    Ok([
        a[0].as_f64().ok_or(ConvertError::BadNet("vert x"))? as f32,
        a[1].as_f64().ok_or(ConvertError::BadNet("vert y"))? as f32,
        a[2].as_f64().ok_or(ConvertError::BadNet("vert z"))? as f32,
    ])
}

pub fn parse_net_json(text: &str) -> Result<Net, ConvertError> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(json_err)?;
    let verts_v = v
        .get("verts")
        .and_then(|x| x.as_array())
        .ok_or(ConvertError::BadNet("missing verts"))?;
    let faces_v = v
        .get("faces")
        .and_then(|x| x.as_array())
        .ok_or(ConvertError::BadNet("missing faces"))?;
    let mut verts = Vec::with_capacity(verts_v.len());
    for p in verts_v {
        verts.push(as_vec3(p)?);
    }
    let mut faces = Vec::with_capacity(faces_v.len());
    for f in faces_v {
        let ring = f.as_array().ok_or(ConvertError::BadNet("face not array"))?;
        let mut idx = Vec::with_capacity(ring.len());
        for i in ring {
            let n = i
                .as_u64()
                .or_else(|| i.as_i64().map(|x| x as u64))
                .ok_or(ConvertError::BadNet("face index"))?;
            idx.push(n as u32);
        }
        faces.push(idx);
    }
    let n_phi = v.get("n_phi").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    let n_segments = v.get("n_segments").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    Ok(Net {
        kind: v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        m: v.get("m").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        n: v.get("n").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        t: v.get("T").and_then(|x| x.as_i64()).unwrap_or(0) as i32,
        verts,
        faces,
        n_phi,
        n_segments,
        twist: v.get("twist").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
        radius: v.get("radius").and_then(|x| x.as_f64()).unwrap_or(1.0) as f32,
        height: v.get("height").and_then(|x| x.as_f64()).unwrap_or(2.0) as f32,
    })
}

pub fn load_net_json(path: &Path) -> Result<Net, ConvertError> {
    parse_net_json(&std::fs::read_to_string(path)?)
}

fn face_centroid(net: &Net, face: &[u32]) -> [f32; 3] {
    let mut s = [0.0f32; 3];
    let mut n = 0.0f32;
    for &i in face {
        if let Some(v) = net.verts.get(i as usize) {
            s[0] += v[0];
            s[1] += v[1];
            s[2] += v[2];
            n += 1.0;
        }
    }
    if n < 1.0 {
        return [0.0, 0.0, 0.0];
    }
    [s[0] / n, s[1] / n, s[2] / n]
}

fn hubs_of_degree(net: &Net, deg: usize, radius: f32) -> Vec<Hub> {
    net.faces
        .iter()
        .filter(|f| f.len() == deg)
        .map(|f| Hub {
            pos: face_centroid(net, f),
            radius,
        })
        .collect()
}

pub fn pentavalent_hubs(net: &Net) -> Vec<Hub> {
    hubs_of_degree(net, 5, 0.05)
}

pub fn hexavalent_hubs(net: &Net) -> Vec<Hub> {
    hubs_of_degree(net, 6, 0.03)
}

pub fn face_centroids(net: &Net) -> Vec<[f32; 3]> {
    net.faces.iter().map(|f| face_centroid(net, f)).collect()
}

/// Unique undirected edges. Color from the first face that owns the edge.
pub fn catalog_line_verts(net: &Net, faces: &[PixelFace]) -> Result<Vec<CatalogSeg>, ConvertError> {
    catalog_line_verts_palette(net, faces, &SECTION_RGBA)
}

pub fn catalog_line_verts_species(
    net: &Net,
    faces: &[PixelFace],
) -> Result<Vec<CatalogSeg>, ConvertError> {
    catalog_line_verts_palette(net, faces, &SPECIES_RGBA)
}

pub fn catalog_line_verts_palette(
    net: &Net,
    faces: &[PixelFace],
    palette: &[[f32; 4]; 4],
) -> Result<Vec<CatalogSeg>, ConvertError> {
    if faces.len() != net.faces.len() {
        return Err(ConvertError::FaceMismatch {
            pixels: faces.len(),
            faces: net.faces.len(),
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (fi, ring) in net.faces.iter().enumerate() {
        if ring.len() < 2 {
            continue;
        }
        let color = palette[faces[fi].section_bits()];
        for k in 0..ring.len() {
            let i = ring[k];
            let j = ring[(k + 1) % ring.len()];
            let key = if i <= j { (i, j) } else { (j, i) };
            if !seen.insert(key) {
                continue;
            }
            let a = net.verts.get(i as usize).copied().unwrap_or([0.0; 3]);
            let b = net.verts.get(j as usize).copied().unwrap_or([0.0; 3]);
            out.push(CatalogSeg { a, b, color });
        }
    }
    Ok(out)
}

fn json_f32(v: &serde_json::Value) -> Option<f32> {
    v.as_f64().map(|x| x as f32)
}

pub fn parse_chaetotaxy(text: &str) -> Result<Chaetotaxy, ConvertError> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(json_err)?;
    let arr = v
        .get("sites")
        .and_then(|x| x.as_array())
        .ok_or(ConvertError::BadNet("chaetotaxy sites"))?;
    let mut sites = Vec::with_capacity(arr.len());
    for rec in arr {
        let instar = rec.get("instar").and_then(|x| x.as_u64()).unwrap_or(5) as u8;
        sites.push(ChaetaSite {
            id: rec
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            segment: rec
                .get("segment")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            seta: rec
                .get("seta")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            s: json_f32(rec.get("s").unwrap_or(&serde_json::Value::Null)).unwrap_or(0.5),
            phi_deg: json_f32(rec.get("phi_deg").unwrap_or(&serde_json::Value::Null))
                .unwrap_or(0.0),
            amp: json_f32(rec.get("amp").unwrap_or(&serde_json::Value::Null)).unwrap_or(1.0),
            group: rec
                .get("group")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            kind: rec
                .get("kind")
                .and_then(|x| x.as_str())
                .unwrap_or("seta")
                .to_string(),
            instar: instar.clamp(1, 5),
            phi_hinton: rec.get("phi_hinton").and_then(json_f32),
            phi_measured: rec.get("phi_measured").and_then(json_f32),
        });
    }
    Ok(Chaetotaxy {
        n_phi: v.get("n_phi").and_then(|x| x.as_u64()).unwrap_or(36) as u32,
        rings_l5: v.get("rings_l5").and_then(|x| x.as_u64()).unwrap_or(13) as u32,
        dphi_rms: json_f32(v.get("dphi_rms").unwrap_or(&serde_json::Value::Null)).unwrap_or(0.0),
        phi_order_ok: v
            .get("phi_order_ok")
            .and_then(|x| x.as_bool())
            .unwrap_or(true),
        sites,
    })
}

pub fn load_chaetotaxy(path: &Path) -> Result<Chaetotaxy, ConvertError> {
    parse_chaetotaxy(&std::fs::read_to_string(path)?)
}

pub fn load_setal_sites(dir: &Path) -> Result<Vec<SetalSite>, ConvertError> {
    let log_path = dir.join("setal_log.json");
    let painted_path = dir.join("painted.json");
    if !log_path.is_file() {
        return Ok(Vec::new());
    }
    let log: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&log_path)?).map_err(json_err)?;
    let painted: serde_json::Value = if painted_path.is_file() {
        serde_json::from_str(&std::fs::read_to_string(&painted_path)?).map_err(json_err)?
    } else {
        serde_json::Value::Array(vec![])
    };
    let mut face_section: std::collections::HashMap<u64, (usize, f32)> =
        std::collections::HashMap::new();
    if let Some(arr) = painted.as_array() {
        for rec in arr {
            let i = rec.get("i").and_then(|x| x.as_u64()).unwrap_or(0);
            let name = rec
                .get("section")
                .and_then(|x| x.as_str())
                .unwrap_or("elliptic");
            let bits = SECTION_NAMES.iter().position(|s| *s == name).unwrap_or(0);
            let amp = rec.get("amplitude").and_then(|x| x.as_f64()).unwrap_or(1.0) as f32;
            face_section.insert(i, (bits, amp));
        }
    }
    let mut out = Vec::new();
    let arr = log
        .as_array()
        .ok_or(ConvertError::BadNet("setal_log not array"))?;
    for rec in arr {
        let segment = rec
            .get("segment")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let kind = rec
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("seta")
            .to_string();
        let idx = SEGMENTS
            .iter()
            .position(|s| *s == segment.as_str())
            .map(|i| i as i32)
            .unwrap_or(-1);
        let face = rec.get("face").and_then(|x| x.as_u64()).unwrap_or(0);
        let (section, amp) = face_section.get(&face).copied().unwrap_or((0, 1.0));
        out.push(SetalSite {
            tentacle: kind == "tentacle",
            kind,
            segment,
            segment_index: idx,
            phi_deg: rec.get("phi_deg").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
            section,
            amplitude: amp,
        });
    }
    Ok(out)
}

pub fn load_occupancy(path: &Path) -> Result<OccupancyCard, ConvertError> {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path)?).map_err(json_err)?;
    Ok(OccupancyCard {
        section_agree: v
            .get("section_agree")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0) as f32,
        a: v.get("a")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        b: v.get("b")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

pub fn nearest_section(cents: &[[f32; 3]], faces: &[PixelFace], p: [f32; 3]) -> usize {
    let mut best = 0usize;
    let mut best_d = f32::MAX;
    for (i, c) in cents.iter().enumerate() {
        let dx = c[0] - p[0];
        let dy = c[1] - p[1];
        let dz = c[2] - p[2];
        let d = dx * dx + dy * dy + dz * dz;
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    faces.get(best).map(|f| f.section_bits()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(edges: &[[[f32; 3]; 2]]) -> Vec<u8> {
        let mut b = Vec::from(*MAGIC);
        b.extend_from_slice(&VERSION.to_le_bytes());
        b.extend_from_slice(&(edges.len() as u32).to_le_bytes());
        for e in edges {
            for p in e {
                for c in p {
                    b.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
        b
    }

    fn pack_pixel(section: u32) -> [u8; 32] {
        let mut b = [0u8; 32];
        b[28..32].copy_from_slice(&(section << 1).to_le_bytes());
        b
    }

    #[test]
    fn roundtrip_one_edge() {
        let src = [[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]];
        let p = parse_qgae(&pack(&src)).unwrap();
        assert_eq!(p.edges, src);
        assert!(p.hubs.is_empty());
    }

    #[test]
    fn reject_32_byte_non_magic() {
        let err = parse_qgae(&[0u8; 32]).unwrap_err();
        assert!(matches!(err, ConvertError::PixelAlias));
    }

    #[test]
    fn reject_wrong_magic() {
        let err = parse_qgae(b"NOPE\x01\x00\x00\x00\x00\x00\x00\x00").unwrap_err();
        assert!(matches!(err, ConvertError::NotQgae { .. }));
    }

    #[test]
    fn pixel_field_section_bits() {
        let mut blob = Vec::new();
        blob.extend_from_slice(&pack_pixel(0));
        blob.extend_from_slice(&pack_pixel(2));
        let faces = parse_qga_pixel_field(&blob).unwrap();
        assert_eq!(faces.len(), 2);
        assert_eq!(faces[0].section_name(), "elliptic");
        assert_eq!(faces[1].section_name(), "hyperbolic");
        assert!((faces[0].hue() - 0.55).abs() < 1e-6);
        assert!((faces[1].hue() - 0.30).abs() < 1e-6);
    }

    #[test]
    fn pixel_field_rejects_odd_len() {
        let err = parse_qga_pixel_field(&[0u8; 31]).unwrap_err();
        assert!(matches!(err, ConvertError::FieldLen { bytes: 31 }));
    }

    #[test]
    fn pentavalent_hubs_on_tiny_net() {
        let net = Net {
            kind: "goldberg".into(),
            m: 1,
            n: 1,
            t: 3,
            verts: vec![
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ],
            faces: vec![vec![0, 1, 4, 3, 2], vec![0, 1, 5, 3, 2, 4]],
            n_phi: 0,
            n_segments: 0,
            twist: 0.0,
            radius: 1.0,
            height: 2.0,
        };
        let pent = pentavalent_hubs(&net);
        let hex = hexavalent_hubs(&net);
        assert_eq!(pent.len(), 1);
        assert_eq!(hex.len(), 1);
        let pix = vec![
            PixelFace::default(),
            PixelFace {
                packed: 2 << 1,
                ..PixelFace::default()
            },
        ];
        let segs = catalog_line_verts(&net, &pix).unwrap();
        assert!(!segs.is_empty());
        assert_eq!(segs[0].color, SECTION_RGBA[0]);
    }

    #[test]
    fn occupancy_from_compare_json() {
        let t = r#"{"n_faces":72,"section_agree":0.16666666666666666,"a":"capsid-t7-p22","b":"capsid-t7-polyoma"}"#;
        let v: serde_json::Value = serde_json::from_str(t).unwrap();
        assert!((v["section_agree"].as_f64().unwrap() - 1.0 / 6.0).abs() < 1e-9);
    }

    #[test]
    fn species_preview_is_four_bins() {
        assert_eq!(SPECIES_RGBA.len(), 4);
        assert!(SPECIES_RGBA[0][0] < 0.15);
        assert!(SPECIES_RGBA[2][0] > 0.8);
        assert_eq!(SECTION_RGBA.len(), SPECIES_RGBA.len());
    }

    #[test]
    fn chaetotaxy_roundtrip_one_site() {
        let t = r#"{
            "n_phi": 36,
            "rings_l5": 13,
            "phi_order_ok": true,
            "dphi_rms": 4.2,
            "sites": [{
                "id": "T2.D1",
                "segment": "T2",
                "seta": "D1",
                "s": 0.18,
                "phi_deg": 10,
                "amp": 0.7,
                "group": "D",
                "kind": "seta",
                "instar": 2,
                "phi_hinton": 10,
                "phi_measured": 12
            }]
        }"#;
        let a = parse_chaetotaxy(t).unwrap();
        assert_eq!(a.n_phi, 36);
        assert_eq!(a.sites.len(), 1);
        assert_eq!(a.sites[0].instar, 2);
        assert_eq!(a.sites[0].section(), 0);
        assert!(a.sites[0].primary());
        assert!(a.sites[0].mirror());
        assert!((a.dphi_rms - 4.2).abs() < 1e-6);
    }
}
