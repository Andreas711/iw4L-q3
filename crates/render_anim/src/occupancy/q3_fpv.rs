use bevy::prelude::*;
use net::{FrameClock, LocalPresentClient, PresentedSnapshot};
use render_scene::FlyCamera;

use crate::draw::{FpvDrawPlan, FpvSurfaceDraw};

fn f32_to_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7f_ff_ff;

    if exponent <= 0 {
        if exponent < -10 {
            return sign;
        }
        let mantissa = mantissa | 0x80_00_00;
        let shift = 14 - exponent;
        let mut half = (mantissa >> shift) as u16;
        if (mantissa >> (shift - 1)) & 1 != 0 {
            half = half.wrapping_add(1);
        }
        sign | half
    } else if exponent >= 31 {
        sign | 0x7c00
    } else {
        let mut half = sign | ((exponent as u16) << 10) | ((mantissa >> 13) as u16);
        if mantissa & 0x1000 != 0 {
            half = half.wrapping_add(1);
        }
        half
    }
}

fn pack_uv(uv: [f32; 2]) -> u32 {
    (u32::from(f32_to_half(uv[0])) << 16) | u32::from(f32_to_half(uv[1]))
}

fn packed_vertex(
    mut row: [u8; asset_iw4::size::GFX_PACKED_VERTEX],
    position: [f32; 3],
    uv: [f32; 2],
) -> [u8; asset_iw4::size::GFX_PACKED_VERTEX] {
    row[0..4].copy_from_slice(&position[0].to_le_bytes());
    row[4..8].copy_from_slice(&position[1].to_le_bytes());
    row[8..12].copy_from_slice(&position[2].to_le_bytes());
    row[16..20].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    row[20..24].copy_from_slice(&pack_uv(uv).to_le_bytes());
    row
}

fn tag_weapon(model: &asset_q3::Q3WeaponModel) -> Option<&asset_q3::Md3Tag> {
    model
        .hand
        .as_ref()?
        .tags
        .first()?
        .iter()
        .find(|tag| tag.name.eq_ignore_ascii_case("tag_weapon"))
}

fn transform_point(tag: Option<&asset_q3::Md3Tag>, p: [f32; 3]) -> [f32; 3] {
    let Some(tag) = tag else {
        return p;
    };
    [
        tag.origin[0]
            + p[0] * tag.axis[0][0]
            + p[1] * tag.axis[1][0]
            + p[2] * tag.axis[2][0],
        tag.origin[1]
            + p[0] * tag.axis[0][1]
            + p[1] * tag.axis[1][1]
            + p[2] * tag.axis[2][1],
        tag.origin[2]
            + p[0] * tag.axis[0][2]
            + p[1] * tag.axis[1][2]
            + p[2] * tag.axis[2][2],
    ]
}

#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct Q3FpvMotion {
    weapon: u8,
    switch_started_ms: i32,
    last_fire_deadline_ms: i32,
    recoil: f32,
}

impl Default for Q3FpvMotion {
    fn default() -> Self {
        Self {
            weapon: 0,
            switch_started_ms: 0,
            last_fire_deadline_ms: 0,
            recoil: 0.0,
        }
    }
}

