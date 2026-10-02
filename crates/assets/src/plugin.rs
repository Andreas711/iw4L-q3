use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::match_load::register_match_load_systems;
use crate::teardown::register_match_teardown;

pub struct AssetPlugin;

fn load_q3_weapon_models(
    mut prepared: ResMut<crate::PreparedQ3WeaponModels>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(baseq3) = asset_q3::q3_baseq3_from_env() else {
        prepared.error = Some(
            "Quake III baseq3 not found; set IW4L_Q3_BASEQ3 to baseq3 or pak0.pk3".to_owned(),
        );
        return;
    };
    prepared.source = Some(baseq3.clone());
    match asset_q3::load_weapon_models(&baseq3) {
        Ok(assets) => {
            let mut textures = Vec::new();
            for weapon in &assets.models {
                for (surface, texture) in weapon.surface_textures.iter().enumerate() {
                    let Some(texture) = texture else {
                        continue;
                    };
                    let image = Image::new(
                        Extent3d {
                            width: texture.width,
                            height: texture.height,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        texture.rgba.clone(),
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::default(),
                    );
                    textures.push(crate::prepared::Q3SurfaceTextureHandle {
                        weapon: weapon.weapon as u8,
                        surface,
                        image: images.add(image),
                    });
                }
            }
            prepared.assets = std::sync::Arc::new(assets);
            prepared.textures = textures;
            prepared.error = None;
        }
        Err(error) => {
            prepared.error = Some(error.to_string());
        }
    }
}

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::PreparedQ3WeaponModels>()
            .add_systems(Startup, load_q3_weapon_models);
        register_match_load_systems(app);
        register_match_teardown(app);
    }
}
