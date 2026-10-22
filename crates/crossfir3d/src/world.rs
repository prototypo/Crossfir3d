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
use crossfir3d::walls::{facing, hangs_on_walls, open_side, wall_masks};

use crossfir3d::clock::{sun_pose, SunPose, FALLBACK_HOUR};
use crossfir3d::settings::Quality;

use crossfir3d::held::{class_look, ClassLook, Garb};
use crate::characters3d::{character_files, class_dressed, ClassGear, outfit_file, spawn_beast, spawn_character, spawn_monster, Character, Outfit, OwnCharacter, OwnLook, PartOf, StatueFigure};
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

/// The average colour of the opaque pixels of RGBA8 (sRGB) data; grey when none are.
fn mean_colour(rgba: &[u8]) -> Color {
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for px in rgba.as_chunks::<4>().0.iter().filter(|px| px[3] >= 128) {
        for i in 0..3 {
            sum[i] += px[i] as u64;
        }
        n += 1;
    }
    if n == 0 {
        return Color::srgb(0.5, 0.5, 0.5);
    }
    let c = |i: usize| sum[i] as f32 / n as f32 / 255.0;
    Color::srgb(c(0), c(1), c(2))
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
                let mean = mean_colour(image.data.as_deref().unwrap_or_default());
                textures.images.insert(num, (images.add(image), size, mean));
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

/// Where a gliding thing with a 3D model (a flying arrow) sits on square `tile`.
fn model_spot(visual: &Visual, tile: Vec3) -> Option<Vec3> {
    match visual {
        Visual::Model { dy, .. } => Some(tile + Vec3::new(0.5, *dy, 0.5)),
        _ => None,
    }
}

/// What `sync_visuals` draws with: the command queue, the loaded models, and the asset stores.
#[derive(SystemParam)]
struct Draw<'w, 's> {
    commands: Commands<'w, 's>,
    server: Res<'w, AssetServer>,
    models: ResMut<'w, Models>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    textures: Res<'w, Textures>,
}

/// What `sync_visuals` reads: the server's view, the face names, and the own character.
#[derive(SystemParam)]
struct Inputs<'w> {
    view: Res<'w, View>,
    faces: Res<'w, Faces>,
    view_size: Res<'w, ViewSize>,
    own: Res<'w, OwnLook>,
    map_gen: Res<'w, MapGen>,
}

type Existing<'w, 's> = Query<'w, 's, (Entity, &'static mut TileVisual, Option<&'static mut Glide>, Has<Character>)>;

/// What the whole view says about each square: walls, ground heights, the player's square.
struct Frame {
    walls: HashSet<(i32, i32)>,
    heights: HashMap<(i32, i32), f32>,
    player: (i32, i32),
}

/// One square's thing to draw, with everything the spawning helpers need to know about it.
struct Square<'a> {
    x: i32,
    y: i32,
    layer: u8,
    face: u16,
    darkness: u8,
    join: u8,
    name: &'a str,
    h: f32,
    shade: f32,
    /// The square's corner at ground height, moved to the wall's open side for hung things.
    tile: Vec3,
    turn: Quat,
    image: Handle<Image>,
    size: Vec2,
    mean: Color,
    wall: bool,
    own_tile: bool,
    /// Big pictures are sent at their bottom-right square; models stand in the middle.
    fw: u32,
    fh: u32,
    middle: Vec3,
}

impl<'a> Square<'a> {
    /// Gathers one square's tile, picture and placement from the server data and the frame.
    fn new(key:(i32, i32, u8), val: (u16, u8, u8), name: &'a str, shown: (Handle<Image>, Vec2, Color), frame: &Frame) -> Self {
        let ((x, y, layer), (face, darkness, join), (image, size, mean)) = (key, val, shown);
        let h = frame.heights.get(&(x, y)).copied().unwrap_or(0.0);
        let tile = Vec3::new(x as f32, h, y as f32);
        // A brazier or torch on a wall square stands against the wall's open side.
        // It turns to face the room (a clock's dial; torches look the same either way).
        let hung = open_side(&frame.walls, x, y).filter(|_| !is_wall(name) && hangs_on_walls(name));
        let tile = match hung {
            Some((dx, dy)) => tile + Vec3::new(dx as f32, 0.0, dy as f32) * 0.75,
            None => tile,
        };
        let (fw, fh) = footprint(size.x as u32, size.y as u32);
        Square {
            x,
            y,
            layer,
            face,
            darkness,
            join,
            name,
            h,
            shade: darkness as f32 / 255.0,
            tile,
            turn: Quat::from_rotation_y(hung.map_or(0.0, facing)),
            image,
            size,
            mean,
            wall: is_wall(name),
            // Players (a race or class picture) are 3D characters; the own one in the middle.
            own_tile: (x, y) == frame.player && classify_player_face(name).is_some(),
            fw,
            fh,
            middle: tile + Vec3::new(1.0 - fw as f32 / 2.0, 0.0, 1.0 - fh as f32 / 2.0),
        }
    }

    /// The square's tile record, used to tell whether it changed.
    fn tv(&self) -> TileVisual {
        TileVisual { x: self.x, y: self.y, layer: self.layer, face: self.face, darkness: self.darkness, join: self.join }
    }

    /// The middle of the square, where 3D characters stand.
    fn centre(&self) -> Vec3 {
        self.tile + Vec3::new(0.5, 0.0, 0.5)
    }
}

/// The name of a picture, or an empty name if it is not known.
fn name_of(faces: &Faces, face: u16) -> &str {
    faces.0.name(face).unwrap_or("")
}

