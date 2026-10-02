use crate::spec::ProjectileSpec;

// Reference: id Quake III Arena code/game/g_missile.c fire_rocket.
pub const ROCKET: ProjectileSpec = ProjectileSpec {
    direct_damage: 100,
    splash_damage: 100,
    splash_radius: 120.0,
    speed: 900.0,
    lifetime_ms: 15_000,
    refire_ms: 800,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_rocket_values() {
        assert_eq!(ROCKET.direct_damage, 100);
        assert_eq!(ROCKET.splash_radius, 120.0);
        assert_eq!(ROCKET.speed, 900.0);
    }
}
