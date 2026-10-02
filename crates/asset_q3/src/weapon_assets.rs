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
pub struct Q3WeaponModel {
    pub weapon: Quake3Weapon,
    pub path: String,
    pub model: Md3Model,
    pub hand: Option<Md3Model>,
    pub barrel: Option<Md3Model>,
    pub flash: Option<Md3Model>,
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
                set.models.push(Q3WeaponModel {
                    weapon,
                    path: spec.model.to_owned(),
                    model,
                    hand,
                    barrel,
                    flash,
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
}