/// The map square the player stands on, at the middle of the view.
fn player_square(input: &Inputs) -> (i32, i32) {
    input.view.0.world((input.view_size.0 / 2) as i8, (input.view_size.1 / 2) as i8)
}

/// The squares with a wall picture or a wall model.
fn wall_set(want: &Wanted, faces: &Faces, models: &Models) -> HashSet<(i32, i32)> {
    let is_wall_face = |f: &u16| is_wall(name_of(faces, *f)) || matches!(models.map.lookup(name_of(faces, *f)), Visual::Wall { .. });
    want.iter().filter(|(_, (f, _, _))| is_wall_face(f)).map(|((x, y, _), _)| (*x, *y)).collect()
}

/// The squares a building picture covers: from its bottom-right square up and left.
fn covered((x, y): (i32, i32), (w, h): (u32, u32)) -> Vec<(i32, i32)> {
    (0..h as i32).flat_map(|dy| (0..w as i32).map(move |dx| (x - dx, y - dy))).collect()
}

/// Gives wall squares their joins, and marks the squares buildings block as solid.
fn mark_solid(want: &mut Wanted, joins: &HashMap<(i32, i32), u8>, input: &Inputs, draw: &Draw, solid: &mut HashSet<(i32, i32)>) {
    let player = player_square(input);
    for ((x, y, layer), (face, _, join)) in want.iter_mut() {
        if let Some(m) = joins.get(&(*x, *y)) {
            *join = *m;
        }
        if let Visual::Building { recipe, .. } = draw.models.map.lookup(name_of(&input.faces, *face))
            && !lava_floor(recipe, *layer)
        {
            let size = draw.textures.images.get(face).map_or((1, 1), |(_, s, _)| footprint(s.x as u32, s.y as u32));
            let squares = covered((*x, *y), size);
            // The building the player stands in is hidden (`hide_entered`), so it never blocks.
            if !squares.contains(&player) {
                solid.extend(squares);
            }
        }
    }
}

/// How high each wanted square's ground is (its layer 0 picture's height).
fn ground_heights(want: &Wanted, faces: &Faces) -> HashMap<(i32, i32), f32> {
    let floor_height = |x: i32, y: i32| want.get(&(x, y, 0)).and_then(|(f, _, _)| faces.0.name(*f)).map_or(0.0, ground_height);
    want.keys().map(|&(x, y, _)| ((x, y), floor_height(x, y))).collect()
}

/// What to show now and the facts about the view; also sets which squares are solid.
fn plan(input: &Inputs, draw: &Draw, solid: &mut Solid) -> (Wanted, Frame) {
    // ponytail: full diff per change; incremental dirty cells if profiling says so.
    let mut want = wanted(&input.view, &input.faces, &draw.textures);
    let walls = wall_set(&want, &input.faces, &draw.models);
    let joins = wall_masks(&walls);
    solid.0 = walls.clone();
    mark_solid(&mut want, &joins, input, draw, &mut solid.0);
    let heights = ground_heights(&want, &input.faces);
    (want, Frame { walls, heights, player: player_square(input) })
}

/// Map memory: still things outside the server's view stay as last seen, up to
/// MEMORY_SQUARES away.
fn remembered(input: &Inputs, player: (i32, i32), new_map: bool, x: i32, y: i32) -> bool {
    let (x0, y0) = input.view.0.world(0, 0);
    let in_view = (x0..x0 + input.view_size.0 as i32).contains(&x) && (y0..y0 + input.view_size.1 as i32).contains(&y);
    !new_map && !in_view && (x - player.0).abs().max((y - player.1).abs()) <= MEMORY_SQUARES
}

/// Whether an existing entity already shows what is wanted: same face, darkness and join.
fn shows(tv: &TileVisual, (face, dark, join): (u16, u8, u8)) -> bool {
    face == tv.face && dark == tv.darkness && join == tv.join
}

/// Keeps what is unchanged (dropping it from `want`) and despawns what is gone.
/// Gliding things that left their tile are returned as orphans.
fn sweep(existing: &Existing<'_, '_>, want: &mut Wanted, commands: &mut Commands, memory: impl Fn(i32, i32) -> bool) -> Vec<Entity> {
    let mut orphans = Vec::new();
    for (entity, tv, _, _) in existing {
        match want.get(&(tv.x, tv.y, tv.layer)) {
            Some(&entry) if shows(tv, entry) => {
                want.remove(&(tv.x, tv.y, tv.layer));
            }
            _ if glides(tv.layer) => orphans.push(entity),
            None if memory(tv.x, tv.y) => {}
            _ => commands.entity(entity).despawn(),
        }
    }
    orphans
}

/// The orphan that is this square's creature one tile away (or its other frame).
/// A 3D character keeps its body when only its picture changes (each turn and step
/// frame), so it is never rebuilt, nor briefly built as someone else.
fn find_orphan(sq: &Square, orphans: &[Entity], existing: &Existing<'_, '_>, models: &Models, faces: &Faces) -> Option<usize> {
    let key = body_key(&models.map, sq.name);
    orphans.iter().position(|&e| {
        existing.get(e).is_ok_and(|(_, tv, _, character)| {
            let same_body = character && key.is_some() && body_key(&models.map, name_of(faces, tv.face)) == key;
            (tv.face == sq.face || same_body) && tv.layer == sq.layer && (tv.x - sq.x).abs() <= 1 && (tv.y - sq.y).abs() <= 1
        })
    })
}

