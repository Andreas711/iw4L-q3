use crate::spec::HitscanSpec;

// References: bg_public.h DEFAULT_SHOTGUN_* and g_weapon.c ShotgunPattern.
pub const SHOTGUN_DAMAGE_PER_PELLET: i32 = 10;
pub const SHOTGUN_PELLETS: u16 = 11;
pub const SHOTGUN_SPREAD: f32 = 700.0;
pub const SHOTGUN_RANGE: f32 = 8192.0 * 16.0;
pub const SHOTGUN_REFIRE_MS: i32 = 1000;

pub const SHOTGUN: HitscanSpec = HitscanSpec {
    damage: SHOTGUN_DAMAGE_PER_PELLET,
    range: SHOTGUN_RANGE,
    refire_ms: SHOTGUN_REFIRE_MS,
    pellets: SHOTGUN_PELLETS,
    spread: SHOTGUN_SPREAD,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_shotgun_values() {
        assert_eq!(SHOTGUN.pellets, 11);
        assert_eq!(SHOTGUN.damage, 10);
        assert_eq!(SHOTGUN.refire_ms, 1000);
    }
}
