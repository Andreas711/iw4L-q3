use crate::FireGate;

pub const RAILGUN_DAMAGE: i32 = 100;
pub const RAILGUN_RANGE: f32 = 8192.0;
pub const RAILGUN_REFIRE_MS: i32 = 1500;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RailgunState {
    fire_gate: FireGate,
}

impl RailgunState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fire_gate: FireGate::new(),
        }
    }

    #[must_use]
    pub const fn can_fire(self, now_ms: i32) -> bool {
        self.fire_gate.can_fire(now_ms)
    }

    #[must_use]
    pub const fn next_fire_time_ms(self) -> i32 {
        self.fire_gate.next_fire_time_ms()
    }

    pub fn commit_shot(&mut self, now_ms: i32) {
        self.fire_gate.arm(now_ms, RAILGUN_REFIRE_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn railgun_uses_expected_defaults() {
        assert_eq!(RAILGUN_DAMAGE, 100);
        assert_eq!(RAILGUN_RANGE, 8192.0);
        assert_eq!(RAILGUN_REFIRE_MS, 1500);
    }

    #[test]
    fn railgun_refire_is_enforced() {
        let mut state = RailgunState::new();
        assert!(state.can_fire(5000));

        state.commit_shot(5000);

        assert!(!state.can_fire(6499));
        assert!(state.can_fire(6500));
    }
}