/// Moves an old entity to this square and points its glide at the new spot.
fn retarget(sq: &Square, e: Entity, existing: &mut Existing<'_, '_>, models: &Models) {
    let (_, mut tv, glide, character) = existing.get_mut(e).unwrap();
    (tv.x, tv.y, tv.darkness, tv.face) = (sq.x, sq.y, sq.darkness, sq.face);
    if let Some(mut g) = glide {
        // 3D characters and built creatures (bees) stand in the middle of the square.
        let built = matches!(models.map.lookup(sq.name), Visual::Building { .. });
        g.target = if character || built {
            sq.centre()
        } else {
            model_spot(models.map.lookup(sq.name), sq.tile).unwrap_or_else(|| card_position(sq.tile, sq.size, sq.layer))
        };
    }
}

/// A creature that stepped one tile: moves the old entity instead of making a new one.
fn reuse_orphan(sq: &Square, orphans: &mut Vec<Entity>, existing: &mut Existing<'_, '_>, models: &Models, faces: &Faces) -> bool {
    if !glides(sq.layer) {
        return false;
    }
    let Some(i) = find_orphan(sq, orphans, existing, models, faces) else { return false };
    let e = orphans.swap_remove(i);
    retarget(sq, e, existing, models);
    true
}

/// The square's picture as a material, dimmed by the darkness.
fn square_material(sq: &Square, draw: &mut Draw) -> Handle<StandardMaterial> {
    draw.materials.add(StandardMaterial {
        base_color: Color::srgb(sq.shade, sq.shade, sq.shade),
        base_color_texture: Some(sq.image.clone()),
        // Wall art has see-through parts; solid blocks must not show them as holes.
        alpha_mode: if sq.wall { AlphaMode::Opaque } else { AlphaMode::Mask(0.5) },
        perceptual_roughness: 0.9,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}

/// A player's class look: the own one's from the login, another's from a class picture.
fn class_of(sq: &Square, map: &ModelMap, own: &OwnLook) -> Option<ClassLook> {
    match (classify_player_face(sq.name), map.lookup(sq.name)) {
        _ if sq.own_tile => own.class,
        (Some(PlayerFace::Class(arch)), _) => class_look(arch),
        (None, Visual::Person { class, .. }) => *class,
        _ => None,
    }
}

/// The look and body of a player or a `person` creature, and a creature's outfit.
fn person_of(sq: &Square, map: &ModelMap, own: &OwnLook) -> Option<(Look, Body, Option<Outfit>)> {
    match (classify_player_face(sq.name), map.lookup(sq.name)) {
        _ if sq.own_tile => Some((own.look, own.body, None)),
        (Some(kind), _) => Some((other_look(kind), Body::Regular, None)),
        (None, &Visual::Person { body, look, dress, broom, armour, wide, head, build, horns, ears, .. }) => {
            // Pointed ears are the skin's colour (its tint over a human's skin).
            let ears = ears.then(|| [0.78 * look.skin[0], 0.56 * look.skin[1], 0.42 * look.skin[2]]);
            Some((look, body, Some(Outfit { scale: look.scale, dress, broom, armour, wide, head, build, horns, ears })))
        }
        _ => None,
    }
}

/// The three body files of a person: the class's outfit, a bare head under horns or
/// pointed ears, and metal armour.
fn person_files(look: Look, body: Body, class: Option<ClassLook>, outfit: Option<Outfit>) -> (Look, [&'static str; 3]) {
    let (look, mut files) = class_dressed(look, body, class, outfit.is_some());
    // Horns and pointed ears go on a bare head.
    if outfit.is_some_and(|o| o.horns || o.ears.is_some()) {
        files[1] = "";
    }
    if outfit.is_some_and(|o| o.armour) {
        files[2] = outfit_file(body, Garb::Metal);
    }
    (look, files)
}

/// The robe or rags a person wears, if any.
fn dress_of(sq: &Square, map: &ModelMap, class: Option<ClassLook>, armour: bool) -> Option<crate::robe::Dress> {
    let rags = match map.lookup(sq.name) {
        Visual::Person { rags, .. } => *rags,
        _ => None,
    };
    crate::robe::Dress::of(class, armour)
        .map(|d| crate::robe::Dress { rags, ..d })
        .or(rags.map(|_| crate::robe::Dress { robe: false, back: None, over: None, rags }))
}

/// The own character is marked as such; another's class look is kept for its gear.
fn mark_owner_or_class(sq: &Square, draw: &mut Draw, e: Entity, class: Option<ClassLook>, scale: f32) {
    if sq.own_tile {
        draw.commands.entity(e).insert(OwnCharacter);
    } else if let Some(class) = class {
        draw.commands.entity(e).insert(ClassGear(class, scale));
    }
}

/// Spawns a gliding player or `person` creature as a 3D character; false if it is none
/// or one of its body files failed to load.
fn spawn_person(sq: &Square, draw: &mut Draw, own: &OwnLook) -> bool {
    if !glides(sq.layer) {
        return false;
    }
    let Some((look, body, outfit)) = person_of(sq, &draw.models.map, own) else { return false };
    let class = class_of(sq, &draw.models.map, own);
    let (look, files) = person_files(look, body, class, outfit);
    if files.iter().any(|f| draw.models.failed.contains(*f)) {
        return false;
    }
    let pos = sq.centre();
    let load = |f: &str| draw.models.scene(&draw.server, f);
    let at = Transform::from_translation(pos);
    let e = spawn_character(&mut draw.commands, load, files, &look, at, Visibility::Hidden, (sq.tv(), Glide { target: pos }));
    draw.commands.entity(e).insert(Assembled(files.iter().filter(|f| !f.is_empty()).map(|f| f.to_string()).collect()));
    if let Some(dress) = dress_of(sq, &draw.models.map, class, outfit.is_some_and(|o| o.armour)) {
        draw.commands.entity(e).insert(dress);
    }
    mark_owner_or_class(sq, draw, e, class, look.scale);
    if let Some(outfit) = outfit {
        draw.commands.entity(e).insert(outfit);
    }
    true
}

/// Spawns a gliding beast, monster or skeleton as a 3D creature in the middle of its
/// square; false if the square holds none (or its model failed to load).
fn spawn_creature(sq: &Square, draw: &mut Draw) -> bool {
    if !glides(sq.layer) {
        return false;
    }
    let pos = sq.centre();
    let at = Transform::from_translation(pos);
    let extra = (sq.tv(), Glide { target: pos });
    match draw.models.map.lookup(sq.name) {
        Visual::Beast { file, scale, tint } if !draw.models.failed.contains(file) => {
            let (file, scale, tint) = (file.clone(), *scale, tint.unwrap_or([1.0; 3]));
            let scene = draw.models.scene(&draw.server, &file);
            spawn_beast(&mut draw.commands, scene, &file, scale, tint, at, (extra, Assembled(vec![file.clone()])));
        }
        Visual::Monster { file, scale, tint } if !draw.models.failed.contains(file) => {
            let (file, scale, tint) = (file.clone(), *scale, tint.unwrap_or([1.0; 3]));
            let scene = draw.models.scene(&draw.server, &file);
            spawn_monster(&mut draw.commands, scene, scale, tint, at, Visibility::Hidden, (extra, Assembled(vec![file])));
        }
        &Visual::Skeleton { scale, tint } => {
            let file = character_files(Body::Regular)[0];
            let look = Look { scale, ..look_for_race("human") };
            let load = |f: &str| draw.models.scene(&draw.server, f);
            let bony = crate::skeleton::Bony(tint.unwrap_or([1.0; 3]));
            let extra = (extra, Assembled(vec![file.to_string()]), bony);
            spawn_character(&mut draw.commands, load, [file, "", ""], &look, at, Visibility::Hidden, extra);
        }
        _ => return false,
    }
    true
}

/// A building's sign: its material and its width / height.
type Sign = (Handle<StandardMaterial>, f32);

/// The parts of a wall or building, and a building's sign.
fn built_parts(sq: &Square, draw: &mut Draw) -> (Option<Vec<Part>>, Option<Sign>) {
    match draw.models.map.lookup(sq.name) {
        Visual::Wall { style } => (Some(wall_parts(style, sq.join)).filter(|p| !p.is_empty()), None),
        Visual::Building { recipe, .. } if lava_floor(recipe, sq.layer) => (None, None),
        Visual::Building { recipe, sign, icon } => {
            let sign = sign.as_deref().map(|t| sign_material(t, icon.as_deref(), sq.shade, &mut draw.images, &mut draw.materials));
            (parts(recipe, sq.fw, sq.fh), sign)
        }
        _ => (None, None),
    }
}

/// The picture (a block for walls, else a card), hidden while a 3D thing loads,
/// shows if it fails: never nothing.
fn stand_in(sq: &Square, draw: &mut Draw, material: &Handle<StandardMaterial>, parent: Entity, at: Vec3) {
    let picture = if sq.wall {
        Transform::from_translation(sq.tile + Vec3::new(0.5, 0.5, 0.5))
    } else {
        Transform::from_translation(card_position(sq.tile, sq.size, sq.layer)).with_scale(Vec3::new(sq.size.x / TILE_PX, sq.size.y / TILE_PX, 1.0))
    };
    let t = Transform { translation: picture.translation - at, ..picture };
    let mesh = if sq.wall { draw.textures.cube.clone() } else { draw.textures.card.clone() };
    let mut e = draw.commands.spawn((StandIn, Mesh3d(mesh), MeshMaterial3d(material.clone()), t, Visibility::Hidden, ChildOf(parent)));
    if !sq.wall {
        e.insert(Card);
    }
}

/// Spawns the root of a wall or building: its span, the files it needs, a glide for
/// creatures and a door for houses.
fn spawn_building_root(sq: &Square, draw: &mut Draw, parts: &[Part], at: Vec3) -> Entity {
    let span = Span(sq.x + 1 - sq.fw as i32, sq.y + 1 - sq.fh as i32, sq.x, sq.y);
    let files: Vec<String> = parts.iter().filter_map(|p| if let Shape::Model(f) = p.shape { Some(f.to_string()) } else { None }).collect();
    let parent = draw.commands.spawn((sq.tv(), span, Assembled(files), Transform::from_translation(at).with_rotation(sq.turn), Visibility::default())).id();
    if glides(sq.layer) {
        draw.commands.entity(parent).insert(Glide { target: at });
    }
    if let Visual::Building { recipe, .. } = draw.models.map.lookup(sq.name)
        && let Some(step) = doorstep(recipe, sq.fw, sq.fh)
    {
        draw.commands.entity(parent).insert(Door(at + Vec3::from(step)));
    }
    parent
}

/// Spawns a wall or building from its parts; false if the square holds none.
fn spawn_built(sq: &Square, draw: &mut Draw, material: &Handle<StandardMaterial>) -> bool {
    let (built, sign) = built_parts(sq, draw);
    let Some(parts) = built else { return false };
    let at = if sq.layer == 0 || sq.wall { sq.centre() } else { sq.middle };
    let parent = spawn_building_root(sq, draw, &parts, at);
    for p in &parts {
        let child = spawn_part(&mut draw.commands, p, sq.shade, &draw.textures, sign.as_ref(), &draw.server, &mut draw.models, &mut draw.materials);
        draw.commands.entity(parent).add_child(child);
    }
    stand_in(sq, draw, material, parent, at);
    true
}

/// Spawns a statue: a plinth with a stone figure on top; false if the square holds none.
fn spawn_statue(sq: &Square, draw: &mut Draw) -> bool {
    let Visual::Statue { tint } = draw.models.map.lookup(sq.name) else { return false };
    let [r, g, b] = tint.unwrap_or(STATUE_STONE);
    let stone = Color::srgb(r * sq.shade, g * sq.shade, b * sq.shade);
    let parent = draw.commands.spawn((sq.tv(), Transform::from_translation(sq.centre()), Visibility::default())).id();
    let plinth = draw.models.scene(&draw.server, PLINTH);
    draw.commands.spawn((WorldAssetRoot(plinth), Transform::from_scale(Vec3::splat(PLINTH_SCALE)), ModelTint(stone), ChildOf(parent)));
    // Body colours are tints relative to a human (`races::tints`): undo that, so
    // every part comes out the same stone.
    let human = look_for_race("human");
    let rel = |h: [f32; 3]| [r * h[0], g * h[1], b * h[2]];
    let look = Look { scale: 0.5, skin: rel(human.skin), hair: [r, g, b], clothes: rel(human.clothes) };
    let load = |f: &str| draw.models.scene(&draw.server, f);
    let on_top = Transform::from_xyz(0.0, PLINTH_TOP, 0.0);
    let figure = spawn_character(&mut draw.commands, load, character_files(Body::Regular), &look, on_top, Visibility::Inherited, StatueFigure);
    draw.commands.entity(figure).insert(ChildOf(parent));
    true
}

/// A square's 3D model (file, placement, tint), if it has one that has not failed.
fn scene_model(sq: &Square, models: &Models) -> Option<(String, Transform, Color)> {
    match models.map.lookup(sq.name) {
        Visual::Model { file, scale, rot_deg, dy, tint } if !models.failed.contains(file) => {
            let t = Transform::from_xyz(0.0, *dy, 0.0).with_rotation(Quat::from_rotation_y(-rot_deg.to_radians())).with_scale(Vec3::splat(*scale));
            let [r, g, b] = tint.unwrap_or([1.0; 3]);
            Some((file.clone(), t, Color::srgb(r * sq.shade, g * sq.shade, b * sq.shade)))
        }
        _ => None,
    }
}

/// A block under a ground square, reaching down below the lowest ground.
/// Raised and sunken squares (hills, sea) need sides, or they float over a gap.
/// Under a 3D model (mountains, hills) the sides are the picture's average
/// colour: the stretched pixels did not blend into the model.
fn add_ground_side(sq: &Square, draw: &mut Draw, ground: Entity, material: Handle<StandardMaterial>, has_model: bool) {
    let depth = sq.h - GROUND_BOTTOM;
    let side = Transform::from_xyz(0.0, -depth / 2.0 - 0.002, 0.0).with_scale(Vec3::new(1.0, depth, 1.0));
    let side_material = if has_model {
        let c = sq.mean.to_srgba();
        draw.materials.add(StandardMaterial {
            base_color: Color::srgb(c.red * sq.shade, c.green * sq.shade, c.blue * sq.shade),
            perceptual_roughness: 0.9,
            ..default()
        })
    } else {
        material
    };
    draw.commands.entity(ground).with_child((Mesh3d(draw.textures.cube.clone()), MeshMaterial3d(side_material), side));
}

/// A ground square (or a flat decoration just above it), with its side block and,
/// for trees and rocks, its model standing on the ground tile.
fn spawn_ground(sq: &Square, draw: &mut Draw, material: Handle<StandardMaterial>, flat: bool) {
    let model = scene_model(sq, &draw.models);
    // Flat decorations lie just above the ground, as big as their picture.
    let (pos, scale) = if flat {
        (sq.middle + Vec3::Y * (0.01 + sq.layer as f32 * 0.001), Vec3::new(sq.fw as f32, 1.0, sq.fh as f32))
    } else {
        (sq.centre(), Vec3::ONE)
    };
    let t = Transform::from_translation(pos).with_scale(scale);
    let ground = draw.commands.spawn((sq.tv(), Mesh3d(draw.textures.ground.clone()), MeshMaterial3d(material.clone()), t)).id();
    if !flat {
        add_ground_side(sq, draw, ground, material, model.is_some());
    }
    if let (Some((file, t, tint)), false) = (model, flat) {
        let handle = draw.models.scene(&draw.server, &file);
        draw.commands.entity(ground).with_child((WorldAssetRoot(handle), t, ModelTint(tint)));
    }
}

/// The developer's compare row: models beside the card. Children turn with the card,
/// so the row always faces the camera.
fn add_compare_row(sq: &Square, draw: &mut Draw, card: Entity, scale: Vec3) {
    let base = sq.name.split('.').next().unwrap_or(sq.name);
    for (i, file) in draw.models.compare.get(base).cloned().unwrap_or_default().iter().enumerate() {
        let at = Vec3::new(COMPARE_GAP * (i + 1) as f32, 0.0, 0.0) / scale;
        let turn = Quat::from_rotation_y(COMPARE_TURN_DEG.to_radians());
        let scene = draw.models.scene(&draw.server, file);
        draw.commands.entity(card).with_child((WorldAssetRoot(scene), Transform::from_translation(at).with_rotation(turn).with_scale(1.0 / scale)));
    }
}

/// Gives a card the 3D model it waits for: glides to the model's spot, or stands in the middle.
fn add_pending_model(sq: &Square, draw: &mut Draw, card: Entity, pos: Vec3, model: Option<(String, Transform, Color)>) {
    if glides(sq.layer) {
        match (model, model_spot(draw.models.map.lookup(sq.name), sq.tile)) {
            (Some((file, t, tint)), Some(spot)) => {
                let handle = draw.models.scene(&draw.server, &file);
                let transform = Transform { translation: spot, ..t };
                draw.commands.entity(card).insert((Glide { target: spot }, PendingModel { file, handle, transform, tint }));
            }
            _ => {
                draw.commands.entity(card).insert(Glide { target: pos });
            }
        }
    } else if let Some((file, t, tint)) = model {
        let handle = draw.models.scene(&draw.server, &file);
        let transform = Transform { translation: sq.middle + t.translation, ..t };
        draw.commands.entity(card).insert(PendingModel { file, handle, transform, tint });
    }
}

/// An upright picture card, with its compare row and 3D model.
fn spawn_card(sq: &Square, draw: &mut Draw, material: Handle<StandardMaterial>) {
    let model = scene_model(sq, &draw.models);
    let pos = card_position(sq.tile, sq.size, sq.layer);
    let t = Transform::from_translation(pos).with_scale(Vec3::new(sq.size.x / TILE_PX, sq.size.y / TILE_PX, 1.0));
    let card = draw.commands.spawn((sq.tv(), Card, Mesh3d(draw.textures.card.clone()), MeshMaterial3d(material), t)).id();
    add_compare_row(sq, draw, card, t.scale);
    add_pending_model(sq, draw, card, pos, model);
}

/// Spawns what is left once no 3D character, creature, building or statue fits: a wall
/// block, a ground square, or a card.
fn spawn_plain(sq: &Square, draw: &mut Draw, material: Handle<StandardMaterial>) {
    let flat = matches!(draw.models.map.lookup(sq.name), Visual::Flat);
    if sq.wall {
        let t = Transform::from_translation(sq.tile + Vec3::new(0.5, 0.5, 0.5));
        draw.commands.spawn((sq.tv(), Mesh3d(draw.textures.cube.clone()), MeshMaterial3d(material), t));
    } else if sq.layer == 0 || flat {
        spawn_ground(sq, draw, material, flat);
    } else {
        spawn_card(sq, draw, material);
    }
}

/// Makes the one new entity (or tree of entities) for a square.
fn place(sq: &Square, draw: &mut Draw, own: &OwnLook) {
    let material = square_material(sq, draw);
    if spawn_person(sq, draw, own) {
        return;
    }
    // A player never shows as the server's pixel picture, even when the 3D body fails.
    if classify_player_face(sq.name).is_some() {
        return;
    }
    if spawn_creature(sq, draw) || spawn_built(sq, draw, &material) || spawn_statue(sq, draw) {
        return;
    }
    spawn_plain(sq, draw, material);
}

fn sync_visuals(mut draw: Draw, input: Inputs, mut existing: Existing, mut solid: ResMut<Solid>, mut seen_gen: Local<u32>) {
    if !input.view.is_changed() && !draw.textures.is_changed() {
        return;
    }
    // A new map: forget the last one's squares (the square numbers carry on).
    let new_map = *seen_gen != input.map_gen.0;
    *seen_gen = input.map_gen.0;
    let (mut want, frame) = plan(&input, &draw, &mut solid);
    let memory = |x, y| remembered(&input, frame.player, new_map, x, y);
    let mut orphans = sweep(&existing, &mut want, &mut draw.commands, memory);
    for (key, val) in want {
        let Some(shown) = draw.textures.images.get(&val.0).cloned() else { continue };
        let sq = Square::new(key, val, name_of(&input.faces, val.0), shown, &frame);
        if !reuse_orphan(&sq, &mut orphans, &mut existing, &draw.models, &input.faces) {
            place(&sq, &mut draw, &input.own);
        }
    }
    for e in orphans {
        draw.commands.entity(e).despawn();
    }
}

/// Pictures that share a 3D body: every player picture, or one creature's frames.
fn body_key<'a>(map: &ModelMap, name: &'a str) -> Option<&'a str> {
    match map.lookup(name) {
        _ if classify_player_face(name).is_some() => Some("player"),
        Visual::Person { .. } | Visual::Monster { .. } | Visual::Beast { .. } => name.split('.').next(),
        _ => None,
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
    let (w, h, px) = crossfir3d::signs::sign_pixels(text, icon);
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
/// The lava pictures are lava floors on the floor layer, and alchemy baths above it.
fn lava_floor(recipe: &str, layer: u8) -> bool {
    layer == 0 && recipe == "alchemy_bath"
}

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
        .with_rotation(Quat::from_rotation_y(-p.yaw_deg.to_radians()) * Quat::from_rotation_z(-p.tilt_deg.to_radians()))
        .with_scale(Vec3::from(p.scale));
    let [r, g, b] = p.color.unwrap_or([1.0; 3]);
    let color = Color::srgb(r * shade, g * shade, b * shade);
    let (mesh, size) = match p.shape {
        Shape::Model(file) => {
            let handle = models.scene(server, file);
            let mut e = commands.spawn((WorldAssetRoot(handle), t, ModelTint(color), Visibility::Hidden));
            if file.starts_with("kaykit/wall") {
                e.insert(StoneWall(t.scale.y));
            }
            return e.id();
        }
        Shape::Box(size) => (textures.cube.clone(), Vec3::from(size)),
        Shape::Bricks([x, y, z]) | Shape::Water([x, y, z]) => {
            // One scale for every face of the box: bricks by length and height, waves
            // across the top, so the pattern keeps its size on long and short boxes.
            let (image, repeats) = match p.shape {
                Shape::Bricks(_) => (&textures.bricks, Vec2::new(x.max(z) / BRICK_TILE[0], y / BRICK_TILE[1])),
                _ => (&textures.waves, Vec2::new(x, z) / WAVE_TILE),
            };
            let material = materials.add(StandardMaterial {
                base_color: color,
                base_color_texture: Some(image.clone()),
                uv_transform: bevy::math::Affine2::from_scale(repeats),
                perceptual_roughness: 0.9,
                ..default()
            });
            let t = Transform { scale: t.scale * Vec3::new(x, y, z), ..t };
            let mut part = commands.spawn((Mesh3d(textures.cube.clone()), MeshMaterial3d(material), t, Visibility::Hidden));
            if let Shape::Water(_) = p.shape {
                part.insert(Waves(repeats));
            }
            return part.id();
        }
        Shape::Cylinder { radius, height } => (textures.cylinder.clone(), Vec3::new(radius, height, radius)),
        Shape::Cone { radius, height } => (textures.cone.clone(), Vec3::new(radius, height, radius)),
        Shape::Frustum { radius, height } => (textures.frustum.clone(), Vec3::new(radius, height, radius)),
        Shape::Ball(radius) => (textures.ball.clone(), Vec3::splat(radius)),
        Shape::Wedge(size) => (textures.wedge.clone(), Vec3::from(size)),
        Shape::Lamp(_) | Shape::RoundLamp { .. } | Shape::Orb(_) => {
            let (mesh, size) = match p.shape {
                Shape::Orb(radius) => (textures.ball.clone(), Vec3::splat(radius)),
                Shape::RoundLamp { radius, height } => (textures.cylinder.clone(), Vec3::new(radius, height, radius)),
                Shape::Lamp(size) => (textures.cube.clone(), Vec3::from(size)),
                _ => unreachable!(),
            };
            let glow = materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * LAMP_GLOW,
                unlit: true,
                ..default()
            });
            let t = Transform { scale: t.scale * size, ..t };
            let light = PointLight { color, intensity: LAMP_LUMENS, range: LAMP_RANGE, shadow_maps_enabled: false, ..default() };
            // The light is a child of the scaled shape: undo the scale so its range is in squares.
            let unscaled = Transform::from_scale(Vec3::ONE / t.scale);
            return commands.spawn((Mesh3d(mesh), MeshMaterial3d(glow), t, Visibility::Hidden, NotShadowCaster)).with_child((light, unscaled)).id();
        }
        Shape::Mist(radius) => {
            let mist = materials.add(StandardMaterial {
                base_color: color.with_alpha(0.45),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                ..default()
            });
            let t = Transform { scale: t.scale * radius, ..t };
            return commands.spawn((Mesh3d(textures.ball.clone()), MeshMaterial3d(mist), t, Visibility::Hidden, NotShadowCaster)).id();
        }
        Shape::Fire { .. } | Shape::Glow(_) => {
            let glow = materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * LAMP_GLOW,
                unlit: true,
                ..default()
            });
            let (mesh, size) = match p.shape {
                Shape::Fire { radius, height } => (textures.cone.clone(), Vec3::new(radius, height, radius)),
                Shape::Glow(size) => (textures.cube.clone(), Vec3::from(size)),
                _ => unreachable!(),
            };
            let t = Transform { scale: t.scale * size, ..t };
            return commands.spawn((Mesh3d(mesh), MeshMaterial3d(glow), t, Visibility::Hidden, NotShadowCaster)).id();
        }
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

