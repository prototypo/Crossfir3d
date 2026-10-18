//! The 3D scene: ground tiles, walls, buildings, models, upright picture cards, light,
//! fog, and camera. `assets/models.txt` says which picture becomes which model.
//!
//! 1 tile = 1 world unit. Map x is world +X, map y is world +Z (north is -Z).
//! World tile coords = MapView origin + view coords, so they stay put when the map scrolls.

use std::collections::{HashMap, HashSet};

use bevy::asset::{RecursiveDependencyLoadState, RenderAssetUsages};
use bevy::gltf::GltfAssetLabel;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady};
use maccrossfire::buildings::{doorstep, footprint, parts, wall_parts, Part, Shape};
use maccrossfire::models::{all_loaded, Load, ModelMap, Visual};
use maccrossfire::races::{classify_player_face, clothes_for_class, look_for_race, Body, Look, PlayerFace};
use maccrossfire::terrain::{ground_height, is_wall};
use maccrossfire::walls::wall_masks;

use maccrossfire::clock::{sun_pose, FALLBACK_HOUR};
use maccrossfire::settings::Quality;

use crate::characters3d::{character_files, spawn_character, Character, OwnCharacter, OwnLook};
use crate::graphics::CurrentSettings;
use crate::input::{CameraHeading, CameraView, DoorExit, Facing};
use maccrossfire::camera::{compass, pull_in, sight_blocked, zoom, PIXELS_PER_LINE};
use crate::{AppState, Clock, Faces, LastComc, Session, View, ViewSize};

const TILE_PX: f32 = 32.0;
const MAX_FACE_PX: u32 = 2048;
/// Over the shoulder: just above the head, close behind (owner: torso and head in view).
const PITCH_DEG: f32 = 10.0;
const ZOOM_MIN: f32 = 1.0;
const ZOOM_MAX: f32 = 12.0;
/// The camera looks at this point above the player's feet (about shoulder height;
/// the character is about 1.1 tall).
const LOOK_HEIGHT: f32 = 0.95;
/// The camera sits this far to the right of the player and looks this far ahead,
/// so the player stands left of the middle, FPS-like.
const SHOULDER: f32 = 0.35;
const AHEAD: f32 = 4.0;
/// Tab: high above, looking almost straight down (straight down has no "up").
const TOP_PITCH_DEG: f32 = 80.0;
const TOP_DISTANCE: f32 = 14.0;
/// The mouse wheel zooms the top view between these.
const TOP_ZOOM_MIN: f32 = 5.0;
const TOP_ZOOM_MAX: f32 = 30.0;
/// F: in front of the player, looking at their face.
const FACE_PITCH_DEG: f32 = 5.0;
const FACE_DISTANCE: f32 = 1.3;
const SUN_LUX: f32 = 10_000.0;
const MOON_LUX: f32 = 2_500.0;
const FOG_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);
/// Models and `models.txt`. ponytail: the source tree; next to the app when packaged (M7).
pub const ASSET_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(FOG_COLOR))
            // 4x the M1 value: shadows were almost black (owner, M3 lighting test).
            .insert_resource(GlobalAmbientLight { brightness: 600.0, ..default() })
            .init_resource::<Textures>()
            .init_resource::<Solid>()
            .init_resource::<Doorstep>()
            .init_resource::<DoorExit>()
            .insert_resource(Models::load())
            .add_observer(tint_model)
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (load_textures, sync_visuals, check_models, swap_models, reveal_assembled, hide_entered, glide, follow_player, face_camera, place_arrow, lighting)
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
    /// Wall squares: the sides that join a neighbour (it changes the piece).
    join: u8,
}

/// Moves toward `target` over the current step time. Used for creatures and the camera.
#[derive(Component)]
pub(crate) struct Glide {
    pub target: Vec3,
}

#[derive(Component)]
struct CameraRig {
    focus: Vec3,
    distance: f32,
    /// The top view's own zoom.
    top_distance: f32,
    /// Radians clockwise from north (seen from above), eased toward the heading.
    yaw: f32,
    /// Radians down from level, eased toward the view's pitch.
    pitch: f32,
}

