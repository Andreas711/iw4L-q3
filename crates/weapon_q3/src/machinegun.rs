use crate::spec::HitscanSpec;

// Quake III Arena gameplay values, independently represented for IW4L.
// Reference: id-Software/Quake-III-Arena code/game/g_weapon.c
pub const MACHINEGUN_DAMAGE: i32 = 7;
pub const MACHINEGUN_TEAM_DAMAGE: i32 = 5;
pub const MACHINEGUN_SPREAD: f32 = 200.0;
pub const MACHINEGUN_RANGE: f32 = 8192.0 * 16.0;
pub const MACHINEGUN_REFIRE_MS: i32 = 100;

pub const MACHINEGUN: HitscanSpec = HitscanSpec {
    damage: MACHINEGUN_DAMAGE,
    range: MACHINEGUN_RANGE,
    refire_ms: MACHINEGUN_REFIRE_MS,
    pellets: 1,
    spread: MACHINEGUN_SPREAD,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_machinegun_values() {
        assert_eq!(MACHINEGUN.damage, 7);
        assert_eq!(MACHINEGUN.refire_ms, 100);
        assert_eq!(MACHINEGUN.pellets, 1);
    }
}
