//! The 3D scene: ground tiles, walls, buildings, models, upright picture cards, light,
//! fog, and camera. `assets/models.txt` says which picture becomes which model.
//!
//! 1 tile = 1 world unit. Map x is world +X, map y is world +Z (north is -Z).
//! World tile coords = MapView origin + view coords, so they stay put when the map scrolls.

use std::collections::{HashMap, HashSet};

use bevy::asset::{RecursiveDependencyLoadState, RenderAssetUsages};
use bevy::gltf::GltfAssetLabel;
use bevy::image::{CompressedImageFormats, ImageAddressMode, ImageSampler, ImageSamplerDescriptor, ImageType};
use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::light::NotShadowCaster;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady};
use crossfir3d::buildings::{brick_pixels, doorstep, footprint, parts, wall_parts, wave_pixels, Part, Shape, BRICK_TILE, WAVE_TILE};
use crossfir3d::models::{all_loaded, Load, ModelMap, Visual};
use crossfir3d::races::{classify_player_face, clothes_for_class, look_for_race, Body, Look, PlayerFace};
use crossfir3d::terrain::{ground_height, is_wall};
use crossfir3d::walls::{facing, hangs_on_walls, open_side, wall_beside, wall_masks};

use crossfir3d::clock::{sun_pose, SunPose, FALLBACK_HOUR};
use crossfir3d::settings::Quality;

use crossfir3d::held::{class_look, ClassLook, Garb};
use crate::characters3d::{BeastLook, character_files, class_dressed, ClassGear, outfit_file, spawn_beast, spawn_character, spawn_monster, Character, Outfit, OwnCharacter, OwnLook, PartOf, Placement, StatueFigure};
use crate::graphics::CurrentSettings;
use crate::input::{CameraHeading, CameraView, DoorExit, Facing};
use crossfir3d::camera::{compass, pull_in, sight_blocked, zoom, PIXELS_PER_LINE};
use crate::{AppState, Clock, Faces, LastComc, Session, View, ViewSize};

const TILE_PX: f32 = 32.0;
/// Ground squares are blocks down to here, below the lowest ground (deep sea, -0.4).
const GROUND_BOTTOM: f32 = -1.0;
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
/// Statues: the Modular Dungeon plinth (2.19 tall), its scale, and where its top is.
const PLINTH: &str = "dungeon/Pedestal.gltf";
const PLINTH_SCALE: f32 = 0.3;
const PLINTH_TOP: f32 = 2.19 * PLINTH_SCALE;
const STATUE_STONE: [f32; 3] = [0.7, 0.7, 0.68];
const STONE_SHADE: f32 = 0.35;
/// Map memory: how far from the player squares seen before are still drawn.
const MEMORY_SQUARES: i32 = 40;
/// Street lamps: how bright the lamp box looks, and the light it sheds (lumens, squares).
const LAMP_GLOW: f32 = 4.0;
const LAMP_LUMENS: f32 = 60_000.0;
const LAMP_RANGE: f32 = 5.0;
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
/// Models and `models.txt`: bundled with the program, else the source tree's.
pub fn asset_dir() -> String {
    let dev = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets"));
    crossfir3d::settings::bundled(&std::env::current_exe().unwrap_or_default(), "assets", dev).to_string_lossy().into_owned()
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    /// Registers the world resources and the systems that run while in the game.
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(FOG_COLOR))
            // 4x the M1 value: shadows were almost black (owner, M3 lighting test).
            .insert_resource(GlobalAmbientLight { brightness: 600.0, ..default() })
            .init_resource::<Textures>()
            .init_resource::<Solid>()
            .init_resource::<Doorstep>()
            .init_resource::<MapGen>()
            .init_resource::<DoorExit>()
            .init_resource::<Indoors>()
            .insert_resource(Models::load())
            .add_observer(tint_model)
            .add_systems(Startup, (setup, crate::sky::spawn_sky))
            .add_systems(
                Update,
                (load_textures, sync_visuals, check_models, swap_models, reveal_assembled, hide_entered, glide, follow_player, face_camera, place_arrow, detect_indoors, raise_walls, lighting, crate::sky::update_sky)
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(Update, move_waves.run_if(in_state(AppState::InGame)))
            .add_systems(OnEnter(AppState::InGame), spawn_arrow)
            .add_systems(OnExit(AppState::InGame), (despawn_visuals, despawn_arrow, forget_faces));
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
pub(crate) struct CameraRig {
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
pub(crate) struct Card;

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
    /// Developer check: picture name -> model files shown in a row beside its card.
    compare: HashMap<String, Vec<String>>,
}

impl Models {
    /// Loads models.txt, keeping compared pictures as cards.
    fn load() -> Models {
        let path = format!("{}/models.txt", asset_dir());
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            warn!("{path}: {e}; every picture stays a card");
            String::new()
        });
        let compare = compare_list();
        // Compared pictures stay cards (exact lines first win), with their models beside.
        let cards: String = compare.keys().map(|name| format!("{name} card\n")).collect();
        Models { map: ModelMap::parse(&(cards + &text)), scenes: HashMap::new(), failed: HashSet::new(), compare }
    }

    fn scene(&mut self, server: &AssetServer, file: &str) -> Handle<WorldAsset> {
        let load = || server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{file}")));
        self.scenes.entry(file.to_string()).or_insert_with(load).clone()
    }
}

