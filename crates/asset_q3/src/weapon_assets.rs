use std::collections::HashMap;
use std::path::{Path, PathBuf};

use weapon_q3::Quake3Weapon;

use crate::{Md3Error, Md3Model, Pk3Archive, Pk3Error, parse_md3};

#[derive(Clone, Copy, Debug)]
pub struct WeaponAssetSpec {
    pub weapon: Quake3Weapon,
    pub model: &'static str,
    pub icon: &'static str,
}

pub const fn weapon_asset_spec(weapon: Quake3Weapon) -> WeaponAssetSpec {
    match weapon {
        Quake3Weapon::Gauntlet => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/gauntlet/gauntlet.md3",
            icon: "icons/iconw_gauntlet",
        },
        Quake3Weapon::Machinegun => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/machinegun/machinegun.md3",
            icon: "icons/iconw_machinegun",
        },
        Quake3Weapon::Shotgun => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/shotgun/shotgun.md3",
            icon: "icons/iconw_shotgun",
        },
        Quake3Weapon::GrenadeLauncher => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/grenadel/grenadel.md3",
            icon: "icons/iconw_grenade",
        },
        Quake3Weapon::RocketLauncher => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/rocketl/rocketl.md3",
            icon: "icons/iconw_rocket",
        },
        Quake3Weapon::LightningGun => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/lightning/lightning.md3",
            icon: "icons/iconw_lightning",
        },
        Quake3Weapon::Railgun => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/railgun/railgun.md3",
            icon: "icons/iconw_railgun",
        },
        Quake3Weapon::PlasmaGun => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/plasma/plasma.md3",
            icon: "icons/iconw_plasma",
        },
        Quake3Weapon::Bfg => WeaponAssetSpec {
            weapon,
            model: "models/weapons2/bfg/bfg.md3",
            icon: "icons/iconw_bfg",
        },
    }
}

#[derive(Clone, Debug)]
pub struct Q3Texture {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Q3WeaponModel {
    pub weapon: Quake3Weapon,
    pub path: String,
    pub model: Md3Model,
    pub hand: Option<Md3Model>,
    pub barrel: Option<Md3Model>,
    pub flash: Option<Md3Model>,
    /// One decoded colour texture per weapon-model surface when the MD3 shader
    /// resolves directly to an image in pak0.pk3.
    pub surface_textures: Vec<Option<Q3Texture>>,
}

#[derive(Clone, Debug, Default)]
pub struct Q3WeaponAssetSet {
    pub source: PathBuf,
    pub models: Vec<Q3WeaponModel>,
    pub missing: Vec<String>,
}

impl Q3WeaponAssetSet {
    pub fn model(&self, weapon: Quake3Weapon) -> Option<&Q3WeaponModel> {
        self.models.iter().find(|entry| entry.weapon == weapon)
    }

    pub fn loaded_count(&self) -> usize {
        self.models.len()
    }

    pub fn texture_count(&self) -> usize {
        self.models
            .iter()
            .flat_map(|model| model.surface_textures.iter())
            .filter(|texture| texture.is_some())
            .count()
    }

    pub fn surface_count(&self) -> usize {
        self.models.iter().map(|model| model.model.surfaces.len()).sum()
    }
}

#[derive(Debug)]
pub enum WeaponAssetError {
    Pk3(Pk3Error),
    Md3 { path: String, error: Md3Error },
    MissingPak0(PathBuf),
}

impl core::fmt::Display for WeaponAssetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Pk3(e) => write!(f, "{e}"),
            Self::Md3 { path, error } => write!(f, "{path}: {error}"),
            Self::MissingPak0(path) => write!(f, "pak0.pk3 not found under {}", path.display()),
        }
    }
}

impl std::error::Error for WeaponAssetError {}

impl From<Pk3Error> for WeaponAssetError {
    fn from(value: Pk3Error) -> Self {
        Self::Pk3(value)
    }
}

