use bevy::input::gamepad::{Gamepad, GamepadButton, GamepadConnection, GamepadConnectionEvent};
use bevy::prelude::*;
use frame::{UiMenuKey, UiMenuRequest};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Sticks {
    /// Forward and right.
    pub movement: Vec2,
    /// Right and up.
    pub look: Vec2,
}

fn radial(stick: Vec2, deadzone: f32) -> Vec2 {
    let length = stick.length();
    if length <= deadzone || length <= f32::EPSILON {
        return Vec2::ZERO;
    }
    let scaled = ((length - deadzone) / (1.0 - deadzone).max(0.01)).min(1.0);
    stick / length * scaled
}

pub(crate) fn sticks(pad: &Gamepad, settings: &frame::GameSettings) -> Sticks {
    let left = radial(pad.left_stick(), settings.pad_deadzone_left);
    let right = radial(pad.right_stick(), settings.pad_deadzone_right);
    match settings.pad_stick_layout {
        1 => Sticks {
            movement: Vec2::new(right.y, right.x),
            look: left,
        },
        2 => Sticks {
            movement: Vec2::new(left.y, right.x),
            look: Vec2::new(left.x, right.y),
        },
        3 => Sticks {
            movement: Vec2::new(right.y, left.x),
            look: Vec2::new(right.x, left.y),
        },
        _ => Sticks {
            movement: Vec2::new(left.y, left.x),
            look: right,
        },
    }
}

fn curve(deflection: f32, kind: u8) -> f32 {
    let d = deflection.clamp(0.0, 1.0);
    match kind {
        1 => d,
        2 => 1.0 - (1.0 - d) * (1.0 - d),
        _ => 0.35 * d + 0.65 * d * d * d,
    }
}

pub(crate) fn shaped_look(look: Vec2, settings: &frame::GameSettings) -> Vec2 {
    let deflection = look.length();
    if deflection <= f32::EPSILON {
        return Vec2::ZERO;
    }
    let shaped = look / deflection * curve(deflection, settings.pad_curve);
    Vec2::new(
        shaped.x,
        if settings.pad_invert {
            -shaped.y
        } else {
            shaped.y
        },
    )
}

pub(crate) fn track_active_pad(
    gamepads: Query<(Entity, &Gamepad, Option<&Name>)>,
    mut connections: MessageReader<GamepadConnectionEvent>,
    mut active: ResMut<frame::ActivePad>,
) {
    for event in connections.read() {
        match &event.connection {
            GamepadConnection::Connected { name, .. } => {
                diag::info!(Ui, "controller connected: {name}");
            }
            GamepadConnection::Disconnected => {
                diag::info!(Ui, "controller disconnected");
            }
        }
    }
    if active.0.is_some_and(|entity| gamepads.get(entity).is_err()) {
        active.0 = None;
    }
    for (entity, pad, name) in &gamepads {
        let moved = pad.left_stick().length() > 0.5 || pad.right_stick().length() > 0.5;
        if (pad.get_just_pressed().next().is_some() || moved) && active.0 != Some(entity) {
            diag::info!(
                Ui,
                "controller in use: {}",
                name.map_or("unnamed", |n| n.as_str())
            );
            active.0 = Some(entity);
        }
    }
}

const REPEAT_DELAY: f32 = 0.4;
const REPEAT_EVERY: f32 = 0.12;

pub(crate) fn drive_menus_with_pad(
    gamepads: Query<&Gamepad>,
    active: Res<frame::ActivePad>,
    script_menus: Option<Res<hud::ScriptMenus>>,
    capture: Res<frame::UiBindingCapture>,
    time: Res<Time>,
    mut requests: MessageWriter<UiMenuRequest>,
    mut repeat: Local<Option<(UiMenuKey, f32)>>,
) {
    // A binding being listened for takes the controller's buttons itself.
    let pad = active
        .0
        .and_then(|entity| gamepads.get(entity).ok())
        .filter(|_| capture.command.is_none());
    let Some(pad) = pad else {
        *repeat = None;
        return;
    };
    if pad.just_pressed(GamepadButton::Start) {
        requests.write(UiMenuRequest::Key(UiMenuKey::Escape));
    }
    if !script_menus.is_some_and(|menus| menus.captures_input()) {
        *repeat = None;
        return;
    }
    if pad.just_pressed(GamepadButton::South) {
        requests.write(UiMenuRequest::Key(UiMenuKey::Enter));
    }
    if pad.just_pressed(GamepadButton::East) {
        requests.write(UiMenuRequest::Key(UiMenuKey::Escape));
    }
    let stick = pad.left_stick();
    let direction = if pad.pressed(GamepadButton::DPadUp) || stick.y > 0.6 {
        Some(UiMenuKey::Up)
    } else if pad.pressed(GamepadButton::DPadDown) || stick.y < -0.6 {
        Some(UiMenuKey::Down)
    } else if pad.pressed(GamepadButton::DPadLeft) || stick.x < -0.6 {
        Some(UiMenuKey::Left)
    } else if pad.pressed(GamepadButton::DPadRight) || stick.x > 0.6 {
        Some(UiMenuKey::Right)
    } else {
        None
    };
    let now = time.elapsed_secs();
    let fire = match (direction, *repeat) {
        (Some(key), Some((held, next))) if key == held => {
            if now >= next {
                *repeat = Some((key, now + REPEAT_EVERY));
            }
            now >= next
        }
        (Some(key), _) => {
            *repeat = Some((key, now + REPEAT_DELAY));
            true
        }
        (None, _) => {
            *repeat = None;
            false
        }
    };
    if fire && let Some(key) = direction {
        requests.write(UiMenuRequest::Key(key));
    }
}
