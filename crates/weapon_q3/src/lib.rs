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
}
