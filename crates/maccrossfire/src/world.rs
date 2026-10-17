//! The 3D scene: ground tiles, wall blocks, upright picture cards, light, fog, and camera.
//!
//! 1 tile = 1 world unit. Map x is world +X, map y is world +Z (north is -Z).
//! World tile coords = MapView origin + view coords, so they stay put when the map scrolls.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use maccrossfire::terrain::{ground_height, is_wall};

use crate::input::Facing;
use crate::{AppState, Faces, LastComc, Session, View, ViewSize};

const TILE_PX: f32 = 32.0;
const MAX_FACE_PX: u32 = 2048;
const PITCH_DEG: f32 = 50.0;
const ZOOM_MIN: f32 = 6.0;
const ZOOM_MAX: f32 = 22.0;
const SUN_LUX: f32 = 10_000.0;
const FOG_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(FOG_COLOR))
            .insert_resource(GlobalAmbientLight { brightness: 150.0, ..default() })
            .init_resource::<Textures>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (load_textures, sync_visuals, glide, follow_player, place_arrow, lighting)
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(AppState::InGame), spawn_arrow)
            .add_systems(OnExit(AppState::InGame), (despawn_visuals, despawn_arrow));
    }
}

/// One drawn thing: a ground tile, wall, or card on one layer of one world tile.
#[derive(Component)]
pub struct TileVisual {
    pub x: i32,
    pub y: i32,
    pub layer: u8,
    face: u16,
    darkness: u8,
}

/// Moves toward `target` over the current step time. Used for creatures and the camera.
#[derive(Component)]
struct Glide {
    target: Vec3,
}

#[derive(Component)]
struct CameraRig {
    focus: Vec3,
    distance: f32,
}

#[derive(Component)]
struct Sun;

