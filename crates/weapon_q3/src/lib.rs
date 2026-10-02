#![no_std]
#![forbid(unsafe_code)]

// Quake III weapon rules only.
//
// IW4L's `sim` crate remains authoritative for input ownership, lag
// compensation, collision, damage application, snapshots and replay.

pub mod bfg;
pub mod gauntlet;
pub mod grenade;
pub mod lightning;
pub mod machinegun;
pub mod plasma;
pub mod railgun;
pub mod rocket;
pub mod shotgun;
pub mod spec;

pub const DEFAULT_KNOCKBACK: f32 = 1000.0;
pub const PLAYER_MASS: f32 = 200.0;
pub const MAX_KNOCKBACK_DAMAGE: i32 = 200;
pub const SELF_DAMAGE_SCALE: f32 = 0.5;

#[must_use]
pub fn knockback_velocity_delta(direction: [f32; 3], damage: i32) -> [f32; 3] {
    let len = (direction[0] * direction[0]
        + direction[1] * direction[1]
        + direction[2] * direction[2])
        .sqrt();
    if len <= f32::EPSILON || damage <= 0 {
        return [0.0; 3];
    }
    let force = DEFAULT_KNOCKBACK * damage.min(MAX_KNOCKBACK_DAMAGE) as f32 / PLAYER_MASS;
    direction.map(|v| v / len * force)
}

#[must_use]
pub fn self_damage(damage: i32) -> i32 {
    ((damage as f32) * SELF_DAMAGE_SCALE).max(1.0) as i32
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Quake3Weapon {
    Gauntlet = 1,
    Machinegun = 2,
    Shotgun = 3,
    GrenadeLauncher = 4,
    RocketLauncher = 5,
    LightningGun = 6,
    Railgun = 7,
    PlasmaGun = 8,
    Bfg = 9,
}

impl Quake3Weapon {
    #[must_use]
    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::Gauntlet),
            2 => Some(Self::Machinegun),
            3 => Some(Self::Shotgun),
            4 => Some(Self::GrenadeLauncher),
            5 => Some(Self::RocketLauncher),
            6 => Some(Self::LightningGun),
            7 => Some(Self::Railgun),
            8 => Some(Self::PlasmaGun),
            9 => Some(Self::Bfg),
            _ => None,
        }
    }

    #[must_use]
    pub const fn refire_ms(self) -> i32 {
        match self {
            Self::Gauntlet => gauntlet::GAUNTLET.refire_ms,
            Self::Machinegun => machinegun::MACHINEGUN.refire_ms,
            Self::Shotgun => shotgun::SHOTGUN.refire_ms,
            Self::GrenadeLauncher => grenade::GRENADE.refire_ms,
            Self::RocketLauncher => rocket::ROCKET.refire_ms,
            Self::LightningGun => lightning::LIGHTNING.refire_ms,
            Self::Railgun => railgun::RAILGUN.refire_ms,
            Self::PlasmaGun => plasma::PLASMA.refire_ms,
            Self::Bfg => bfg::BFG.refire_ms,
        }
    }

    #[must_use]
    pub const fn weapon_pickup_ammo(self) -> i32 {
        match self {
            Self::Gauntlet => -1,
            Self::Machinegun => 40,
            Self::Shotgun => 10,
            Self::GrenadeLauncher => 10,
            Self::RocketLauncher => 10,
            Self::LightningGun => 100,
            Self::Railgun => 10,
            Self::PlasmaGun => 50,
            Self::Bfg => 20,
        }
    }

    #[must_use]
    pub const fn ammo_pickup_amount(self) -> i32 {
        match self {
            Self::Gauntlet => 0,
            Self::Machinegun => 50,
            Self::Shotgun => 10,
            Self::GrenadeLauncher => 5,
            Self::RocketLauncher => 5,
            Self::LightningGun => 60,
            Self::Railgun => 10,
            Self::PlasmaGun => 30,
            Self::Bfg => 15,
        }
    }

    #[must_use]
    pub const fn uses_ammo(self) -> bool {
        !matches!(self, Self::Gauntlet)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gauntlet => "gauntlet",
            Self::Machinegun => "machinegun",
            Self::Shotgun => "shotgun",
            Self::GrenadeLauncher => "grenade",
            Self::RocketLauncher => "rocket",
            Self::LightningGun => "lightning",
            Self::Railgun => "railgun",
            Self::PlasmaGun => "plasma",
            Self::Bfg => "bfg",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FireGate {
    next_fire_time_ms: i32,
}

impl FireGate {
    #[must_use]
    pub const fn new() -> Self {
        Self { next_fire_time_ms: 0 }
    }

    #[must_use]
    pub const fn next_fire_time_ms(self) -> i32 {
        self.next_fire_time_ms
    }

    #[must_use]
    pub const fn can_fire(self, now_ms: i32) -> bool {
        now_ms >= self.next_fire_time_ms
    }

    pub fn arm(&mut self, now_ms: i32, cooldown_ms: i32) {
        self.next_fire_time_ms = now_ms.saturating_add(cooldown_ms.max(0));
    }
}

#[cfg(test)]
mod tests {
    use super::FireGate;

    #[test]
    fn fire_gate_blocks_until_deadline() {
        let mut gate = FireGate::new();
        assert!(gate.can_fire(1000));

        gate.arm(1000, 1500);

        assert!(!gate.can_fire(2499));
        assert!(gate.can_fire(2500));
    }

    #[test]
    fn q3_weapon_ids_round_trip() {
        for id in 1u8..=9 {
            let weapon = super::Quake3Weapon::from_id(id).expect("Q3 weapon id");
            assert_eq!(weapon as u8, id);
            assert!(weapon.refire_ms() > 0);
            assert!(!weapon.name().is_empty());
        }
        assert!(super::Quake3Weapon::from_id(0).is_none());
        assert!(super::Quake3Weapon::from_id(10).is_none());
    }

    #[test]
    fn q3_pickup_ammo_matches_base_game_rules() {
        use super::Quake3Weapon as W;

        assert_eq!(W::Gauntlet.weapon_pickup_ammo(), -1);
        assert_eq!(W::Machinegun.weapon_pickup_ammo(), 40);
        assert_eq!(W::RocketLauncher.weapon_pickup_ammo(), 10);
        assert_eq!(W::LightningGun.weapon_pickup_ammo(), 100);
        assert_eq!(W::Bfg.ammo_pickup_amount(), 15);
    }
}
