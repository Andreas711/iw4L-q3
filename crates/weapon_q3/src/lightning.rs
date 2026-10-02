use crate::spec::HitscanSpec;

// Reference: bg_public.h LIGHTNING_RANGE and g_weapon.c Weapon_LightningFire.
pub const LIGHTNING_DAMAGE: i32 = 8;
pub const LIGHTNING_RANGE: f32 = 768.0;
pub const LIGHTNING_REFIRE_MS: i32 = 50;

pub const LIGHTNING: HitscanSpec = HitscanSpec {
    damage: LIGHTNING_DAMAGE,
    range: LIGHTNING_RANGE,
    refire_ms: LIGHTNING_REFIRE_MS,
    pellets: 1,
    spread: 0.0,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_lightning_values() {
        assert_eq!(LIGHTNING.damage, 8);
        assert_eq!(LIGHTNING.range, 768.0);
        assert_eq!(LIGHTNING.refire_ms, 50);
    }
}
