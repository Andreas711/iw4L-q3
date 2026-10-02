use crate::spec::ProjectileSpec;

// Reference: id Quake III Arena code/game/g_missile.c fire_grenade.
// Q3 adds +0.2 to the firing direction's Z component then renormalises.
pub const GRENADE_VERTICAL_BIAS: f32 = 0.2;
pub const GRENADE: ProjectileSpec = ProjectileSpec {
    direct_damage: 100,
    splash_damage: 100,
    splash_radius: 150.0,
    speed: 700.0,
    lifetime_ms: 2_500,
    refire_ms: 800,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_grenade_values() {
        assert_eq!(GRENADE.speed, 700.0);
        assert_eq!(GRENADE.lifetime_ms, 2500);
        assert_eq!(GRENADE_VERTICAL_BIAS, 0.2);
    }
}