/// Flat picture cards, which always turn to face the camera.
#[derive(Component)]
struct Card;

/// Tiles the camera must not sit in: walls and buildings.
#[derive(Resource, Default)]
pub struct Solid(pub HashSet<(i32, i32)>);

#[derive(Component)]
pub struct Sun;

/// The mapping file and one loaded scene per model file.
#[derive(Resource)]
struct Models {
    map: ModelMap,
    scenes: HashMap<String, Handle<WorldAsset>>,
    failed: HashSet<String>,
}

impl Models {
    fn load() -> Models {
        let path = format!("{ASSET_DIR}/models.txt");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            warn!("{path}: {e}; every picture stays a card");
            String::new()
        });
        Models { map: ModelMap::parse(&text), scenes: HashMap::new(), failed: HashSet::new() }
    }

    fn scene(&mut self, server: &AssetServer, file: &str) -> Handle<WorldAsset> {
        let load = || server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{file}")));
        self.scenes.entry(file.to_string()).or_insert_with(load).clone()
    }
}

/// A model waiting to load; its picture card shows until then (and stays if it fails).
#[derive(Component)]
struct PendingModel {
    file: String,
    handle: Handle<WorldAsset>,
    transform: Transform,
    tint: Color,
}

/// The squares a building covers (min x, min y, max x, max y). Buildings on town maps
/// are entrances: the player steps onto them, and then that building is hidden.
#[derive(Component)]
struct Span(i32, i32, i32, i32);

/// A house's doorstep (world position): where the player is shown while the server
/// has them on one of its squares (a shop's entrance row, say).
#[derive(Component)]
struct Door(Vec3);

/// The doorstep the player is shown at now, if they are in a house.
#[derive(Resource, Default)]
struct Doorstep(Option<Vec3>);

/// A thing put together from model files (building, wall, character): its parts are
/// hidden and its picture stands in until every file has loaded.
#[derive(Component)]
struct Assembled(Vec<String>);

/// The picture standing in for an `Assembled` thing.
#[derive(Component)]
struct StandIn;

/// Colour a model's materials are multiplied by once it is spawned (tint and darkness).
#[derive(Component)]
pub(crate) struct ModelTint(pub Color);

/// Outfits carry bare skin too (parts named "...MI_Regular_..."); those get this colour.
#[derive(Component)]
pub(crate) struct SkinTint(pub Color);