/// Developer check: CROSSFIR3D_COMPARE=file, lines `picture_name file1 file2 ...`
/// (files under assets/models). Those pictures stay cards, with the models in a row
/// to the right, turned a little so their depth shows.
fn compare_list() -> HashMap<String, Vec<String>> {
    let Ok(path) = std::env::var("CROSSFIR3D_COMPARE") else { return HashMap::new() };
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        warn!("{path}: {e}");
        String::new()
    });
    text.lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            Some((words.next()?.to_string(), words.map(String::from).collect()))
        })
        .collect()
}

const COMPARE_GAP: f32 = 1.0;
const COMPARE_TURN_DEG: f32 = 35.0;

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

/// Counts `newmap`s, so map memory forgets the last map.
#[derive(Resource, Default)]
pub struct MapGen(pub u32);

/// The doorstep the player is shown at now, if they are in a house.
#[derive(Resource, Default)]
struct Doorstep(Option<Vec3>);

/// A thing put together from model files (building, wall, character): its parts and a
/// building's picture are hidden until every file has loaded; then the parts show, or
/// the picture if one failed. Characters and creatures have no picture: the server's
/// pixel art is never drawn for them.
#[derive(Component)]
struct Assembled(Vec<String>);

/// Inside a dungeon, cave or house (most floors in view are indoor floors): a stone
/// ceiling and taller walls instead of the sky.
#[derive(Resource, Default)]
pub(crate) struct Indoors(pub bool);

/// Indoors, stone walls are this many times as tall, up to the ceiling.
pub(crate) const INDOOR_WALLS: f32 = 2.0;

/// A stone wall piece and its outdoor height scale.
#[derive(Component)]
struct StoneWall(f32);

fn detect_indoors(view: Res<View>, faces: Res<Faces>, mut indoors: ResMut<Indoors>) {
    let now = crossfir3d::models::is_indoors(view.0.cells().map(|(_, c)| faces.0.name(c.layers[0]).unwrap_or("")));
    if indoors.0 != now {
        indoors.0 = now;
    }
}

fn raise_walls(indoors: Res<Indoors>, mut walls: Query<(Ref<StoneWall>, &mut Transform)>) {
    for (wall, mut t) in &mut walls {
        if indoors.is_changed() || wall.is_added() {
            t.scale.y = wall.0 * if indoors.0 { INDOOR_WALLS } else { 1.0 };
        }
    }
}

/// The picture standing in for an `Assembled` thing.
#[derive(Component)]
struct StandIn;

/// Colour a model's materials are multiplied by once it is spawned (tint and darkness).
#[derive(Component)]
pub(crate) struct ModelTint(pub Color);

/// A tinted beast: its picture turns grey first, so the tint gives its true colour
/// (a plain multiply turned a red dragon olive under a green tint).
#[derive(Component)]
pub(crate) struct Recolour;