/// What becomes of an assembled thing's part (or its picture) once its files are done:
/// shown (true), gone (false), or None while they load.
fn on_load(state: Load, picture: bool) -> Option<bool> {
    match state {
        Load::Loading => None,
        Load::Loaded => Some(!picture),
        Load::Failed => Some(picture),
    }
}

/// Shows an assembled thing once all its files have loaded (its picture goes); if one
/// fails, its parts go and the picture shows.
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
            match on_load(state, stand_ins.contains(c)) {
                Some(true) => {
                    commands.entity(c).try_insert(Visibility::Inherited);
                }
                Some(false) => commands.entity(c).try_despawn(),
                None => {}
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
    tints: Query<(&ModelTint, Option<&SkinTint>, Has<Recolour>)>,
    mut images: ResMut<Assets<Image>>,
    mut greys: Local<HashMap<AssetId<Image>, Handle<Image>>>,
    part_of: Query<&PartOf>,
    statues: Query<(), With<StatueFigure>>,
    names: Query<&Name>,
    children: Query<&Children>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((ModelTint(tint), skin, recolour)) = tints.get(ready.entity) else { return };
    if *tint == Color::WHITE && skin.is_none() {
        return;
    }
    // A statue's figure is plain stone: its tint, with no picture on it.
    let stone = part_of.get(ready.entity).is_ok_and(|p| statues.contains(p.0));
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
        if recolour && let Some(tex) = copy.base_color_texture.clone() {
            let grey = greys.entry(tex.id()).or_insert_with(|| {
                let grey = images.get(&tex).map(grey_image);
                grey.map_or(tex.clone(), |g| images.add(g))
            });
            copy.base_color_texture = Some(grey.clone());
        }
        if stone {
            // Darker than the tint: the plinth's own marble colour darkens it too.
            copy.base_color = LinearRgba::new(tint.red * STONE_SHADE, tint.green * STONE_SHADE, tint.blue * STONE_SHADE, 1.0).into();
            copy.base_color_texture = None;
            // Outfits can be shiny or metallic; stone is neither (it mirrored the sky).
            copy.metallic = 0.0;
            copy.metallic_roughness_texture = None;
            copy.perceptual_roughness = 0.9;
            copy.reflectance = 0.2;
            copy.emissive = LinearRgba::BLACK;
            copy.emissive_texture = None;
        }
        commands.entity(d).try_insert(MeshMaterial3d(materials.add(copy)));
    }
}