#[derive(Resource, Default)]
struct Textures {
    images: HashMap<u16, (Handle<Image>, Vec2)>,
    ground: Handle<Mesh>,
    cube: Handle<Mesh>,
    card: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    wedge: Handle<Mesh>,
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut textures: ResMut<Textures>) {
    textures.ground = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    textures.cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    textures.cylinder = meshes.add(Cylinder::new(1.0, 1.0));
    // Unit wedge: base on y = 0 along x, peak at y = 1, running along z.
    let triangle = Triangle2d::new(Vec2::new(-0.5, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.0, 1.0));
    textures.wedge = meshes.add(Extrusion::new(triangle, 1.0));
    // Unit card with its origin at the bottom middle, so scaling grows it up and sideways.
    textures.card = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));

    // About a metre further back than close-up: the back of the head and shoulders show.
    let distance = 2.3;
    commands.spawn((
        Camera3d::default(),
        camera_transform(Vec3::ZERO, distance, 0.0, PITCH_DEG.to_radians(), SHOULDER, AHEAD),
        CameraRig { focus: Vec3::ZERO, distance, top_distance: TOP_DISTANCE, yaw: 0.0, pitch: PITCH_DEG.to_radians() },
        Glide { target: Vec3::ZERO },
        DistanceFog { color: FOG_COLOR, falloff: FogFalloff::Linear { start: 15.0, end: 25.0 }, ..default() },
    ));
    commands.spawn((
        Sun,
        DirectionalLight { illuminance: SUN_LUX, shadow_maps_enabled: true, ..default() },
        Transform::from_xyz(-4.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// The direction the camera looks along the ground for a yaw (0 = north, -Z).
fn look_dir(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, -yaw.cos())
}

/// Above the player's feet and `side` to the right.
fn shoulder(focus: Vec3, yaw: f32, side: f32) -> Vec3 {
    focus + Vec3::Y * LOOK_HEIGHT + look_dir(yaw + std::f32::consts::FRAC_PI_2) * side
}

fn camera_eye(focus: Vec3, distance: f32, yaw: f32, pitch: f32, side: f32) -> Vec3 {
    shoulder(focus, yaw, side) - look_dir(yaw) * pitch.cos() * distance + Vec3::Y * pitch.sin() * distance
}

/// Looks `ahead` past the shoulder point.
fn camera_transform(focus: Vec3, distance: f32, yaw: f32, pitch: f32, side: f32, ahead: f32) -> Transform {
    let target = shoulder(focus, yaw, side) + look_dir(yaw) * ahead;
    Transform::from_translation(camera_eye(focus, distance, yaw, pitch, side)).looking_at(target, Vec3::Y)
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

/// What should be on screen, keyed by (world x, world y, layer): face, darkness, join.
type Wanted = HashMap<(i32, i32, u8), (u16, u8, u8)>;

fn wanted(view: &View, faces: &Faces, textures: &Textures) -> Wanted {
    let mut out = Wanted::new();
    for ((vx, vy), cell) in view.0.cells() {
        let (wx, wy) = view.0.world(vx, vy);
        for (layer, raw) in cell.layers.iter().enumerate() {
            let face = faces.0.resolve(*raw);
            if face != 0 && textures.images.contains_key(&face) {
                out.insert((wx, wy, layer as u8), (face, cell.darkness, 0));
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
    server: Res<AssetServer>,
    mut models: ResMut<Models>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing: Query<(Entity, &mut TileVisual, Option<&mut Glide>, Has<Character>)>,
    mut solid: ResMut<Solid>,
    (view_size, own, mut images): (Res<ViewSize>, Res<OwnLook>, ResMut<Assets<Image>>),
) {
    if !view.is_changed() && !textures.is_changed() {
        return;
    }
    // ponytail: full diff per change; incremental dirty cells if profiling says so.
    let mut want = wanted(&view, &faces, &textures);
    let name_of = |face: u16| faces.0.name(face).unwrap_or("");
    let walls: HashSet<(i32, i32)> = want
        .iter()
        .filter(|(_, (f, _, _))| is_wall(name_of(*f)) || matches!(models.map.lookup(name_of(*f)), Visual::Wall { .. }))
        .map(|((x, y, _), _)| (*x, *y))
        .collect();
    let player = view.0.world((view_size.0 / 2) as i8, (view_size.1 / 2) as i8);
    let joins = wall_masks(&walls);
    solid.0 = walls;
    for ((x, y, _), (face, _, join)) in want.iter_mut() {
        if let Some(m) = joins.get(&(*x, *y)) {
            *join = *m;
        }
        if let Visual::Building { .. } = models.map.lookup(name_of(*face)) {
            let (w, h) = textures.images.get(face).map_or((1, 1), |(_, s)| footprint(s.x as u32, s.y as u32));
            let squares: Vec<(i32, i32)> = (0..h as i32).flat_map(|dy| (0..w as i32).map(move |dx| (*x - dx, *y - dy))).collect();
            // The building the player stands in is hidden (`hide_entered`), so it never blocks.
            if !squares.contains(&player) {
                solid.0.extend(squares);
            }
        }
    }
    let floor_height = |x: i32, y: i32, want: &Wanted| {
        want.get(&(x, y, 0)).and_then(|(f, _, _)| faces.0.name(*f)).map_or(0.0, ground_height)
    };
    let heights: HashMap<(i32, i32), f32> =
        want.keys().map(|&(x, y, _)| ((x, y), floor_height(x, y, &want))).collect();

    // Keep what is unchanged. Gliding things that left their tile become orphans.
    let mut orphans = Vec::new();
    for (entity, tv, _, _) in &existing {
        match want.get(&(tv.x, tv.y, tv.layer)) {
            Some(&(face, dark, join)) if face == tv.face && dark == tv.darkness && join == tv.join => {
                want.remove(&(tv.x, tv.y, tv.layer));
            }
            _ if glides(tv.layer) => orphans.push(entity),
            _ => commands.entity(entity).despawn(),
        }
    }

    for ((x, y, layer), (face, darkness, join)) in want {
        let Some((image, size)) = textures.images.get(&face) else { continue };
        let name = name_of(face);
        let h = heights.get(&(x, y)).copied().unwrap_or(0.0);
        let shade = darkness as f32 / 255.0;
        let tile = Vec3::new(x as f32, h, y as f32);

        // A creature that stepped one tile: move the old entity instead of making a new one.
        if glides(layer) {
            let found = orphans.iter().position(|&e| {
                existing.get(e).is_ok_and(|(_, tv, _, _)| {
                    tv.face == face && tv.layer == layer && (tv.x - x).abs() <= 1 && (tv.y - y).abs() <= 1
                })
            });
            if let Some(i) = found {
                let e = orphans.swap_remove(i);
                let (_, mut tv, glide, character) = existing.get_mut(e).unwrap();
                (tv.x, tv.y, tv.darkness) = (x, y, darkness);
                if let Some(mut g) = glide {
                    g.target = if character { tile + Vec3::new(0.5, 0.0, 0.5) } else { card_position(tile, *size, layer) };
                }
                continue;
            }
        }

        let tv = TileVisual { x, y, layer, face, darkness, join };
        let wall = is_wall(name);
        let flat = matches!(models.map.lookup(name), Visual::Flat);
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
        // The picture (a block for walls, else a card) stands in while a 3D thing loads,
        // and stays if it fails: never nothing.
        let picture = if wall {
            Transform::from_translation(tile + Vec3::new(0.5, 0.5, 0.5))
        } else {
            Transform::from_translation(card_position(tile, *size, layer))
                .with_scale(Vec3::new(size.x / TILE_PX, size.y / TILE_PX, 1.0))
        };
        let stand_in = |commands: &mut Commands, parent: Entity, at: Vec3| {
            let t = Transform { translation: picture.translation - at, ..picture };
            let mesh = if wall { textures.cube.clone() } else { textures.card.clone() };
            let mut e = commands.spawn((StandIn, Mesh3d(mesh), MeshMaterial3d(material.clone()), t, ChildOf(parent)));
            if !wall {
                e.insert(Card);
            }
        };

        // Players (a race or class picture) are 3D characters; the own one in the middle.
        if glides(layer) && let Some(kind) = classify_player_face(name) {
            let own_tile = (x, y) == player;
            let (look, body) = if own_tile { (own.look, own.body) } else { (other_look(kind), Body::Regular) };
            let files = character_files(body);
            if !files.iter().any(|f| models.failed.contains(*f)) {
                let pos = tile + Vec3::new(0.5, 0.0, 0.5);
                let load = |f: &str| models.scene(&server, f);
                let at = Transform::from_translation(pos);
                let e = spawn_character(&mut commands, load, files, &look, at, Visibility::Hidden, (tv, Glide { target: pos }));
                commands.entity(e).insert(Assembled(files.iter().map(|f| f.to_string()).collect()));
                if own_tile {
                    commands.entity(e).insert(OwnCharacter);
                }
                stand_in(&mut commands, e, pos);
                continue;
            }
        }
        // Big pictures are sent at their bottom-right square; models stand in the middle.
        let (fw, fh) = footprint(size.x as u32, size.y as u32);
        let middle = tile + Vec3::new(1.0 - fw as f32 / 2.0, 0.0, 1.0 - fh as f32 / 2.0);
        let (built, sign) = match models.map.lookup(name) {
            Visual::Wall { style } => (Some(wall_parts(style, join)).filter(|p| !p.is_empty()), None),
            Visual::Building { recipe, sign, icon } => {
                let sign = sign.as_deref().map(|t| sign_material(t, icon.as_deref(), shade, &mut images, &mut materials));
                (parts(recipe, fw, fh), sign)
            }
            _ => (None, None),
        };
        if let Some(parts) = built {
            let at = if layer == 0 || wall { tile + Vec3::new(0.5, 0.0, 0.5) } else { middle };
            let span = Span(x + 1 - fw as i32, y + 1 - fh as i32, x, y);
            let files: Vec<String> =
                parts.iter().filter_map(|p| if let Shape::Model(f) = p.shape { Some(f.to_string()) } else { None }).collect();
            let parent = commands.spawn((tv, span, Assembled(files), Transform::from_translation(at), Visibility::default())).id();
            if let Visual::Building { recipe, .. } = models.map.lookup(name)
                && let Some(step) = doorstep(recipe, fw, fh)
            {
                commands.entity(parent).insert(Door(at + Vec3::from(step)));
            }
            for p in &parts {
                let child = spawn_part(&mut commands, p, shade, &textures, sign.as_ref(), &server, &mut models, &mut materials);
                commands.entity(parent).add_child(child);
            }
            stand_in(&mut commands, parent, at);
            continue;
        }

        let model = match models.map.lookup(name) {
            Visual::Model { file, scale, rot_deg, dy, tint } if !models.failed.contains(file) => {
                let t = Transform::from_xyz(0.0, *dy, 0.0)
                    .with_rotation(Quat::from_rotation_y(-rot_deg.to_radians()))
                    .with_scale(Vec3::splat(*scale));
                let [r, g, b] = tint.unwrap_or([1.0; 3]);
                Some((file.clone(), t, Color::srgb(r * shade, g * shade, b * shade)))
            }
            _ => None,
        };
        if wall {
            let t = Transform::from_translation(tile + Vec3::new(0.5, 0.5, 0.5));
            commands.spawn((tv, Mesh3d(textures.cube.clone()), MeshMaterial3d(material), t));
        } else if layer == 0 || flat {
            // Flat decorations lie just above the ground, as big as their picture.
            let (pos, scale) = if flat {
                (middle + Vec3::Y * (0.01 + layer as f32 * 0.001), Vec3::new(fw as f32, 1.0, fh as f32))
            } else {
                (tile + Vec3::new(0.5, 0.0, 0.5), Vec3::ONE)
            };
            let t = Transform::from_translation(pos).with_scale(scale);
            let mut e = commands.spawn((tv, Mesh3d(textures.ground.clone()), MeshMaterial3d(material), t));
            // Trees and rocks on ground squares stand on the ground tile.
            if let (Some((file, t, tint)), false) = (model, flat) {
                let handle = models.scene(&server, &file);
                e.with_child((WorldAssetRoot(handle), t, ModelTint(tint)));
            }
        } else {
            let pos = card_position(tile, *size, layer);
            let t = Transform::from_translation(pos).with_scale(Vec3::new(size.x / TILE_PX, size.y / TILE_PX, 1.0));
            let mut e = commands.spawn((tv, Card, Mesh3d(textures.card.clone()), MeshMaterial3d(material), t));
            if glides(layer) {
                e.insert(Glide { target: pos });
            } else if let Some((file, t, tint)) = model {
                let handle = models.scene(&server, &file);
                let transform = Transform { translation: middle + t.translation, ..t };
                e.insert(PendingModel { file, handle, transform, tint });
            }
        }
    }
    for e in orphans {
        commands.entity(e).despawn();
    }
}

/// Another player's look: their race if the picture shows it, else a human in their
/// class colours.
fn other_look(kind: PlayerFace) -> Look {
    match kind {
        PlayerFace::Race(race) => look_for_race(race),
        PlayerFace::Class(class) => Look { clothes: clothes_for_class(class), ..look_for_race("human") },
    }
}

/// Pixel text (and icon) for a building's sign: its material and its width / height.
fn sign_material(
    text: &str,
    icon: Option<&str>,
    shade: f32,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<StandardMaterial>, f32) {
    let (w, h, px) = maccrossfire::signs::sign_pixels(text, icon);
    let size = Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    let mut image = Image::new(size, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    image.sampler = ImageSampler::nearest();
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(shade, shade, shade),
        base_color_texture: Some(images.add(image)),
        alpha_mode: AlphaMode::Mask(0.5),
        perceptual_roughness: 0.9,
        ..default()
    });
    (material, w as f32 / h as f32)
}

/// One part of a building or wall, as a child entity.
#[allow(clippy::too_many_arguments)]
fn spawn_part(
    commands: &mut Commands,
    p: &Part,
    shade: f32,
    textures: &Textures,
    sign: Option<&(Handle<StandardMaterial>, f32)>,
    server: &AssetServer,
    models: &mut Models,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    let t = Transform::from_translation(Vec3::from(p.pos))
        .with_rotation(Quat::from_rotation_y(-p.yaw_deg.to_radians()))
        .with_scale(Vec3::from(p.scale));
    let [r, g, b] = p.color.unwrap_or([1.0; 3]);
    let color = Color::srgb(r * shade, g * shade, b * shade);
    let (mesh, size) = match p.shape {
        Shape::Model(file) => {
            let handle = models.scene(server, file);
            return commands.spawn((WorldAssetRoot(handle), t, ModelTint(color), Visibility::Hidden)).id();
        }
        Shape::Box(size) => (textures.cube.clone(), Vec3::from(size)),
        Shape::Cylinder { radius, height } => (textures.cylinder.clone(), Vec3::new(radius, height, radius)),
        Shape::Wedge(size) => (textures.wedge.clone(), Vec3::from(size)),
        Shape::Sign([w, h]) => {
            let Some((material, aspect)) = sign else { return commands.spawn((t, Visibility::Hidden)).id() };
            // Fit the text inside the board, keeping its shape, centred top to bottom.
            let (sw, sh) = if w / h > *aspect { (h * aspect, h) } else { (w, w / aspect) };
            let t = Transform { translation: t.translation + Vec3::Y * (h - sh) / 2.0, scale: t.scale * Vec3::new(sw, sh, 1.0), ..t };
            return commands.spawn((Mesh3d(textures.card.clone()), MeshMaterial3d(material.clone()), t, Visibility::Hidden)).id();
        }
    };
    let material = materials.add(StandardMaterial { base_color: color, perceptual_roughness: 0.9, ..default() });
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform { scale: t.scale * size, ..t }, Visibility::Hidden)).id()
}

/// Logs each model file that fails to load, once.
fn check_models(server: Res<AssetServer>, mut models: ResMut<Models>) {
    let Models { scenes, failed, .. } = &mut *models;
    for (file, handle) in scenes.iter() {
        if failed.contains(file) {
            continue;
        }
        if let RecursiveDependencyLoadState::Failed(e) = server.recursive_dependency_load_state(handle) {
            warn!("model {file}: {e}; showing the picture instead");
            failed.insert(file.clone());
        }
    }
}

impl Models {
    fn load_state(&self, server: &AssetServer, file: &str) -> Load {
        match self.scenes.get(file) {
            _ if self.failed.contains(file) => Load::Failed,
            Some(h) if server.is_loaded_with_dependencies(h) => Load::Loaded,
            _ => Load::Loading,
        }
    }
}

/// Shows an assembled thing once all its files have loaded (its picture goes); if one
/// fails, its parts go and the picture stays.
fn reveal_assembled(
    mut commands: Commands,
    server: Res<AssetServer>,
    models: Res<Models>,
    things: Query<(Entity, &Assembled, &Children)>,
    stand_ins: Query<(), With<StandIn>>,
) {
    for (e, Assembled(files), children) in &things {
        let state = all_loaded(files.iter().map(|f| models.load_state(&server, f)));
        if state == Load::Loading {
            continue;
        }
        for &c in children {
            match (state, stand_ins.contains(c)) {
                (Load::Loaded, true) | (Load::Failed, false) => commands.entity(c).try_despawn(),
                (Load::Loaded, false) => {
                    commands.entity(c).try_insert(Visibility::Inherited);
                }
                _ => {}
            }
        }
        commands.entity(e).try_remove::<Assembled>();
    }
}

/// A loaded model takes the place of its picture card; a failed one leaves the card.
fn swap_models(
    mut commands: Commands,
    server: Res<AssetServer>,
    models: Res<Models>,
    pending: Query<(Entity, &PendingModel)>,
) {
    for (e, p) in &pending {
        if models.failed.contains(&p.file) {
            commands.entity(e).try_remove::<PendingModel>();
        } else if server.is_loaded_with_dependencies(&p.handle) {
            commands
                .entity(e)
                .try_remove::<(PendingModel, Card, Mesh3d, MeshMaterial3d<StandardMaterial>)>()
                .try_insert((WorldAssetRoot(p.handle.clone()), p.transform, ModelTint(p.tint)));
        }
    }
}

/// Multiplies a spawned model's materials by its tint (each instance gets its own copy).
/// ponytail: one material copy per instance; share copies per (material, tint) if draw calls hurt.
fn tint_model(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    tints: Query<(&ModelTint, Option<&SkinTint>)>,
    names: Query<&Name>,
    children: Query<&Children>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((ModelTint(tint), skin)) = tints.get(ready.entity) else { return };
    if *tint == Color::WHITE && skin.is_none() {
        return;
    }
    for d in children.iter_descendants(ready.entity) {
        let Ok(m) = meshes.get(d) else { continue };
        let is_skin = names.get(d).is_ok_and(|n| n.as_str().contains("MI_Regular"));
        let tint = match skin {
            Some(SkinTint(s)) if is_skin => s.to_linear(),
            _ => tint.to_linear(),
        };
        let Some(mut copy) = materials.get(&m.0).cloned() else { continue };
        let c = copy.base_color.to_linear();
        copy.base_color = LinearRgba::new(c.red * tint.red, c.green * tint.green, c.blue * tint.blue, c.alpha).into();
        commands.entity(d).try_insert(MeshMaterial3d(materials.add(copy)));
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

#[allow(clippy::too_many_arguments)]
fn follow_player(
    time: Res<Time>,
    comc: Res<LastComc>,
    view: Res<View>,
    size: Res<ViewSize>,
    heading: Res<CameraHeading>,
    cam: Res<CameraView>,
    solid: Res<Solid>,
    scroll: Res<AccumulatedMouseScroll>,
    mut pinch: MessageReader<PinchGesture>,
    doorstep: Res<Doorstep>,
    rig: Single<(&mut Transform, &mut CameraRig, &mut DistanceFog)>,
) {
    let (mut t, mut rig, mut fog) = rig.into_inner();
    // A trackpad's two-finger scroll comes in pixels, a mouse wheel in lines.
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
    };
    let pinched: f32 = pinch.read().map(|p| p.0).sum();
    if cam.top {
        rig.top_distance = zoom(rig.top_distance, lines, pinched, TOP_ZOOM_MIN, TOP_ZOOM_MAX);
    } else {
        rig.distance = zoom(rig.distance, lines, pinched, ZOOM_MIN, ZOOM_MAX);
    }
    let (px, py) = view.0.world((size.0 / 2) as i8, (size.1 / 2) as i8);
    let target = doorstep.0.map_or(Vec3::new(px as f32 + 0.5, 0.0, py as f32 + 0.5), |d| d.with_y(0.0));
    let step_secs = comc.time_ms.clamp(100, 400) as f32 / 1000.0;
    let d = target - rig.focus;
    rig.focus = if d.length() > 2.0 { target } else { rig.focus + d.clamp_length_max(time.delta_secs() / step_secs) };
    let behind = heading.0 as f32 * std::f32::consts::FRAC_PI_4;
    let (goal, pitch, want, side, ahead) = if cam.top {
        (0.0, TOP_PITCH_DEG, rig.top_distance, 0.0, 0.0)
    } else if cam.face {
        (behind + std::f32::consts::PI, FACE_PITCH_DEG, FACE_DISTANCE, 0.0, 0.0)
    } else {
        (behind, PITCH_DEG, rig.distance, SHOULDER, AHEAD)
    };
    // Ease the yaw the short way round toward the goal (about 0.4 s per 45°).
    let diff = (goal - rig.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    rig.yaw += diff.clamp(-1.0, 1.0) * (time.delta_secs() * 4.0).min(1.0);
    // Pull in while a wall or building stands between player and camera; never rise
    // above them (that looked into upper floors).
    let (focus, yaw) = (rig.focus, rig.yaw);
    let blocked = |dist: f32| {
        let eye = camera_eye(focus, dist, yaw, pitch.to_radians(), side);
        sight_blocked((focus.x, focus.z), (eye.x, eye.z), |x, y| solid.0.contains(&(x, y)))
    };
    let distance = if cam.top { want } else { pull_in(want, blocked) };
    let step = pitch.to_radians() - rig.pitch;
    rig.pitch += step * (time.delta_secs() * 4.0).min(1.0);
    *t = camera_transform(rig.focus, distance, rig.yaw, rig.pitch, side, ahead);
    // Hide the edge of what the server sends: fog from 40% of the view radius to the edge.
    let radius = size.0.min(size.1) as f32 / 2.0;
    fog.falloff = FogFalloff::Linear { start: distance + radius * 0.4, end: distance + radius };
}

/// The player on a house's squares stands on its doorstep, outside, with the house in
/// view. Anything else they stand in (a well, a castle) is hidden instead.
fn hide_entered(
    view: Res<View>,
    size: Res<ViewSize>,
    mut step: ResMut<Doorstep>,
    mut exit: ResMut<DoorExit>,
    mut q: Query<(&Span, Option<&Door>, &mut Visibility)>,
    mut own: Query<&mut Glide, With<OwnCharacter>>,
) {
    let (px, py) = view.0.world((size.0 / 2) as i8, (size.1 / 2) as i8);
    step.0 = None;
    for (Span(x0, y0, x1, y1), door, mut vis) in &mut q {
        let inside = (*x0..=*x1).contains(&px) && (*y0..=*y1).contains(&py);
        if let (true, Some(Door(at))) = (inside, door) {
            step.0 = Some(*at);
        }
        vis.set_if_neq(if inside && door.is_none() { Visibility::Hidden } else { Visibility::Inherited });
    }
    // The square in front of the door, one step out of the doorway.
    exit.came_out = step.0.is_some() && exit.new_map;
    exit.out = step.0.map(|at| {
        let d = (at.x.floor() as i32 - px, (at.z + 0.25).floor() as i32 - py);
        compass((d.0.signum() as i8, d.1.signum() as i8), 0)
    });
    // Moving off the house gives the character a new tile target (`sync_visuals`).
    if let (Some(at), Ok(mut glide)) = (step.0, own.single_mut()) {
        glide.target = at;
    }
}

/// Cards turn about the vertical axis to face the camera.
fn face_camera(rig: Single<&CameraRig>, mut cards: Query<&mut Transform, With<Card>>) {
    let rotation = Quat::from_rotation_y(-rig.yaw);
    for mut t in &mut cards {
        t.rotation = rotation;
    }
}

/// The game clock sets where the light comes from and its colour; its strength follows
/// the darkness of the player's square (the server darkens by hour, season, indoors).
fn lighting(
    view: Res<View>,
    size: Res<ViewSize>,
    clock: Res<Clock>,
    settings: Res<CurrentSettings>,
    sun: Single<(&mut DirectionalLight, &mut Transform), With<Sun>>,
) {
    let (mut light, mut transform) = sun.into_inner();
    let dark = view.0.cell((size.0 / 2) as i8, (size.1 / 2) as i8).map_or(255, |c| c.darkness);
    let forced = std::env::var("MACCROSSFIRE_HOUR").ok().and_then(|h| h.parse().ok()); // screenshots
    let pose = sun_pose(forced.or(clock.0.hour()).unwrap_or(FALLBACK_HOUR));
    // azimuth 0 = east (+X), PI/2 = south (+Z), PI = west (-X)
    let dir = Vec3::new(
        pose.azimuth.cos() * pose.elevation.cos(),
        pose.elevation.sin(),
        pose.azimuth.sin() * pose.elevation.cos(),
    );
    *transform = Transform::from_translation(dir * 20.0).looking_at(Vec3::ZERO, Vec3::Y);
    let (color, strength) = if pose.moon {
        (Color::srgb(0.6, 0.7, 1.0), MOON_LUX)
    } else if pose.elevation < 15f32.to_radians() {
        (Color::srgb(1.0, 0.72, 0.45), SUN_LUX)
    } else {
        (Color::srgb(1.0, 0.97, 0.9), SUN_LUX)
    };
    light.color = color;
    light.illuminance = strength * dark as f32 / 255.0;
    light.shadow_maps_enabled = !pose.moon && settings.0.shadows != Quality::Off;
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
