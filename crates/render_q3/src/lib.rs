use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use net::{FrameClock, LocalPresentClient, PresentedSnapshot};

pub struct RenderQ3Plugin;

// Q3 MD3 viewmodels are authored in a much larger local unit scale than the
// Bevy camera-space bridge. Frame them to an intentional first-person size
// instead of shrinking them to a small world prop.
const Q3_VIEW_TARGET_RADIUS: f32 = 6.0;
const Q3_VIEW_NEAR_MARGIN: f32 = 0.20;

#[derive(Component)]
struct Q3ViewRoot;

#[derive(Component)]
struct Q3ViewSurface;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Q3ProjectileKey {
    Authoritative(sim::ProjectileId),
    Predicted {
        owner: sim::ClientId,
        weapon: sim::Quake3Weapon,
        launch_time: i32,
    },
}

#[derive(Component)]
struct Q3ProjectileVisual {
    key: Q3ProjectileKey,
    weapon: sim::Quake3Weapon,
}

#[derive(Resource, Default)]
struct Q3ProjectileMeshCache {
    rows: HashMap<
        sim::Quake3Weapon,
        Vec<(Handle<Mesh>, Handle<StandardMaterial>)>,
    >,
}

#[derive(Resource, Default)]
struct Q3ViewState {
    weapon: Option<sim::Quake3Weapon>,
    switch_started_ms: i32,
    last_fire_deadline_ms: i32,
    recoil: f32,
}

impl Plugin for RenderQ3Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Q3ViewState>()
            .init_resource::<Q3ProjectileMeshCache>()
            .add_systems(
                Update,
                (
                    ensure_q3_view_root,
                    sync_q3_view_model.after(ensure_q3_view_root),
                    sync_q3_projectiles,
                )
                    .in_set(net::ClientSet::Present),
            );
    }
}

