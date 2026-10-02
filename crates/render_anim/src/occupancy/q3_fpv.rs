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
    // IW4 uses the VU half-float packing used by GfxPackedVertex.
    (u32::from(f32_to_half(uv[0])) << 16) | u32::from(f32_to_half(uv[1]))
}

fn normalise(v: [f32; 3]) -> [f32; 3] {
    let len_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if len_sq <= f32::EPSILON {
        return [0.0, 0.0, 1.0];
    }
    let inv = len_sq.sqrt().recip();
    [v[0] * inv, v[1] * inv, v[2] * inv]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn pack_unit_vec(v: [f32; 3]) -> u32 {
    // IW4's scale-based PackedUnitVec decode is:
    // (byte - 127) * ((scale_byte + 192) / 32385).
    // scale_byte=63 makes that exactly (byte - 127) / 127.
    let v = normalise(v);
    let enc = |x: f32| -> u8 {
        (x.clamp(-1.0, 1.0) * 127.0 + 127.0)
            .round()
            .clamp(0.0, 254.0) as u8
    };
    u32::from(enc(v[0]))
        | (u32::from(enc(v[1])) << 8)
        | (u32::from(enc(v[2])) << 16)
        | (63u32 << 24)
}

fn tangent_frames(surface: &asset_q3::Md3Surface) -> Vec<([f32; 3], f32)> {
    let Some(vertices) = surface.frames.first() else {
        return Vec::new();
    };
    let n = vertices.len();
    let mut tan = vec![[0.0; 3]; n];
    let mut bitan = vec![[0.0; 3]; n];

    for tri in &surface.triangles {
        let [i0, i1, i2] = tri.map(|v| v as usize);
        if i0 >= n || i1 >= n || i2 >= n {
            continue;
        }
        let p0 = vertices[i0].position;
        let p1 = vertices[i1].position;
        let p2 = vertices[i2].position;
        let uv0 = surface.texcoords[i0];
        let uv1 = surface.texcoords[i1];
        let uv2 = surface.texcoords[i2];

        let e1 = sub(p1, p0);
        let e2 = sub(p2, p0);
        let du1 = uv1[0] - uv0[0];
        let dv1 = uv1[1] - uv0[1];
        let du2 = uv2[0] - uv0[0];
        let dv2 = uv2[1] - uv0[1];
        let det = du1 * dv2 - dv1 * du2;
        if det.abs() <= 1.0e-8 {
            continue;
        }
        let inv = det.recip();
        let t = scale(
            sub(scale(e1, dv2), scale(e2, dv1)),
            inv,
        );
        let b = scale(
            sub(scale(e2, du1), scale(e1, du2)),
            inv,
        );
        for i in [i0, i1, i2] {
            tan[i] = add(tan[i], t);
            bitan[i] = add(bitan[i], b);
        }
    }

    vertices
        .iter()
        .enumerate()
        .map(|(i, vertex)| {
            let nrm = normalise(vertex.normal);
            let projected = sub(tan[i], scale(nrm, dot(nrm, tan[i])));
            let tangent = if dot(projected, projected) <= 1.0e-8 {
                let seed = if nrm[2].abs() < 0.9 {
                    [0.0, 0.0, 1.0]
                } else {
                    [0.0, 1.0, 0.0]
                };
                normalise(cross(seed, nrm))
            } else {
                normalise(projected)
            };
            let sign = if dot(cross(nrm, tangent), bitan[i]) < 0.0 {
                -1.0
            } else {
                1.0
            };
            (tangent, sign)
        })
        .collect()
}

fn packed_vertex(
    position: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 3],
    binormal_sign: f32,
    uv: [f32; 2],
) -> [u8; asset_iw4::size::GFX_PACKED_VERTEX] {
    let mut row = [0u8; asset_iw4::size::GFX_PACKED_VERTEX];
    row[0..4].copy_from_slice(&position[0].to_le_bytes());
    row[4..8].copy_from_slice(&position[1].to_le_bytes());
    row[8..12].copy_from_slice(&position[2].to_le_bytes());
    row[12..16].copy_from_slice(&binormal_sign.to_le_bytes());
    row[16..20].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    row[20..24].copy_from_slice(&pack_uv(uv).to_le_bytes());
    row[24..28].copy_from_slice(&pack_unit_vec(normal).to_le_bytes());
    row[28..32].copy_from_slice(&pack_unit_vec(tangent).to_le_bytes());
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

fn transform_vector(tag: Option<&asset_q3::Md3Tag>, p: [f32; 3]) -> [f32; 3] {
    let Some(tag) = tag else {
        return normalise(p);
    };
    normalise([
        p[0] * tag.axis[0][0] + p[1] * tag.axis[1][0] + p[2] * tag.axis[2][0],
        p[0] * tag.axis[0][1] + p[1] * tag.axis[1][1] + p[2] * tag.axis[2][1],
        p[0] * tag.axis[0][2] + p[1] * tag.axis[1][2] + p[2] * tag.axis[2][2],
    ])
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

#[derive(Resource, Default)]
pub(crate) struct Q3FpvTextureSwap {
    weapon: u8,
    base_materials: Vec<render_scene::SmodelPassMaterial>,
    surface_materials: Vec<usize>,
    slots: Vec<(usize, Option<Handle<Image>>)>,
}

fn restore_q3_texture_slots(
    images: &mut assets::image_handles::RuntimeImageHandles,
    state: &mut Q3FpvTextureSwap,
) {
    if state.slots.is_empty() {
        return;
    }
    let pool = images.make_mut();
    for (slot, original) in state.slots.drain(..) {
        if let Some(dst) = pool.material_images.get_mut(slot) {
            *dst = original;
        }
    }
}

fn q3_base_material_candidates(
    base: &[render_scene::SmodelPassMaterial],
) -> Vec<usize> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (index, material) in base.iter().enumerate() {
        let Some(color) = material.color.as_ref() else {
            continue;
        };
        if seen.insert(color.id()) {
            out.push(index);
        }
    }
    if out.is_empty() && !base.is_empty() {
        out.push(0);
    }
    out
}

fn bind_q3_surface_textures(
    q3: &assets::PreparedQ3WeaponModels,
    weapon: weapon_q3::Quake3Weapon,
    surface_count: usize,
    images: &mut assets::image_handles::RuntimeImageHandles,
    state: &mut Q3FpvTextureSwap,
) {
    restore_q3_texture_slots(images, state);
    state.surface_materials.clear();

    let candidates = q3_base_material_candidates(&state.base_materials);
    if candidates.is_empty() {
        state.weapon = weapon as u8;
        return;
    }

    let pool = images.make_mut();
    for surface in 0..surface_count {
        let base_index = candidates[surface % candidates.len()];
        state.surface_materials.push(base_index);

        let Some(texture) = q3.texture_or_fallback(weapon, surface).cloned() else {
            continue;
        };
        let Some(base_color) = state.base_materials[base_index].color.as_ref() else {
            continue;
        };
        let Some(slot) = pool
            .material_images
            .iter()
            .position(|handle| handle.as_ref().is_some_and(|handle| handle.id() == base_color.id()))
        else {
            continue;
        };

        if !state.slots.iter().any(|(existing, _)| *existing == slot) {
            state.slots.push((slot, pool.material_images[slot].clone()));
        }
        pool.material_images[slot] = Some(texture);
    }
    state.weapon = weapon as u8;
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
        -raise_down + bob_up,
        recoil_back,
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
    mut runtime_images: Option<ResMut<assets::image_handles::RuntimeImageHandles>>,
    mut texture_swap: ResMut<Q3FpvTextureSwap>,
    mut plan: ResMut<FpvDrawPlan>,
) {
    let runtime = presented
        .snapshot()
        .and_then(|snapshot| snapshot.meta.for_client(local.0))
        .and_then(|meta| meta.q3_weapon);

    let Some(runtime) = runtime.filter(|runtime| runtime.active) else {
        if let Some(images) = runtime_images.as_deref_mut() {
            restore_q3_texture_slots(images, &mut texture_swap);
        }
        texture_swap.weapon = 0;
        texture_swap.surface_materials.clear();
        texture_swap.base_materials.clear();
        return;
    };

    let Some(q3) = q3.as_ref() else {
        return;
    };
    let Some(entry) = q3.assets.model(runtime.weapon) else {
        return;
    };

    // Capture the real IW4 viewmodel materials once, before Q3 replaces the
    // plan. Their material ordinals give us a valid IW4 shader/technique while
    // we retarget the exact colour-image slots to the Q3 skin.
    if texture_swap.base_materials.is_empty() {
        if plan.materials.is_empty() {
            return;
        }
        texture_swap.base_materials = plan.materials.clone();
    }

    if texture_swap.weapon != runtime.weapon as u8
        || texture_swap.surface_materials.len() != entry.model.surfaces.len()
    {
        let Some(images) = runtime_images.as_deref_mut() else {
            return;
        };
        bind_q3_surface_textures(
            q3,
            runtime.weapon,
            entry.model.surfaces.len(),
            images,
            &mut texture_swap,
        );
    }

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

        let frames = tangent_frames(surface);
        if frames.len() != vertices.len() {
            continue;
        }

        let vertex_base = packed.len() as u32;
        for ((vertex, uv), (tangent, sign)) in vertices
            .iter()
            .zip(&surface.texcoords)
            .zip(frames.into_iter())
        {
            let position = transform_point(tag, vertex.position);
            let normal = transform_vector(tag, vertex.normal);
            let tangent = transform_vector(tag, tangent);
            packed.push(packed_vertex(position, normal, tangent, sign, *uv));
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
            let base_index = texture_swap
                .surface_materials
                .get(surface_index)
                .copied()
                .unwrap_or(0)
                .min(texture_swap.base_materials.len().saturating_sub(1));
            let mut material = texture_swap.base_materials[base_index].clone();
            material.color = q3
                .texture_or_fallback(runtime.weapon, surface_index)
                .cloned();
            material.specular = None;
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
    plan.revisions.bump_surfaces();
    plan.revisions.bump_materials();
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
        let row = packed_vertex(
            [1.0, 2.0, 3.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            1.0,
            [0.5, 0.5],
        );
        assert_eq!(row.len(), asset_iw4::size::GFX_PACKED_VERTEX);
    }
}
