use crate::spec::ProjectileSpec;

// Reference: id Quake III Arena code/game/g_missile.c fire_plasma.
pub const PLASMA: ProjectileSpec = ProjectileSpec {
    direct_damage: 20,
    splash_damage: 15,
    splash_radius: 20.0,
    speed: 2000.0,
    lifetime_ms: 10_000,
    refire_ms: 100,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_plasma_values() {
        assert_eq!(PLASMA.direct_damage, 20);
        assert_eq!(PLASMA.splash_damage, 15);
        assert_eq!(PLASMA.speed, 2000.0);
    }
}