fn strip_shader_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut block = false;
    while i < bytes.len() {
        if block {
            if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            block = true;
            i += 2;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push('\n');
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn shader_tokens(input: &str) -> Vec<String> {
    let clean = strip_shader_comments(input);
    let mut spaced = String::with_capacity(clean.len() + 64);
    for ch in clean.chars() {
        match ch {
            '{' | '}' => {
                spaced.push(' ');
                spaced.push(ch);
                spaced.push(' ');
            }
            _ => spaced.push(ch),
        }
    }
    spaced.split_whitespace().map(str::to_owned).collect()
}

fn load_shader_maps(pk3: &mut Pk3Archive) -> HashMap<String, String> {
    let shader_names: Vec<String> = pk3
        .names()
        .iter()
        .filter(|name| name.starts_with("scripts/") && name.ends_with(".shader"))
        .cloned()
        .collect();
    let mut maps = HashMap::new();

    for name in shader_names {
        let Ok(bytes) = pk3.read(&name) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let tokens = shader_tokens(&text);
        let mut i = 0usize;
        while i + 1 < tokens.len() {
            let shader_name = tokens[i].replace('\\', "/").to_ascii_lowercase();
            if tokens[i + 1] != "{" {
                i += 1;
                continue;
            }
            i += 2;
            let mut depth = 1i32;
            let mut chosen: Option<String> = None;
            while i < tokens.len() && depth > 0 {
                match tokens[i].as_str() {
                    "{" => {
                        depth += 1;
                        i += 1;
                    }
                    "}" => {
                        depth -= 1;
                        i += 1;
                    }
                    "map" | "clampmap" if depth >= 2 && i + 1 < tokens.len() => {
                        let candidate = tokens[i + 1].replace('\\', "/");
                        if chosen.is_none()
                            && !candidate.starts_with("$")
                            && !candidate.starts_with("*")
                        {
                            chosen = Some(candidate);
                        }
                        i += 2;
                    }
                    "animmap" if depth >= 2 && i + 2 < tokens.len() => {
                        // animMap <frequency> <image1> <image2> ...
                        let candidate = tokens[i + 2].replace('\\', "/");
                        if chosen.is_none()
                            && !candidate.starts_with("$")
                            && !candidate.starts_with("*")
                        {
                            chosen = Some(candidate);
                        }
                        i += 3;
                    }
                    _ => i += 1,
                }
            }
            if let Some(image) = chosen {
                maps.entry(shader_name).or_insert(image);
            }
        }
    }
    maps
}

fn image_format(path: &str) -> Option<image::ImageFormat> {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "tga" => Some(image::ImageFormat::Tga),
        "jpg" | "jpeg" => Some(image::ImageFormat::Jpeg),
        "png" => Some(image::ImageFormat::Png),
        _ => None,
    }
}
fn read_q3_texture_resolved(
    pk3: &mut Pk3Archive,
    shader_maps: &HashMap<String, String>,
    shader: &str,
) -> Option<Q3Texture> {
    read_q3_texture(pk3, shader).or_else(|| {
        let key = shader.replace('\\', "/").to_ascii_lowercase();
        shader_maps
            .get(&key)
            .and_then(|image| read_q3_texture(pk3, image))
    })
}

fn q3_texture_candidates(shader: &str) -> Vec<String> {
    let normalised = shader.replace('\\', "/");
    let path = Path::new(&normalised);
    let mut out = Vec::with_capacity(5);

    // Quake III's renderer treats image extensions as hints. A shader commonly
    // says ".tga" even when the PK3 only contains the equivalent ".jpg".
    // Preserve the authored spelling first, then search the supported sibling
    // extensions by stem.
    if path.extension().is_some() {
        out.push(normalised.clone());
        let stem = path.with_extension("").to_string_lossy().replace('\\', "/");
        for ext in ["tga", "jpg", "jpeg", "png"] {
            let candidate = format!("{stem}.{ext}");
            if !out.iter().any(|existing| existing.eq_ignore_ascii_case(&candidate)) {
                out.push(candidate);
            }
        }
    } else {
        for ext in ["tga", "jpg", "jpeg", "png"] {
            out.push(format!("{normalised}.{ext}"));
        }
    }
    out
}

fn read_q3_texture(pk3: &mut Pk3Archive, shader: &str) -> Option<Q3Texture> {
    for path in q3_texture_candidates(shader) {
        let Some(format) = image_format(&path) else {
            continue;
        };
        let bytes = match pk3.read(&path) {
            Ok(bytes) => bytes,
            Err(Pk3Error::Missing(_)) => continue,
            Err(_) => return None,
        };

        // Do not let one bad candidate abort extension fallback. Q3 data often
        // uses a canonical .tga reference while the actual asset is JPEG.
        let Ok(decoded) = image::load_from_memory_with_format(&bytes, format) else {
            continue;
        };
        let rgba = decoded.to_rgba8();
        let (width, height) = rgba.dimensions();
        return Some(Q3Texture {
            path,
            width,
            height,
            rgba: rgba.into_raw(),
        });
    }
    None
}

fn pak0_path(baseq3: &Path) -> Result<PathBuf, WeaponAssetError> {
    if baseq3.is_file() {
        return Ok(baseq3.to_path_buf());
    }
    let path = baseq3.join("pak0.pk3");
    path.is_file()
        .then_some(path)
        .ok_or_else(|| WeaponAssetError::MissingPak0(baseq3.to_path_buf()))
}

pub fn load_weapon_models(baseq3: impl AsRef<Path>) -> Result<Q3WeaponAssetSet, WeaponAssetError> {
    let pak0 = pak0_path(baseq3.as_ref())?;
    let mut pk3 = Pk3Archive::open(&pak0)?;
    let shader_maps = load_shader_maps(&mut pk3);
    let mut set = Q3WeaponAssetSet {
        source: pak0,
        ..Default::default()
    };
    for id in 1u8..=9 {
        let Some(weapon) = Quake3Weapon::from_id(id) else {
            continue;
        };
        let spec = weapon_asset_spec(weapon);
        match pk3.read(spec.model) {
            Ok(bytes) => {
                let model = parse_md3(&bytes).map_err(|error| WeaponAssetError::Md3 {
                    path: spec.model.to_owned(),
                    error,
                })?;
                let stem = spec.model.strip_suffix(".md3").unwrap_or(spec.model);
                let read_companion = |pk3: &mut Pk3Archive, suffix: &str| -> Option<Md3Model> {
                    let path = format!("{stem}_{suffix}.md3");
                    pk3.read(&path).ok().and_then(|bytes| parse_md3(&bytes).ok())
                };
                let mut hand = read_companion(&mut pk3, "hand");
                if hand.is_none() {
                    hand = pk3
                        .read("models/weapons2/shotgun/shotgun_hand.md3")
                        .ok()
                        .and_then(|bytes| parse_md3(&bytes).ok());
                }
                let barrel = read_companion(&mut pk3, "barrel");
                let flash = read_companion(&mut pk3, "flash");
                let surface_textures = model
                    .surfaces
                    .iter()
                    .map(|surface| {
                        surface
                            .shaders
                            .first()
                            .and_then(|shader| read_q3_texture_resolved(&mut pk3, &shader_maps, shader))
                    })
                    .collect();
                set.models.push(Q3WeaponModel {
                    weapon,
                    path: spec.model.to_owned(),
                    model,
                    hand,
                    barrel,
                    flash,
                    surface_textures,
                });
            }
            Err(Pk3Error::Missing(_)) => set.missing.push(spec.model.to_owned()),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(set)
}

pub fn q3_baseq3_from_env() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("IW4L_Q3_BASEQ3").map(PathBuf::from) {
        return Some(explicit);
    }

    let games = std::env::var_os("IW4L_GAMES").map(PathBuf::from)?;
    let candidates = [
        games.join("Quake III Arena").join("baseq3"),
        games.join("Quake 3 Arena").join("baseq3"),
        games.join("quake3").join("baseq3"),
        games.join("q3").join("baseq3"),
        games.join("baseq3"),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("pak0.pk3").is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_weapon_paths_are_md3() {
        for id in 1u8..=9 {
            let weapon = Quake3Weapon::from_id(id).unwrap();
            assert!(weapon_asset_spec(weapon).model.ends_with(".md3"));
        }
    }

    #[test]
    fn q3_texture_extension_fallback_matches_renderer_convention() {
        assert_eq!(
            q3_texture_candidates("models/weapons2/shotgun/shotgun.tga"),
            vec![
                "models/weapons2/shotgun/shotgun.tga",
                "models/weapons2/shotgun/shotgun.jpg",
                "models/weapons2/shotgun/shotgun.jpeg",
                "models/weapons2/shotgun/shotgun.png",
            ]
        );
    }

    #[test]
    fn q3_texture_without_extension_checks_all_supported_formats() {
        assert_eq!(
            q3_texture_candidates("models/weapons2/rocketl/rocketl"),
            vec![
                "models/weapons2/rocketl/rocketl.tga",
                "models/weapons2/rocketl/rocketl.jpg",
                "models/weapons2/rocketl/rocketl.jpeg",
                "models/weapons2/rocketl/rocketl.png",
            ]
        );
    }
}