/// A copy of an sRGB 8-bit picture in grey, brightened so a mid-dark colour comes out
/// near white (then the tint alone sets the colour).
fn grey_image(image: &Image) -> Image {
    let mut grey = image.clone();
    if let Some(data) = grey.data.as_mut() {
        for px in data.chunks_exact_mut(4) {
            let l = (0.3 * px[0] as f32 + 0.59 * px[1] as f32 + 0.11 * px[2] as f32) / 255.0;
            // ponytail: one fixed lift for every atlas; tune per file if one looks washed out.
            let v = ((l * 1.6).min(1.0) * 255.0) as u8;
            px[..3].fill(v);
        }
    }
    grey
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
    // Haze from the edge of what the server sends out to the edge of map memory: nearby
    // houses keep their colours, remembered ones fade.
    let radius = size.0.min(size.1) as f32 / 2.0;
    fog.falloff = FogFalloff::Linear { start: distance + radius, end: distance + MEMORY_SQUARES as f32 };
}

/// The player on a house's squares stands on its doorstep, outside, with the house in
/// view. One-square things (a well) are drawn with them; anything bigger they stand in
/// (a castle) is hidden.
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
        let big = x1 > x0 || y1 > y0;
        vis.set_if_neq(if inside && door.is_none() && big { Visibility::Hidden } else { Visibility::Inherited });
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
/// The player's square's light, 0 (dark) ..= 255 (the server darkens by hour, season,
/// indoors).
pub(crate) fn darkness(view: &View, size: &ViewSize) -> u8 {
    view.0.cell((size.0 / 2) as i8, (size.1 / 2) as i8).map_or(255, |c| c.darkness)
}

