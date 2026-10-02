#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitscanSpec {
    pub damage: i32,
    pub range: f32,
    pub refire_ms: i32,
    pub pellets: u16,
    pub spread: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileSpec {
    pub direct_damage: i32,
    pub splash_damage: i32,
    pub splash_radius: f32,
    pub speed: f32,
    pub lifetime_ms: i32,
    pub refire_ms: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeleeSpec {
    pub damage: i32,
    pub range: f32,
    pub refire_ms: i32,
}
