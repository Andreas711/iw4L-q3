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