/// Outfits carry bare skin too (parts named "...MI_Regular_..."); those get this colour.
#[derive(Component)]
pub(crate) struct SkinTint(pub Color);

impl Textures {
    /// A face's picture and its size in pixels, once it has arrived.
    pub(crate) fn picture(&self, face: u16) -> Option<(Handle<Image>, Vec2)> {
        self.images.get(&face).map(|(h, size, _)| (h.clone(), *size))
    }
}

/// Face numbers belong to one server and session: the next may reuse a number for
/// another picture.
fn forget_faces(mut textures: ResMut<Textures>) {
    textures.images.clear();
}

#[derive(Resource, Default)]
pub(crate) struct Textures {
    /// Each picture, its size, and its average colour (for the sides of raised squares).
    images: HashMap<u16, (Handle<Image>, Vec2, Color)>,
    ground: Handle<Mesh>,
    cube: Handle<Mesh>,
    card: Handle<Mesh>,
    cylinder: Handle<Mesh>,
    cone: Handle<Mesh>,
    frustum: Handle<Mesh>,
    ball: Handle<Mesh>,
    wedge: Handle<Mesh>,
    bricks: Handle<Image>,
    waves: Handle<Image>,
}

/// Water whose wave picture drifts; how many times the picture repeats across it.
#[derive(Component)]
struct Waves(Vec2);

/// Slides each water's waves slowly along, swaying a little from side to side.
fn move_waves(time: Res<Time>, waves: Query<(&MeshMaterial3d<StandardMaterial>, &Waves)>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let t = time.elapsed_secs();
    let drift = Vec2::new(0.03 * (t * 0.4).sin(), 0.04 * t);
    for (material, Waves(repeats)) in &waves {
        if let Some(mut m) = materials.get_mut(&material.0) {
            m.uv_transform = bevy::math::Affine2::from_scale_angle_translation(*repeats, 0.0, drift);
        }
    }
}

/// A picture that repeats across a face.
pub(crate) fn tiled((w, h, px): (u32, u32, Vec<u8>), sampler: ImageSamplerDescriptor) -> Image {
    let size = Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    let mut image = Image::new(size, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    let repeat = ImageAddressMode::Repeat;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor { address_mode_u: repeat, address_mode_v: repeat, ..sampler });
    image
}

/// Creates the shared meshes and textures and spawns the camera rig.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut textures: ResMut<Textures>,
) {
    textures.bricks = images.add(tiled(brick_pixels(), ImageSamplerDescriptor::nearest()));
    textures.waves = images.add(tiled(wave_pixels(), ImageSamplerDescriptor::linear()));
    textures.ground = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    textures.cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    textures.cylinder = meshes.add(Cylinder::new(1.0, 1.0));
    textures.cone = meshes.add(Cone::new(1.0, 1.0));
    textures.frustum = meshes.add(ConicalFrustum { radius_top: 0.4, radius_bottom: 1.0, height: 1.0 });
    textures.ball = meshes.add(Sphere::new(1.0));
    // Unit wedge: base on y = 0 along x, peak at y = 1, running along z.
    let triangle = Triangle2d::new(Vec2::new(-0.5, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.0, 1.0));
    textures.wedge = meshes.add(Extrusion::new(triangle, 1.0));
    // Unit card with its origin at the bottom middle, so scaling grows it up and sideways.
    textures.card = meshes.add(Rectangle::new(1.0, 1.0).mesh().build().translated_by(Vec3::Y * 0.5));

    // About a metre further back than close-up: the back of the head and shoulders show.
    let distance = 2.3;
    commands.spawn((
        Camera3d::default(),
        camera_transform(Vec3::ZERO, distance, 0.0, PITCH_DEG.to_radians(), Framing { side: SHOULDER, ahead: AHEAD }),
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

mod arrow;
mod light;
mod load;
mod parts;
mod rig;
mod spawn;
mod sync;

pub(crate) use light::{darkness, sun_now};
use {arrow::*, light::lighting, load::*, parts::*, rig::*, spawn::*, sync::*};
#[cfg(test)]
mod tests;
