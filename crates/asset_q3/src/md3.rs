use core::fmt;

const MD3_IDENT: &[u8; 4] = b"IDP3";
const MD3_VERSION: i32 = 15;
const MD3_HEADER: usize = 108;
const MD3_FRAME: usize = 56;
const MD3_TAG: usize = 112;
const MD3_SURFACE_HEADER: usize = 108;
const MD3_SHADER: usize = 68;
const MD3_TRIANGLE: usize = 12;
const MD3_ST: usize = 8;
const MD3_XYZNORMAL: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct Md3Frame {
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub local_origin: [f32; 3],
    pub radius: f32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Md3Tag {
    pub name: String,
    pub origin: [f32; 3],
    pub axis: [[f32; 3]; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Md3Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Md3Surface {
    pub name: String,
    pub shaders: Vec<String>,
    pub triangles: Vec<[u32; 3]>,
    pub texcoords: Vec<[f32; 2]>,
    /// One vertex array per MD3 animation frame.
    pub frames: Vec<Vec<Md3Vertex>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Md3Model {
    pub name: String,
    pub frames: Vec<Md3Frame>,
    /// Tags are stored frame-major: tags[frame][tag].
    pub tags: Vec<Vec<Md3Tag>>,
    pub surfaces: Vec<Md3Surface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Md3Error {
    Truncated,
    BadIdent,
    BadVersion(i32),
    BadOffset,
    BadCount,
    BadSurfaceIdent,
    IndexOutOfRange,
}

impl fmt::Display for Md3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => write!(f, "truncated MD3"),
            Self::BadIdent => write!(f, "not an MD3 file"),
            Self::BadVersion(v) => write!(f, "unsupported MD3 version {v}"),
            Self::BadOffset => write!(f, "invalid MD3 offset"),
            Self::BadCount => write!(f, "invalid MD3 count"),
            Self::BadSurfaceIdent => write!(f, "invalid MD3 surface header"),
            Self::IndexOutOfRange => write!(f, "MD3 triangle index out of range"),
        }
    }
}

impl std::error::Error for Md3Error {}

fn bytes<const N: usize>(data: &[u8], off: usize) -> Result<[u8; N], Md3Error> {
    data.get(off..off + N)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(Md3Error::Truncated)
}

fn i32le(data: &[u8], off: usize) -> Result<i32, Md3Error> {
    Ok(i32::from_le_bytes(bytes(data, off)?))
}

fn i16le(data: &[u8], off: usize) -> Result<i16, Md3Error> {
    Ok(i16::from_le_bytes(bytes(data, off)?))
}

fn u16le(data: &[u8], off: usize) -> Result<u16, Md3Error> {
    Ok(u16::from_le_bytes(bytes(data, off)?))
}

fn f32le(data: &[u8], off: usize) -> Result<f32, Md3Error> {
    Ok(f32::from_le_bytes(bytes(data, off)?))
}

fn fixed_cstr(data: &[u8], off: usize, len: usize) -> Result<String, Md3Error> {
    let raw = data.get(off..off + len).ok_or(Md3Error::Truncated)?;
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    Ok(String::from_utf8_lossy(&raw[..end]).into_owned())
}

fn count(v: i32) -> Result<usize, Md3Error> {
    let n = usize::try_from(v).map_err(|_| Md3Error::BadCount)?;
    (n <= 1_000_000).then_some(n).ok_or(Md3Error::BadCount)
}

fn offset(v: i32) -> Result<usize, Md3Error> {
    usize::try_from(v).map_err(|_| Md3Error::BadOffset)
}

fn checked(base: usize, rel: usize) -> Result<usize, Md3Error> {
    base.checked_add(rel).ok_or(Md3Error::BadOffset)
}

fn span(data: &[u8], off: usize, count: usize, stride: usize) -> Result<(), Md3Error> {
    let bytes = count.checked_mul(stride).ok_or(Md3Error::BadOffset)?;
    let end = off.checked_add(bytes).ok_or(Md3Error::BadOffset)?;
    if end <= data.len() {
        Ok(())
    } else {
        Err(Md3Error::Truncated)
    }
}

fn decode_normal(encoded: u16) -> [f32; 3] {
    let lat = f32::from((encoded >> 8) as u8) * core::f32::consts::TAU / 255.0;
    let lng = f32::from((encoded & 0xff) as u8) * core::f32::consts::TAU / 255.0;
    [lat.cos() * lng.sin(), lat.sin() * lng.sin(), lng.cos()]
}

pub fn parse_md3(data: &[u8]) -> Result<Md3Model, Md3Error> {
    if data.len() < MD3_HEADER {
        return Err(Md3Error::Truncated);
    }
    if data.get(0..4) != Some(MD3_IDENT.as_slice()) {
        return Err(Md3Error::BadIdent);
    }
    let version = i32le(data, 4)?;
    if version != MD3_VERSION {
        return Err(Md3Error::BadVersion(version));
    }

    let name = fixed_cstr(data, 8, 64)?;
    let num_frames = count(i32le(data, 76)?)?;
    let num_tags = count(i32le(data, 80)?)?;
    let num_surfaces = count(i32le(data, 84)?)?;
    let ofs_frames = offset(i32le(data, 92)?)?;
    let ofs_tags = offset(i32le(data, 96)?)?;
    let ofs_surfaces = offset(i32le(data, 100)?)?;
    let ofs_eof = offset(i32le(data, 104)?)?;
    if ofs_eof > data.len() || ofs_frames < MD3_HEADER || ofs_surfaces < MD3_HEADER {
        return Err(Md3Error::BadOffset);
    }

    span(data, ofs_frames, num_frames, MD3_FRAME)?;
    let mut frames = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let o = ofs_frames + i * MD3_FRAME;
        frames.push(Md3Frame {
            mins: [f32le(data, o)?, f32le(data, o + 4)?, f32le(data, o + 8)?],
            maxs: [
                f32le(data, o + 12)?,
                f32le(data, o + 16)?,
                f32le(data, o + 20)?,
            ],
            local_origin: [
                f32le(data, o + 24)?,
                f32le(data, o + 28)?,
                f32le(data, o + 32)?,
            ],
            radius: f32le(data, o + 36)?,
            name: fixed_cstr(data, o + 40, 16)?,
        });
    }

    let tag_rows = num_frames.checked_mul(num_tags).ok_or(Md3Error::BadCount)?;
    span(data, ofs_tags, tag_rows, MD3_TAG)?;
    let mut tags = Vec::with_capacity(num_frames);
    for frame in 0..num_frames {
        let mut row = Vec::with_capacity(num_tags);
        for tag in 0..num_tags {
            let o = ofs_tags + (frame * num_tags + tag) * MD3_TAG;
            row.push(Md3Tag {
                name: fixed_cstr(data, o, 64)?,
                origin: [
                    f32le(data, o + 64)?,
                    f32le(data, o + 68)?,
                    f32le(data, o + 72)?,
                ],
                axis: [
                    [f32le(data, o + 76)?, f32le(data, o + 80)?, f32le(data, o + 84)?],
                    [f32le(data, o + 88)?, f32le(data, o + 92)?, f32le(data, o + 96)?],
                    [f32le(data, o + 100)?, f32le(data, o + 104)?, f32le(data, o + 108)?],
                ],
            });
        }
        tags.push(row);
    }

    let mut surfaces = Vec::with_capacity(num_surfaces);
    let mut surface_off = ofs_surfaces;
    for _ in 0..num_surfaces {
        if data.get(surface_off..surface_off + 4) != Some(MD3_IDENT.as_slice()) {
            return Err(Md3Error::BadSurfaceIdent);
        }
        let surface_name = fixed_cstr(data, surface_off + 4, 64)?;
        let s_frames = count(i32le(data, surface_off + 72)?)?;
        let s_shaders = count(i32le(data, surface_off + 76)?)?;
        let s_verts = count(i32le(data, surface_off + 80)?)?;
        let s_tris = count(i32le(data, surface_off + 84)?)?;
        let tri_off = checked(surface_off, offset(i32le(data, surface_off + 88)?)?)?;
        let shader_off = checked(surface_off, offset(i32le(data, surface_off + 92)?)?)?;
        let st_off = checked(surface_off, offset(i32le(data, surface_off + 96)?)?)?;
        let xyz_off = checked(surface_off, offset(i32le(data, surface_off + 100)?)?)?;
        let end_rel = offset(i32le(data, surface_off + 104)?)?;
        let surface_end = checked(surface_off, end_rel)?;
        if end_rel < MD3_SURFACE_HEADER || surface_end > data.len() || surface_end > ofs_eof {
            return Err(Md3Error::BadOffset);
        }

        span(data, tri_off, s_tris, MD3_TRIANGLE)?;
        span(data, shader_off, s_shaders, MD3_SHADER)?;
        span(data, st_off, s_verts, MD3_ST)?;
        span(
            data,
            xyz_off,
            s_frames.checked_mul(s_verts).ok_or(Md3Error::BadCount)?,
            MD3_XYZNORMAL,
        )?;

        let mut triangles = Vec::with_capacity(s_tris);
        for i in 0..s_tris {
            let o = tri_off + i * MD3_TRIANGLE;
            let tri = [
                u32::try_from(i32le(data, o)?).map_err(|_| Md3Error::IndexOutOfRange)?,
                u32::try_from(i32le(data, o + 4)?).map_err(|_| Md3Error::IndexOutOfRange)?,
                u32::try_from(i32le(data, o + 8)?).map_err(|_| Md3Error::IndexOutOfRange)?,
            ];
            if tri.iter().any(|&v| v as usize >= s_verts) {
                return Err(Md3Error::IndexOutOfRange);
            }
            triangles.push(tri);
        }

        let mut shaders = Vec::with_capacity(s_shaders);
        for i in 0..s_shaders {
            shaders.push(fixed_cstr(data, shader_off + i * MD3_SHADER, 64)?);
        }

        let mut texcoords = Vec::with_capacity(s_verts);
        for i in 0..s_verts {
            let o = st_off + i * MD3_ST;
            texcoords.push([f32le(data, o)?, f32le(data, o + 4)?]);
        }

        let mut surface_frames = Vec::with_capacity(s_frames);
        for frame in 0..s_frames {
            let mut verts = Vec::with_capacity(s_verts);
            for vertex in 0..s_verts {
                let o = xyz_off + (frame * s_verts + vertex) * MD3_XYZNORMAL;
                verts.push(Md3Vertex {
                    position: [
                        f32::from(i16le(data, o)?) / 64.0,
                        f32::from(i16le(data, o + 2)?) / 64.0,
                        f32::from(i16le(data, o + 4)?) / 64.0,
                    ],
                    normal: decode_normal(u16le(data, o + 6)?),
                });
            }
            surface_frames.push(verts);
        }

        surfaces.push(Md3Surface {
            name: surface_name,
            shaders,
            triangles,
            texcoords,
            frames: surface_frames,
        });
        surface_off = surface_end;
    }

    Ok(Md3Model {
        name,
        frames,
        tags,
        surfaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_i32(buf: &mut [u8], off: usize, value: i32) {
        buf[off..off + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn rejects_wrong_ident() {
        let data = vec![0u8; MD3_HEADER];
        assert_eq!(parse_md3(&data), Err(Md3Error::BadIdent));
    }

    #[test]
    fn parses_empty_valid_md3() {
        let mut data = vec![0u8; MD3_HEADER];
        data[..4].copy_from_slice(MD3_IDENT);
        put_i32(&mut data, 4, MD3_VERSION);
        put_i32(&mut data, 92, MD3_HEADER as i32);
        put_i32(&mut data, 96, MD3_HEADER as i32);
        put_i32(&mut data, 100, MD3_HEADER as i32);
        put_i32(&mut data, 104, MD3_HEADER as i32);
        let model = parse_md3(&data).expect("valid empty md3");
        assert!(model.frames.is_empty());
        assert!(model.surfaces.is_empty());
    }
}
