use bevy::prelude::*;

use crate::match_load::register_match_load_systems;
use crate::teardown::register_match_teardown;

pub struct AssetPlugin;

fn load_q3_weapon_models(mut prepared: ResMut<crate::PreparedQ3WeaponModels>) {
    let Some(baseq3) = asset_q3::q3_baseq3_from_env() else {
        prepared.error = Some(
            "Quake III baseq3 not found; set IW4L_Q3_BASEQ3 to baseq3 or pak0.pk3".to_owned(),
        );
        return;
    };
    prepared.source = Some(baseq3.clone());
    match asset_q3::load_weapon_models(&baseq3) {
        Ok(assets) => {
            prepared.assets = std::sync::Arc::new(assets);
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
