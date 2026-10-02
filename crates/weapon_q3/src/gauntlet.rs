use crate::spec::MeleeSpec;

// Reference: id Quake III Arena code/game/g_weapon.c CheckGauntletAttack.
pub const GAUNTLET: MeleeSpec = MeleeSpec {
    damage: 50,
    range: 32.0,
    refire_ms: 400,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_gauntlet_values() {
        assert_eq!(GAUNTLET.damage, 50);
        assert_eq!(GAUNTLET.range, 32.0);
    }
}