fn ensure_q3_view_root(
    mut commands: Commands,
    world_camera: Query<Entity, With<render_scene::FpvLens>>,
    roots: Query<Entity, With<Q3ViewRoot>>,
) {
    if !roots.is_empty() {
        return;
    }
    let Ok(parent) = world_camera.single() else {
        return;
    };

    // Q3 presentation shares the engine's existing FpvLens camera/render
    // target. Parenting directly to the lens makes the root transform camera-
    // local without creating a second Camera3d.
    let root = commands
        .spawn((
            Q3ViewRoot,
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    commands.entity(parent).add_child(root);
}

fn q3_tag_weapon(model: &asset_q3::Q3WeaponModel) -> Option<&asset_q3::Md3Tag> {
    model
        .hand
        .as_ref()?
        .tags
        .first()?
        .iter()
        .find(|tag| tag.name.eq_ignore_ascii_case("tag_weapon"))
}

fn q3_tag_point(tag: Option<&asset_q3::Md3Tag>, p: [f32; 3]) -> [f32; 3] {
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

fn q3_tag_vector(tag: Option<&asset_q3::Md3Tag>, p: [f32; 3]) -> [f32; 3] {
    let Some(tag) = tag else {
        return normalise(p);
    };
    normalise([
        p[0] * tag.axis[0][0] + p[1] * tag.axis[1][0] + p[2] * tag.axis[2][0],
        p[0] * tag.axis[0][1] + p[1] * tag.axis[1][1] + p[2] * tag.axis[2][1],
        p[0] * tag.axis[0][2] + p[1] * tag.axis[1][2] + p[2] * tag.axis[2][2],
    ])
}

fn normalise(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > f32::EPSILON {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn q3_model_scale(model: &asset_q3::Q3WeaponModel) -> f32 {
    let radius = model
        .model
        .frames
        .first()
        .map(|frame| frame.radius.abs())
        .filter(|radius| *radius > 1.0e-3)
        .unwrap_or(32.0);
    Q3_VIEW_TARGET_RADIUS / radius
}

fn q3_to_camera(v: [f32; 3], scale: f32) -> [f32; 3] {
    // Q3: +X forward, +Y left, +Z up.
    // Bevy camera local: +X right, +Y up, -Z forward.
    [-v[1] * scale, v[2] * scale, -v[0] * scale]
}

fn q3_front_extent(
    model: &asset_q3::Q3WeaponModel,
    tag: Option<&asset_q3::Md3Tag>,
    scale: f32,
) -> f32 {
    model
        .model
        .surfaces
        .iter()
        .filter_map(|surface| surface.frames.first())
        .flat_map(|vertices| vertices.iter())
        .map(|vertex| q3_to_camera(q3_tag_point(tag, vertex.position), scale)[2])
        .fold(0.0_f32, f32::max)
}

fn q3_surface_mesh(
    surface: &asset_q3::Md3Surface,
    tag: Option<&asset_q3::Md3Tag>,
    scale: f32,
) -> Option<Mesh> {
    let vertices = surface.frames.first()?;
    if vertices.len() != surface.texcoords.len() {
        return None;
    }

    let positions: Vec<[f32; 3]> = vertices
        .iter()
        .map(|vertex| q3_to_camera(q3_tag_point(tag, vertex.position), scale))
        .collect();
    let normals: Vec<[f32; 3]> = vertices
        .iter()
        .map(|vertex| q3_to_camera(q3_tag_vector(tag, vertex.normal), 1.0))
        .map(normalise)
        .collect();
    let uvs: Vec<[f32; 2]> = surface
        .texcoords
        .iter()
        .map(|uv| [uv[0], 1.0 - uv[1]])
        .collect();
    let indices: Vec<u32> = surface
        .triangles
        .iter()
        .flat_map(|triangle| [triangle[0], triangle[1], triangle[2]])
        .collect();

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs),
    )
}

fn rebuild_weapon(
    commands: &mut Commands,
    root: Entity,
    model: &asset_q3::Q3WeaponModel,
    q3_assets: &assets::PreparedQ3WeaponModels,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands.entity(root).despawn_children();

    let tag = q3_tag_weapon(model);
    let scale = q3_model_scale(model);
    let mut children = Vec::new();
    for (surface_index, surface) in model.model.surfaces.iter().enumerate() {
        let Some(mesh) = q3_surface_mesh(surface, tag, scale) else {
            continue;
        };
        let image = q3_assets
            .texture_or_fallback(model.weapon, surface_index)
            .cloned();
        let blend = model
            .surface_blends
            .get(surface_index)
            .copied()
            .unwrap_or_default();
        let alpha_mode = match blend {
            asset_q3::Q3BlendMode::Opaque => AlphaMode::Opaque,
            asset_q3::Q3BlendMode::Alpha => AlphaMode::Blend,
            asset_q3::Q3BlendMode::Add => AlphaMode::Add,
            asset_q3::Q3BlendMode::Multiply => AlphaMode::Multiply,
        };
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: image,
            unlit: true,
            alpha_mode,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        });
        let entity = commands
            .spawn((
                Q3ViewSurface,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::default(),
                NoFrustumCulling,
            ))
            .id();
        children.push(entity);
    }
    commands.entity(root).add_children(&children);
}

fn sync_q3_view_model(
    mut commands: Commands,
    q3_assets: Option<Res<assets::PreparedQ3WeaponModels>>,
    presented: Res<PresentedSnapshot>,
    local: Res<LocalPresentClient>,
    clock: Res<FrameClock>,
    mut state: ResMut<Q3ViewState>,
    lenses: Query<&Projection, With<render_scene::FpvLens>>,
    mut roots: Query<(Entity, &mut Transform, &mut Visibility), With<Q3ViewRoot>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((root_entity, mut root_transform, mut root_visibility)) = roots.single_mut() else {
        return;
    };

    let runtime = presented
        .snapshot()
        .and_then(|snapshot| snapshot.meta.for_client(local.0))
        .and_then(|meta| meta.q3_weapon)
        .filter(|runtime| runtime.active);

    let Some(runtime) = runtime else {
        *root_visibility = Visibility::Hidden;
        state.weapon = None;
        state.recoil = 0.0;
        return;
    };


    let Some(q3_assets) = q3_assets.as_ref() else {
        *root_visibility = Visibility::Hidden;
        return;
    };
    let Some(model) = q3_assets.assets.model(runtime.weapon) else {
        *root_visibility = Visibility::Hidden;
        return;
    };

    let now = clock.time();
    if state.weapon != Some(runtime.weapon) {
        rebuild_weapon(
            &mut commands,
            root_entity,
            model,
            q3_assets,
            &mut meshes,
            &mut materials,
        );
        state.weapon = Some(runtime.weapon);
        state.switch_started_ms = now;
        state.last_fire_deadline_ms = runtime.next_fire_time_ms;
        state.recoil = 0.0;
    } else if runtime.next_fire_time_ms > state.last_fire_deadline_ms {
        state.last_fire_deadline_ms = runtime.next_fire_time_ms;
        state.recoil = 1.0;
    }

    state.recoil = (state.recoil - clock.frametime_secs().clamp(0.0, 0.05) * 7.0).max(0.0);

    let speed = presented
        .player(local.0)
        .map(|ps| (ps.velocity[0] * ps.velocity[0] + ps.velocity[1] * ps.velocity[1]).sqrt())
        .unwrap_or(0.0);
    let bob_scale = (speed / 320.0).clamp(0.0, 1.0);
    let phase = now as f32 * 0.008;
    let bob_x = phase.sin() * 0.018 * bob_scale;
    let bob_y = (phase * 2.0).sin().abs() * 0.012 * bob_scale;

    let raise_t = ((now - state.switch_started_ms) as f32 / 250.0).clamp(0.0, 1.0);
    let lower = (1.0 - raise_t) * 0.22;

    let near = lenses
        .single()
        .ok()
        .and_then(|projection| match projection {
            Projection::Perspective(perspective) => Some(perspective.near),
            _ => None,
        })
        .unwrap_or(2.0);

    // Keep the nearest vertex just behind IW4L's near plane. This lets the
    // weapon fill the intended part of the screen without clipping even though
    // different Q3 guns have very different MD3 bounds.
    let scale = q3_model_scale(model);
    let front_extent = q3_front_extent(model, q3_tag_weapon(model), scale);
    let depth = near + Q3_VIEW_NEAR_MARGIN + front_extent.max(0.0);

    root_transform.translation = Vec3::new(
        1.10 + bob_x,
        -0.90 - lower + bob_y,
        -depth + state.recoil * 0.22,
    );
    root_transform.rotation = Quat::from_rotation_x(state.recoil * 0.06);
    *root_visibility = Visibility::Visible;
}


fn q3_world_surface_mesh(surface: &asset_q3::Md3Surface) -> Option<Mesh> {
    let vertices = surface.frames.first()?;
    if vertices.len() != surface.texcoords.len() {
        return None;
    }
    let positions: Vec<[f32; 3]> = vertices.iter().map(|vertex| vertex.position).collect();
    let normals: Vec<[f32; 3]> = vertices.iter().map(|vertex| normalise(vertex.normal)).collect();
    let uvs: Vec<[f32; 2]> = surface
        .texcoords
        .iter()
        .map(|uv| [uv[0], 1.0 - uv[1]])
        .collect();
    let indices: Vec<u32> = surface
        .triangles
        .iter()
        .flat_map(|triangle| [triangle[0], triangle[1], triangle[2]])
        .collect();

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs),
    )
}

