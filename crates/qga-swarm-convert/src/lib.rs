//! QGAE edge files → wire parcels. No qga-gpu. No wgpu.

use std::path::Path;

pub const MAGIC: &[u8; 4] = b"QGAE";
pub const VERSION: u32 = 1;

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

#[derive(Debug)]
pub enum ConvertError {
    Io(std::io::Error),
    NotQgae { got: [u8; 4] },
    PixelAlias,
    BadVersion(u32),
    Truncated,
}

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvertError::Io(e) => write!(f, "{e}"),
            ConvertError::NotQgae { got } => write!(f, "not QGAE magic: {got:?}"),
            ConvertError::PixelAlias => {
                write!(f, "32-byte file without QGAE; will not alias qga_pixel onto GpuParticle")
            }
            ConvertError::BadVersion(v) => write!(f, "unsupported QGAE version {v}"),
            ConvertError::Truncated => write!(f, "truncated QGAE file"),
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
}
