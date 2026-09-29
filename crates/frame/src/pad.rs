use bevy::prelude::{Entity, Resource};

/// The last controller pressed or moved, so a virtual joystick or an idle
/// second pad can stay connected without taking over.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivePad(pub Option<Entity>);