fn projectile_meshes(
    weapon: sim::Quake3Weapon,
    q3_assets: &assets::PreparedQ3WeaponModels,
    cache: &mut Q3ProjectileMeshCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Option<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    if let Some(rows) = cache.rows.get(&weapon) {
        return Some(rows.clone());
    }

    let source = q3_assets.assets.projectile(weapon)?;
    let mut rows = Vec::new();
    for (surface_index, surface) in source.model.surfaces.iter().enumerate() {
        let Some(mesh) = q3_world_surface_mesh(surface) else {
            continue;
        };
        let blend = source
            .surface_blends
            .get(surface_index)
            .copied()
            .unwrap_or_default();
        let alpha_mode = match blend {
            asset_q3::Q3BlendMode::Opaque => AlphaMode::Opaque,
            asset_q3::Q3BlendMode::Alpha => AlphaMode::Blend,
            asset_q3::Q3BlendMode::Add => AlphaMode::Add,
            asset_q3::Q3BlendMode::Multiply => AlphaMode::Multiply,
        };
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: q3_assets
                .projectile_texture_or_fallback(weapon, surface_index)
                .cloned(),
            unlit: true,
            alpha_mode,
            cull_mode: None,
            fog_enabled: true,
            ..default()
        });
        rows.push((meshes.add(mesh), material));
    }
    if rows.is_empty() {
        return None;
    }
    cache.rows.insert(weapon, rows.clone());
    Some(rows)
}

