use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use net::{FrameClock, LocalPresentClient, PresentedSnapshot};

pub struct RenderQ3Plugin;

const Q3_VIEW_LAYER: usize = 31;
const Q3_VIEW_FOV_DEGREES: f32 = 72.0;
const Q3_MODEL_SCALE: f32 = 0.025;

#[derive(Component)]
struct Q3ViewCamera;

#[derive(Component)]
struct Q3ViewRoot;

#[derive(Component)]
struct Q3ViewSurface;

#[derive(Resource, Default)]
struct Q3ViewState {
    weapon: Option<sim::Quake3Weapon>,
    switch_started_ms: i32,
    last_fire_deadline_ms: i32,
    recoil: f32,
}

impl Plugin for RenderQ3Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Q3ViewState>().add_systems(
            Update,
            (
                ensure_q3_view_camera,
                sync_q3_view_model.after(ensure_q3_view_camera),
            )
                .in_set(net::ClientSet::Present),
        );
    }
}

fn ensure_q3_view_camera(
    mut commands: Commands,
    world_camera: Query<Entity, With<render_scene::FlyCamera>>,
    q3_camera: Query<Entity, With<Q3ViewCamera>>,
) {
    if !q3_camera.is_empty() {
        return;
    }
    let Ok(parent) = world_camera.single() else {
        return;
    };

    let camera = commands
        .spawn((
            Q3ViewCamera,
            Camera3d::default(),
            Camera {
                order: 100,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Projection::from(PerspectiveProjection {
                fov: Q3_VIEW_FOV_DEGREES.to_radians(),
                near: 0.01,
                far: 10.0,
                ..default()
            }),
            RenderLayers::layer(Q3_VIEW_LAYER),
            Transform::default(),
        ))
        .id();
    commands.entity(parent).add_child(camera);

    let root = commands
        .spawn((
            Q3ViewRoot,
            Transform::default(),
            Visibility::Hidden,
            RenderLayers::layer(Q3_VIEW_LAYER),
        ))
        .id();
    commands.entity(camera).add_child(root);
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

fn q3_to_camera(v: [f32; 3]) -> [f32; 3] {
    // Q3: +X forward, +Y left, +Z up.
    // Bevy camera local: +X right, +Y up, -Z forward.
    [
        -v[1] * Q3_MODEL_SCALE,
        v[2] * Q3_MODEL_SCALE,
        -v[0] * Q3_MODEL_SCALE,
    ]
}

fn q3_surface_mesh(
    surface: &asset_q3::Md3Surface,
    tag: Option<&asset_q3::Md3Tag>,
) -> Option<Mesh> {
    let vertices = surface.frames.first()?;
    if vertices.len() != surface.texcoords.len() {
        return None;
    }

    let positions: Vec<[f32; 3]> = vertices
        .iter()
        .map(|vertex| q3_to_camera(q3_tag_point(tag, vertex.position)))
        .collect();
    let normals: Vec<[f32; 3]> = vertices
        .iter()
        .map(|vertex| q3_to_camera(q3_tag_vector(tag, vertex.normal)))
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
    let mut children = Vec::new();
    for (surface_index, surface) in model.model.surfaces.iter().enumerate() {
        let Some(mesh) = q3_surface_mesh(surface, tag) else {
            continue;
        };
        let image = q3_assets
            .texture_or_fallback(model.weapon, surface_index)
            .cloned();
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: image,
            unlit: true,
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
                RenderLayers::layer(Q3_VIEW_LAYER),
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

    root_transform.translation = Vec3::new(
        0.13 + bob_x,
        -0.18 - lower + bob_y,
        -0.38 + state.recoil * 0.06,
    );
    root_transform.rotation = Quat::from_rotation_x(state.recoil * 0.06);
    *root_visibility = Visibility::Visible;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q3_axes_map_forward_to_camera_forward() {
        assert_eq!(q3_to_camera([1.0, 0.0, 0.0]), [0.0, 0.0, -Q3_MODEL_SCALE]);
        assert_eq!(q3_to_camera([0.0, 1.0, 0.0]), [-Q3_MODEL_SCALE, 0.0, 0.0]);
        assert_eq!(q3_to_camera([0.0, 0.0, 1.0]), [0.0, Q3_MODEL_SCALE, 0.0]);
    }
}
