use crate::spec::ProjectileSpec;

// Reference: id Quake III Arena code/game/g_missile.c fire_bfg.
pub const BFG: ProjectileSpec = ProjectileSpec {
    direct_damage: 100,
    splash_damage: 100,
    splash_radius: 120.0,
    speed: 2000.0,
    lifetime_ms: 10_000,
    refire_ms: 200,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_bfg_values() {
        assert_eq!(BFG.direct_damage, 100);
        assert_eq!(BFG.splash_radius, 120.0);
        assert_eq!(BFG.speed, 2000.0);
    }
}