fn projectile_key(row: &net::PresentedProjectile) -> Option<Q3ProjectileKey> {
    let weapon = row.q3_weapon()?;
    match row {
        net::PresentedProjectile::Authoritative(projectile) => {
            Some(Q3ProjectileKey::Authoritative(projectile.id))
        }
        net::PresentedProjectile::Predicted {
            owner,
            launch_time,
            ..
        } => Some(Q3ProjectileKey::Predicted {
            owner: *owner,
            weapon,
            launch_time: *launch_time,
        }),
    }
}

fn projectile_origin(
    snapshot: &PresentedSnapshot,
    row: &net::PresentedProjectile,
    at_time: i32,
) -> [f32; 3] {
    match row {
        net::PresentedProjectile::Authoritative(projectile) => {
            snapshot.projectile_origin_at(projectile, at_time)
        }
        net::PresentedProjectile::Predicted { .. } => row.origin_at(at_time),
    }
}

fn projectile_rotation(row: &net::PresentedProjectile, at_time: i32) -> Quat {
    let velocity = row.velocity();
    let direction = Vec3::from_array(velocity);
    if direction.length_squared() > 1.0e-6 {
        Quat::from_rotation_arc(Vec3::X, direction.normalize())
    } else {
        let angles = entity_iw4::evaluate_trajectory(&row.apos(), at_time);
        Quat::from_euler(
            EulerRot::ZYX,
            angles[1].to_radians(),
            angles[0].to_radians(),
            angles[2].to_radians(),
        )
    }
}

fn sync_q3_projectiles(
    mut commands: Commands,
    q3_assets: Option<Res<assets::PreparedQ3WeaponModels>>,
    presented: Res<PresentedSnapshot>,
    clock: Res<FrameClock>,
    mut cache: ResMut<Q3ProjectileMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut visuals: Query<(Entity, &Q3ProjectileVisual, &mut Transform)>,
) {
    let Some(q3_assets) = q3_assets.as_ref() else {
        return;
    };
    let at_time = presented.trajectory_time_ms(clock.time());
    let rows = presented.presented_projectiles();

    let mut live = HashMap::new();
    for row in rows {
        let Some(weapon) = row.q3_weapon() else {
            continue;
        };
        let Some(key) = projectile_key(row) else {
            continue;
        };
        live.insert(key, (weapon, row));
    }

    for (entity, visual, mut transform) in &mut visuals {
        let Some((weapon, row)) = live.remove(&visual.key) else {
            commands.entity(entity).despawn();
            continue;
        };
        if weapon != visual.weapon {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation = Vec3::from_array(projectile_origin(&presented, row, at_time));
        transform.rotation = projectile_rotation(row, at_time);
    }

    for (key, (weapon, row)) in live {
        let Some(parts) = projectile_meshes(
            weapon,
            q3_assets,
            &mut cache,
            &mut meshes,
            &mut materials,
        ) else {
            // Plasma/BFG are shader-based Q3 effects and deliberately remain
            // without an IW4 stand-in until their native effect lane lands.
            continue;
        };

        let root = commands
            .spawn((
                Q3ProjectileVisual { key, weapon },
                Transform {
                    translation: Vec3::from_array(projectile_origin(&presented, row, at_time)),
                    rotation: projectile_rotation(row, at_time),
                    ..default()
                },
                Visibility::Visible,
            ))
            .id();
        let mut children = Vec::with_capacity(parts.len());
        for (mesh, material) in parts {
            children.push(
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::default(),
                    ))
                    .id(),
            );
        }
        commands.entity(root).add_children(&children);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_axes_map_forward_to_camera_forward() {
        let scale = 0.25;
        assert_eq!(q3_to_camera([1.0, 0.0, 0.0], scale), [0.0, 0.0, -scale]);
        assert_eq!(q3_to_camera([0.0, 1.0, 0.0], scale), [-scale, 0.0, 0.0]);
        assert_eq!(q3_to_camera([0.0, 0.0, 1.0], scale), [0.0, scale, 0.0]);
    }

    #[test]
    fn q3_viewmodel_target_radius_is_first_person_sized() {
        assert!(Q3_VIEW_TARGET_RADIUS >= 5.0);
        assert!(Q3_VIEW_NEAR_MARGIN > 0.0);
    }
}