#[derive(Resource, Default)]
struct Textures {
    images: HashMap<u16, (Handle<Image>, Vec2)>,
    ground: Handle<Mesh>,
    cube: Handle<Mesh>,
    card: Handle<Mesh>,
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut textures: ResMut<Textures>) {
    textures.ground = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    textures.cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    // Unit card with its origin at the bottom middle, so scaling grows it up and sideways.
    textures.card = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));

    let distance = 12.0;
    commands.spawn((
        Camera3d::default(),
        camera_transform(Vec3::ZERO, distance),
        CameraRig { focus: Vec3::ZERO, distance },
        Glide { target: Vec3::ZERO },
        DistanceFog { color: FOG_COLOR, falloff: FogFalloff::Linear { start: 15.0, end: 25.0 }, ..default() },
    ));
    commands.spawn((
        Sun,
        DirectionalLight { illuminance: SUN_LUX, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(-4.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn camera_transform(focus: Vec3, distance: f32) -> Transform {
    let pitch = PITCH_DEG.to_radians();
    let eye = focus + Vec3::new(0.0, pitch.sin() * distance, pitch.cos() * distance);
    Transform::from_translation(eye).looking_at(focus, Vec3::Y)
}

/// Turns newly arrived face PNGs into GPU images.
fn load_textures(
    mut faces: ResMut<Faces>,
    mut textures: ResMut<Textures>,
    mut images: ResMut<Assets<Image>>,
    session: Option<NonSend<Session>>,
) {
    for num in faces.0.take_new() {
        let Some(png) = faces.0.png(num) else { continue };
        let image = Image::from_buffer(
            png,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::nearest(),
            RenderAssetUsages::RENDER_WORLD,
        );
        match image {
            Ok(image) if usable_size(image.size()) => {
                let size = image.size().as_vec2();
                textures.images.insert(num, (images.add(image), size));
            }
            Ok(image) => warn!("face {num}: size {} is too big, skipped", image.size()),
            Err(e) => {
                warn!("face {num}: bad PNG: {e}");
                let ask = faces.0.on_bad_png(num);
                if let Some(s) = &session {
                    s.send(ask);
                }
            }
        }
    }
}

/// Server images become GPU textures; a huge one would fail on the GPU and crash.
fn usable_size(size: UVec2) -> bool {
    size.x > 0 && size.y > 0 && size.max_element() <= MAX_FACE_PX
}

/// What should be on screen, keyed by (world x, world y, layer).
type Wanted = HashMap<(i32, i32, u8), (u16, u8)>;

fn wanted(view: &View, faces: &Faces, textures: &Textures) -> Wanted {
    let mut out = Wanted::new();
    for ((vx, vy), cell) in view.0.cells() {
        let (wx, wy) = view.0.world(vx, vy);
        for (layer, raw) in cell.layers.iter().enumerate() {
            let face = faces.0.resolve(*raw);
            if face != 0 && textures.images.contains_key(&face) {
                out.insert((wx, wy, layer as u8), (face, cell.darkness));
            }
        }
    }
    out
}

/// Creatures and flying things glide between tiles; everything else snaps.
fn glides(layer: u8) -> bool {
    layer >= 6
}

#[allow(clippy::too_many_arguments)]
fn sync_visuals(
    mut commands: Commands,
    view: Res<View>,
    faces: Res<Faces>,
    textures: Res<Textures>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing: Query<(Entity, &mut TileVisual, Option<&mut Glide>)>,
) {
    if !view.is_changed() && !textures.is_changed() {
        return;
    }
    // ponytail: full diff per change; incremental dirty cells if profiling says so.
    let mut want = wanted(&view, &faces, &textures);
    let floor_height = |x: i32, y: i32, want: &Wanted| {
        want.get(&(x, y, 0)).and_then(|(f, _)| faces.0.name(*f)).map_or(0.0, ground_height)
    };
    let heights: HashMap<(i32, i32), f32> =
        want.keys().map(|&(x, y, _)| ((x, y), floor_height(x, y, &want))).collect();

    // Keep what is unchanged. Gliding things that left their tile become orphans.
    let mut orphans = Vec::new();
    for (entity, tv, _) in &existing {
        match want.get(&(tv.x, tv.y, tv.layer)) {
            Some(&(face, dark)) if face == tv.face && dark == tv.darkness => {
                want.remove(&(tv.x, tv.y, tv.layer));
            }
            _ if glides(tv.layer) => orphans.push(entity),
            _ => commands.entity(entity).despawn(),
        }
    }

    for ((x, y, layer), (face, darkness)) in want {
        let Some((image, size)) = textures.images.get(&face) else { continue };
        let name = faces.0.name(face).unwrap_or("");
        let h = heights.get(&(x, y)).copied().unwrap_or(0.0);
        let shade = darkness as f32 / 255.0;
        let tile = Vec3::new(x as f32, h, y as f32);

        // A creature that stepped one tile: move the old entity instead of making a new one.
        if glides(layer) {
            let found = orphans.iter().position(|&e| {
                existing.get(e).is_ok_and(|(_, tv, _)| {
                    tv.face == face && tv.layer == layer && (tv.x - x).abs() <= 1 && (tv.y - y).abs() <= 1
                })
            });
            if let Some(i) = found {
                let e = orphans.swap_remove(i);
                let (_, mut tv, glide) = existing.get_mut(e).unwrap();
                (tv.x, tv.y, tv.darkness) = (x, y, darkness);
                if let Some(mut g) = glide {
                    g.target = card_position(tile, *size, layer);
                }
                continue;
            }
        }

        let wall = is_wall(name);
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(shade, shade, shade),
            base_color_texture: Some(image.clone()),
            // Wall art has see-through parts; solid blocks must not show them as holes.
            alpha_mode: if wall { AlphaMode::Opaque } else { AlphaMode::Mask(0.5) },
            perceptual_roughness: 0.9,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        let tv = TileVisual { x, y, layer, face, darkness };
        if wall {
            let t = Transform::from_translation(tile + Vec3::new(0.5, 0.5, 0.5));
            commands.spawn((tv, Mesh3d(textures.cube.clone()), MeshMaterial3d(material), t));
        } else if layer == 0 {
            let t = Transform::from_translation(tile + Vec3::new(0.5, 0.0, 0.5));
            commands.spawn((tv, Mesh3d(textures.ground.clone()), MeshMaterial3d(material), t));
        } else {
            let pos = card_position(tile, *size, layer);
            let t = Transform::from_translation(pos).with_scale(Vec3::new(size.x / TILE_PX, size.y / TILE_PX, 1.0));
            let mut e = commands.spawn((tv, Mesh3d(textures.card.clone()), MeshMaterial3d(material), t));
            if glides(layer) {
                e.insert(Glide { target: pos });
            }
        }
    }
    for e in orphans {
        commands.entity(e).despawn();
    }
}

/// Big images are sent at their bottom-right tile, so the card's right edge sits on
/// that tile's right edge. Higher layers sit a little nearer the camera.
fn card_position(tile: Vec3, size: Vec2, layer: u8) -> Vec3 {
    let width = size.x / TILE_PX;
    tile + Vec3::new(1.0 - width / 2.0, 0.0, 0.5 + layer as f32 * 0.01)
}

fn glide(time: Res<Time>, comc: Res<LastComc>, mut q: Query<(&mut Transform, &Glide), Without<CameraRig>>) {
    let step_secs = comc.time_ms.clamp(100, 400) as f32 / 1000.0;
    let max = time.delta_secs() / step_secs;
    for (mut t, g) in &mut q {
        let d = g.target - t.translation;
        // Far jumps (teleports, new maps) snap.
        t.translation = if d.length() > 2.0 { g.target } else { t.translation + d.clamp_length_max(max) };
    }
}

fn follow_player(
    time: Res<Time>,
    comc: Res<LastComc>,
    view: Res<View>,
    size: Res<ViewSize>,
    scroll: Res<AccumulatedMouseScroll>,
    rig: Single<(&mut Transform, &mut CameraRig, &mut DistanceFog)>,
) {
    let (mut t, mut rig, mut fog) = rig.into_inner();
    rig.distance = (rig.distance - scroll.delta.y).clamp(ZOOM_MIN, ZOOM_MAX);
    let (px, py) = view.0.world((size.0 / 2) as i8, (size.1 / 2) as i8);
    let target = Vec3::new(px as f32 + 0.5, 0.0, py as f32 + 0.5);
    let step_secs = comc.time_ms.clamp(100, 400) as f32 / 1000.0;
    let d = target - rig.focus;
    rig.focus = if d.length() > 2.0 { target } else { rig.focus + d.clamp_length_max(time.delta_secs() / step_secs) };
    *t = camera_transform(rig.focus, rig.distance);
    // Hide the edge of what the server sends: fog from 40% of the view radius to the edge.
    let radius = size.0.min(size.1) as f32 / 2.0;
    fog.falloff = FogFalloff::Linear { start: rig.distance + radius * 0.4, end: rig.distance + radius };
}

/// The sun fades with the darkness of the player's tile, so dungeons are not sunlit.
fn lighting(view: Res<View>, size: Res<ViewSize>, mut sun: Single<&mut DirectionalLight, With<Sun>>) {
    let dark = view.0.cell((size.0 / 2) as i8, (size.1 / 2) as i8).map_or(255, |c| c.darkness);
    sun.illuminance = SUN_LUX * dark as f32 / 255.0;
}

fn despawn_visuals(mut commands: Commands, q: Query<Entity, With<TileVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_or_empty_images_rejected() {
        assert!(usable_size(UVec2::new(32, 64)));
        assert!(!usable_size(UVec2::new(16385, 1)));
        assert!(!usable_size(UVec2::new(0, 32)));
    }
}

/// A small arrow on the ground at the player's feet, pointing the way they face.
#[derive(Component)]
struct FacingArrow;

fn spawn_arrow(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    // Points north (-Z) before rotation, just in front of the player's square centre.
    let mesh = meshes.add(Triangle3d::new(
        Vec3::new(0.0, 0.0, -0.48),
        Vec3::new(-0.14, 0.0, -0.28),
        Vec3::new(0.14, 0.0, -0.28),
    ));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.2),
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((FacingArrow, Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
}

fn place_arrow(facing: Res<Facing>, rig: Single<&CameraRig>, mut arrow: Single<&mut Transform, With<FacingArrow>>) {
    // Facing 1 = north .. 8 = northwest, clockwise seen from above.
    let angle = -((facing.0.clamp(1, 8) - 1) as f32) * std::f32::consts::FRAC_PI_4;
    arrow.translation = rig.focus + Vec3::Y * 0.03;
    arrow.rotation = Quat::from_rotation_y(angle);
}

fn despawn_arrow(mut commands: Commands, q: Query<Entity, With<FacingArrow>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
