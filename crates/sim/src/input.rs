use playerstate_iw4::UserCmd;

use crate::identities::MatchPhase;
use crate::world::ClientId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClassId(pub u32);

pub type ActionRequestId = u32;

/// Debug-action range used by the live Quake III weapon integration.
/// Negative DebugDamage values were previously refused, so these do not overlap
/// with the normal `damage` debug action.
const DEBUG_DAMAGE_Q3_FIRE_BASE: i32 = i32::MIN + 0x5100;

#[must_use]
pub const fn q3_debug_fire_amount(weapon: u8) -> Option<i32> {
    match weapon {
        1..=9 => Some(DEBUG_DAMAGE_Q3_FIRE_BASE + weapon as i32),
        _ => None,
    }
}

#[must_use]
pub const fn q3_debug_fire_weapon(amount: i32) -> Option<u8> {
    let weapon = amount as i64 - DEBUG_DAMAGE_Q3_FIRE_BASE as i64;
    if weapon >= 1 && weapon <= 9 {
        Some(weapon as u8)
    } else {
        None
    }
}

/// Compatibility name for the first Railgun-only test command.
pub const DEBUG_DAMAGE_Q3_RAILGUN: i32 = DEBUG_DAMAGE_Q3_FIRE_BASE + 7;

const DEBUG_DAMAGE_Q3_SELECT_BASE: i32 = i32::MIN + 0x5200;

#[must_use]
pub const fn q3_debug_select_amount(weapon: u8) -> Option<i32> {
    if weapon <= 9 {
        Some(DEBUG_DAMAGE_Q3_SELECT_BASE + weapon as i32)
    } else {
        None
    }
}

#[must_use]
pub const fn q3_debug_select_weapon(amount: i32) -> Option<u8> {
    let weapon = amount as i64 - DEBUG_DAMAGE_Q3_SELECT_BASE as i64;
    if weapon >= 0 && weapon <= 9 {
        Some(weapon as u8)
    } else {
        None
    }
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClientAction {
    JoinMatch {
        request_id: ActionRequestId,
    },

    LeaveMatch {
        request_id: ActionRequestId,
    },

    SelectClass {
        request_id: ActionRequestId,
        class_id: ClassId,
        revision: u32,
        loadout: crate::PersonalClass,
    },

    GiveWeapon {
        request_id: ActionRequestId,
        weapon: u32,
    },

    ChangeWeaponConfiguration {
        request_id: ActionRequestId,
        from: u32,
        to: u32,
    },

    ForceDeath {
        request_id: ActionRequestId,
    },

    SpawnClient {
        request_id: ActionRequestId,
    },

    ForceSpawn {
        request_id: ActionRequestId,
        pick: SpawnPick,
    },

    SpawnIntermission {
        request_id: ActionRequestId,
    },

    SetMatchPhase {
        request_id: ActionRequestId,
        phase: MatchPhase,
    },

    Move {
        request_id: ActionRequestId,
        origin: [f32; 3],
        angles: [f32; 3],
    },

    BeginScriptMoverRotateVelocity {
        request_id: ActionRequestId,
        speed: f32,
    },

    DebugDamage {
        request_id: ActionRequestId,
        amount: i32,
    },

    SetName {
        request_id: ActionRequestId,
        name: [u8; 16],
    },

    UseCopycat {
        request_id: ActionRequestId,
    },

    ActionSlot {
        request_id: ActionRequestId,
        slot: u8,
    },

    ChooseDefaultClass {
        request_id: ActionRequestId,
        index: u8,
    },

    MenuResponse {
        request_id: ActionRequestId,
        menu: [u8; MENU_RESPONSE_BYTES],
        response: [u8; MENU_RESPONSE_BYTES],
    },

    ResupplyAmmo {
        request_id: ActionRequestId,
    },

    GiveKillstreak {
        request_id: ActionRequestId,
        name: [u8; MENU_RESPONSE_BYTES],
    },
}

pub const MENU_RESPONSE_BYTES: usize = 48;

pub fn menu_response_field(text: &str) -> Option<[u8; MENU_RESPONSE_BYTES]> {
    let bytes = text.as_bytes();
    if bytes.len() > MENU_RESPONSE_BYTES {
        return None;
    }
    let mut field = [0u8; MENU_RESPONSE_BYTES];
    field[..bytes.len()].copy_from_slice(bytes);
    Some(field)
}

pub fn menu_response_text(field: &[u8; MENU_RESPONSE_BYTES]) -> &str {
    let len = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    std::str::from_utf8(&field[..len]).unwrap_or("")
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpawnPick {
    Seeded(u64),
    At { origin: [f32; 3], yaw: f32 },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickInput {
    pub cmds: Vec<(ClientId, UserCmd)>,
    pub actions: Vec<(ClientId, ClientAction)>,
}

impl TickInput {
    pub fn from_cmds(cmds: Vec<(ClientId, UserCmd)>) -> Self {
        Self {
            cmds,
            actions: Vec::new(),
        }
    }

    pub fn canonicalize(&mut self) {
        self.cmds.sort_by_key(|(id, _)| id.0);

        self.actions
            .sort_by_key(|(id, action)| (id.0, action_request_id(action)));
    }
}

pub fn action_request_id(action: &ClientAction) -> ActionRequestId {
    match *action {
        ClientAction::JoinMatch { request_id }
        | ClientAction::LeaveMatch { request_id }
        | ClientAction::SelectClass { request_id, .. }
        | ClientAction::GiveWeapon { request_id, .. }
        | ClientAction::ChangeWeaponConfiguration { request_id, .. }
        | ClientAction::ForceDeath { request_id }
        | ClientAction::SpawnClient { request_id }
        | ClientAction::ForceSpawn { request_id, .. }
        | ClientAction::SpawnIntermission { request_id }
        | ClientAction::SetMatchPhase { request_id, .. }
        | ClientAction::Move { request_id, .. }
        | ClientAction::BeginScriptMoverRotateVelocity { request_id, .. }
        | ClientAction::DebugDamage { request_id, .. }
        | ClientAction::SetName { request_id, .. }
        | ClientAction::UseCopycat { request_id }
        | ClientAction::ActionSlot { request_id, .. }
        | ClientAction::ChooseDefaultClass { request_id, .. }
        | ClientAction::MenuResponse { request_id, .. }
        | ClientAction::GiveKillstreak { request_id, .. }
        | ClientAction::ResupplyAmmo { request_id } => request_id,
    }
}