/// Quake III weapon models use X-forward, Y-left, Z-up coordinates.
/// Bevy camera-local coordinates are X-right, Y-up, -Z-forward.
fn q3_model_to_camera() -> Mat4 {
    Mat4::from_cols(
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(-1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::W,
    )
}

pub(crate) fn override_q3_fpv_placement(
    clock: Res<FrameClock>,
    presented: Res<PresentedSnapshot>,
    local: Res<LocalPresentClient>,
    cameras: Query<&Transform, With<FlyCamera>>,
    mut motion: ResMut<Q3FpvMotion>,
    mut plan: ResMut<FpvDrawPlan>,
) {
    let Some(snapshot) = presented.snapshot() else {
        return;
    };
    let Some(runtime) = snapshot.meta.for_client(local.0).and_then(|meta| meta.q3_weapon) else {
        return;
    };
    if !runtime.active {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };

    let now = clock.time();
    let weapon = runtime.weapon as u8;
    if motion.weapon != weapon {
        motion.weapon = weapon;
        motion.switch_started_ms = now;
        motion.last_fire_deadline_ms = runtime.next_fire_time_ms;
        motion.recoil = 0.0;
    } else if runtime.next_fire_time_ms != motion.last_fire_deadline_ms {
        if runtime.next_fire_time_ms > motion.last_fire_deadline_ms {
            motion.recoil = 1.0;
        }
        motion.last_fire_deadline_ms = runtime.next_fire_time_ms;
    }

    let dt = clock.frametime_secs().clamp(0.0, 0.05);
    motion.recoil = (motion.recoil - dt * 7.5).max(0.0);

    let speed = presented
        .player(local.0)
        .map(|ps| (ps.velocity[0] * ps.velocity[0] + ps.velocity[1] * ps.velocity[1]).sqrt())
        .unwrap_or(0.0);
    let bob_scale = (speed / 320.0).clamp(0.0, 1.0);
    let phase = now as f32 * 0.008;
    let bob_right = phase.sin() * 0.55 * bob_scale;
    let bob_up = (phase * 2.0).sin().abs() * 0.45 * bob_scale;

    // Q3 changes weapon in two stages: 200 ms drop, then 250 ms raise.
    // The selected snapshot already names the new weapon, so present the raise
    // half here instead of drawing the previous model during the drop.
    let raise_t = ((now - motion.switch_started_ms) as f32 / 250.0).clamp(0.0, 1.0);
    let raise_down = (1.0 - raise_t) * 14.0;

    let recoil_back = motion.recoil * 2.0;
    let recoil_pitch = motion.recoil * 2.5_f32.to_radians();

    let local = Mat4::from_translation(Vec3::new(
        bob_right,
        -raise_down + bob_up - 1.0,
        recoil_back - 2.0,
    )) * Mat4::from_rotation_x(recoil_pitch)
        * q3_model_to_camera();

    plan.world_from_local = camera.to_matrix() * local;
    plan.placement_ok = true;
    plan.settle_visible();
}

pub(crate) fn override_q3_fpv(
    q3: Option<Res<assets::PreparedQ3WeaponModels>>,
    presented: Res<PresentedSnapshot>,
    local: Res<LocalPresentClient>,
    mut plan: ResMut<FpvDrawPlan>,
) {
    let Some(snapshot) = presented.snapshot() else {
        return;
    };
    let Some(runtime) = snapshot.meta.for_client(local.0).and_then(|meta| meta.q3_weapon) else {
        return;
    };
    if !runtime.active {
        return;
    }

    let Some(q3) = q3.as_ref() else {
        return;
    };
    let Some(entry) = q3.assets.model(runtime.weapon) else {
        return;
    };
    let Some(base_material) = plan.materials.first().cloned() else {
        return;
    };

    let packed_template = match &plan.packed_vertices {
        asset_world::PackedVertexPayload::Iw4(rows) => rows.first().copied(),
        asset_world::PackedVertexPayload::Unavailable { .. } => None,
    };
    let Some(packed_template) = packed_template else {
        return;
    };

    let tag = tag_weapon(entry);
    let mut packed = Vec::new();
    let mut indices = Vec::new();
    let mut ranges = Vec::new();
    let mut materials = Vec::new();

    for (surface_index, surface) in entry.model.surfaces.iter().enumerate() {
        let Some(vertices) = surface.frames.first() else {
            continue;
        };
        if vertices.len() != surface.texcoords.len() {
            continue;
        }

        let vertex_base = packed.len() as u32;
        for (vertex, uv) in vertices.iter().zip(&surface.texcoords) {
            let position = transform_point(tag, vertex.position);
            packed.push(packed_vertex(packed_template, position, *uv));
        }

        let start = indices.len() as u32;
        for tri in &surface.triangles {
            // Quake III uses the opposite winding to the IW4 viewmodel lane.
            indices.extend([
                vertex_base + tri[0],
                vertex_base + tri[2],
                vertex_base + tri[1],
            ]);
        }
        let count = indices.len() as u32 - start;
        if count > 0 {
            ranges.push((start, count));
            let mut material = base_material.clone();
            if let Some(image) = q3.texture(runtime.weapon, surface_index) {
                material.color = Some(image.clone());
                material.specular = None;
            }
            materials.push(material);
        }
    }

    if packed.is_empty() || ranges.is_empty() {
        return;
    }

    plan.decoded_n = packed.len();
    plan.indices = indices;
    plan.surface_ranges = ranges;
    plan.materials = materials;
    plan.draws = (0..plan.surface_ranges.len())
        .map(|surface| FpvSurfaceDraw {
            surface: surface as u32,
            material: surface as u32,
            is_scope: false,
        })
        .collect();
    plan.packed_vertices = asset_world::PackedVertexPayload::Iw4(packed);
    plan.hands_plan_n = Some(0);
    plan.gun_plan_n = Some(plan.decoded_n as u32);
    plan.scope_plan_n = Some(0);
    plan.scope_house_plan_n = Some(0);
    plan.scope_lens_plan_n = Some(0);
    plan.plan_draw_n = Some(plan.draws.len() as u32);
    plan.plan_skip_n = Some(0);
    plan.geometry_ok = true;
    let topology = {
        let mut revisions = render_frame::SourceRevisions::default();
        revisions.set_topology_from(&plan.indices, &plan.surface_ranges, plan.decoded_n);
        revisions.topology
    };
    plan.revisions.topology = topology;
    plan.revisions.bump_vertices();
    plan.revisions.bump_draws();
    plan.revision = plan.revision.wrapping_add(1);
    plan.settle_visible();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_round_trip_basics() {
        assert_eq!(f32_to_half(0.0), 0);
        assert_eq!(f32_to_half(1.0), 0x3c00);
        assert_eq!(f32_to_half(-1.0), 0xbc00);
    }

    #[test]
    fn packed_vertex_uses_iw4_stride() {
        let row = packed_vertex([0u8; asset_iw4::size::GFX_PACKED_VERTEX], [1.0, 2.0, 3.0], [0.5, 0.5]);
        assert_eq!(row.len(), asset_iw4::size::GFX_PACKED_VERTEX);
    }
}