/// Where the sun (or moon) is now, and the direction toward it.
pub(crate) fn sun_now(clock: &Clock) -> (SunPose, Vec3) {
    // CROSSFIR3D_HOUR fixes the hour for screenshots; read once, not every frame.
    static FORCED: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    let forced = *FORCED.get_or_init(|| std::env::var("CROSSFIR3D_HOUR").ok().and_then(|h| h.parse().ok()));
    let pose = sun_pose(forced.or(clock.0.hour()).unwrap_or(FALLBACK_HOUR));
    // azimuth 0 = east (+X), PI/2 = south (+Z), PI = west (-X)
    let dir = Vec3::new(
        pose.azimuth.cos() * pose.elevation.cos(),
        pose.elevation.sin(),
        pose.azimuth.sin() * pose.elevation.cos(),
    );
    (pose, dir)
}

fn lighting(
    view: Res<View>,
    size: Res<ViewSize>,
    clock: Res<Clock>,
    settings: Res<CurrentSettings>,
    indoors: Res<Indoors>,
    sun: Single<(&mut DirectionalLight, &mut Transform), With<Sun>>,
) {
    let (mut light, mut transform) = sun.into_inner();
    let dark = darkness(&view, &size);
    // Indoors no sun reaches: a dim, warm torchlight from high up, whatever the hour.
    if indoors.0 {
        *transform = Transform::from_xyz(3.0, 20.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y);
        light.color = Color::srgb(1.0, 0.75, 0.5);
        light.illuminance = 0.35 * SUN_LUX * dark as f32 / 255.0;
        light.shadow_maps_enabled = settings.0.shadows != Quality::Off;
        return;
    }
    let (pose, dir) = sun_now(&clock);
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
    fn the_picture_shows_only_if_the_model_fails() {
        assert_eq!(on_load(Load::Loaded, true), Some(false), "a loaded thing's picture goes");
        assert_eq!(on_load(Load::Loaded, false), Some(true), "and its parts show");
        assert_eq!(on_load(Load::Failed, true), Some(true), "a failed one's picture shows");
        assert_eq!(on_load(Load::Failed, false), Some(false), "and its parts go");
        assert_eq!(on_load(Load::Loading, true), None);
    }

    #[test]
    fn leaving_the_game_forgets_face_pictures() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let mut textures = Textures::default();
        textures.images.insert(7, (Handle::default(), Vec2::ONE, Color::WHITE));
        world.insert_resource(textures);
        world.run_system_once(forget_faces).unwrap();
        assert!(world.resource::<Textures>().images.is_empty());
    }

    #[test]
    fn mean_colour_skips_see_through_pixels() {
        let px = [255, 0, 0, 255, 0, 0, 255, 255, 0, 255, 0, 0];
        assert_eq!(mean_colour(&px), Color::srgb(0.5, 0.0, 0.5));
        assert_eq!(mean_colour(&[]), Color::srgb(0.5, 0.5, 0.5));
    }

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
