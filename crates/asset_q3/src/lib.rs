#![forbid(unsafe_code)]

mod md3;
mod pk3;
mod weapon_assets;

pub use md3::{Md3Error, Md3Frame, Md3Model, Md3Surface, Md3Tag, Md3Vertex, parse_md3};
pub use pk3::{Pk3Archive, Pk3Error};
pub use weapon_assets::{
    Q3WeaponAssetSet, Q3WeaponModel, WeaponAssetSpec, load_weapon_models, q3_baseq3_from_env,
    weapon_asset_spec,
};
