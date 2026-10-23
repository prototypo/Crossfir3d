//! The 3D people: the player's character, other players, and the new-character preview.
//! One body (with hair and an outfit) for everyone, sized and coloured per race, moved
//! by the Universal Animation Library clips. All the files share one skeleton with the
//! same bone names, so a clip drives every part without retargeting.

use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::SystemParam;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::gltf::{Gltf, GltfAssetLabel};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady};
use crossfir3d::login::LoginState;
use crossfir3d::anim::{clip_name, looping, Anim, AnimEvent, AnimState};
use crossfir3d::held::{class_look, held, helmet, worn, Carry, ClassLook, Garb, Hand, Headwear};
use crossfir3d::races::{default_body_path, head_triangles, load_body, look_for_race, tints, Body, Look};
use std::collections::HashMap;

use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::{Indices, VertexAttributeValues};

use crate::input::Facing;
use crate::world::{Glide, ModelTint, Recolour, SkinTint};
use crate::{AppState, Inv, Session};

const ANIMS_FILE: &str = "models/animations/UAL1_Standard.glb";
/// The model is about 1.8 units tall; a person should fit one square.
const BODY_SCALE: f32 = 0.6;
const ALL: [Anim; 5] = [Anim::Idle, Anim::Walk, Anim::Attack, Anim::Cast, Anim::Death];
/// The preview draws only this layer, far away from the world.
const PREVIEW_LAYER: usize = 1;
const PREVIEW_AT: Vec3 = Vec3::new(0.0, -500.0, 0.0);
pub const PREVIEW_PX: u32 = 256;
/// The weapons pack is about 5 times the size of the body models. Its models (grip at
/// the origin) sit in the hand bone unturned (checked by screenshot).
const WEAPON_SCALE: f32 = 0.18;

pub struct CharactersPlugin;

impl Plugin for CharactersPlugin {
    /// Registers the character resources, observers and systems.
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerAnimEvents>()
            .init_resource::<OwnLook>()
            .init_resource::<Preview>()
            .init_resource::<crate::hat::Leather>()
            .add_systems(Update, (crate::robe::dress_robes, crate::skeleton::build_skeletons))
            .add_observer(bind_bones)
            .add_systems(Startup, (load_anims, setup_preview))
            .add_systems(Update, (build_graph, animate, animate_beasts, hold_items, wear_armour, wear_helmet, dress_people, gear_up, freeze_statues))
            .add_systems(PostUpdate, hold_bone_shapes.after(bevy::app::AnimationSystems).before(bevy::transform::TransformSystems::Propagate))
            .add_systems(Update, update_preview.run_if(in_state(AppState::Login)))
            .add_systems(OnEnter(AppState::InGame), set_own_look)
            .add_systems(OnExit(AppState::Login), clear_preview);
    }
}

/// A stone character on a plinth: it stands in the first frame of its idle pose.
#[derive(Component)]
pub struct StatueFigure;

fn freeze_statues(mut q: Query<(&Character, &mut AnimationPlayer), With<StatueFigure>>) {
    for (c, mut player) in &mut q {
        if c.playing.is_some() && !player.all_paused() {
            player.pause_all();
        }
    }
}

/// Things the player did that start an animation (messages, casts).
#[derive(Resource, Default)]
pub struct PlayerAnimEvents(pub Vec<AnimEvent>);

/// How the player's own character looks (their race and Body choice).
#[derive(Resource)]
pub struct OwnLook {
    pub look: Look,
    pub body: Body,
    /// The look of the own character's class: worn or held items take its place.
    pub class: Option<ClassLook>,
}

impl Default for OwnLook {
    fn default() -> Self {
        OwnLook { look: look_for_race("human"), body: Body::Regular, class: None }
    }
}

/// The new-character preview: the picture egui shows, and what is in it.
#[derive(Resource, Default)]
pub struct Preview {
    pub image: Handle<Image>,
    pub body: Body,
    shown: Option<(String, Body)>,
}

#[derive(Resource, Default)]
struct Anims {
    gltf: Handle<Gltf>,
    /// The graph and one node per entry of `ALL`.
    graph: Option<(Handle<AnimationGraph>, Vec<AnimationNodeIndex>)>,
}

#[derive(Component, Default)]
pub struct Character {
    state: AnimState,
    playing: Option<Anim>,
    moving: bool,
}

/// The player's own character (it faces the way the player last moved).
#[derive(Component)]
pub struct OwnCharacter;

/// A body part scene inside a character; its bones get animation targets once spawned.
#[derive(Component)]
pub(crate) struct PartOf(pub Entity);

#[derive(Component)]
struct PreviewBody;

/// What the own character has in its hands now (set once the models are attached).
#[derive(Component, PartialEq)]
struct Holding(Vec<(&'static str, Hand)>);

/// A weapon or shield model in a hand.
#[derive(Component)]
struct HeldModel;

/// The outfit part of a character (the third of `character_files`).
#[derive(Component)]
pub(crate) struct OutfitPart;

/// What the own character wears now (see `crossfir3d::held::worn`).
#[derive(Component, PartialEq)]
struct Wearing(Option<(Garb, [f32; 3])>);

/// Body, hair, and outfit files for a Body choice.
pub fn character_files(body: Body) -> [&'static str; 3] {
    match body {
        Body::Regular => ["characters/Superhero_Male_FullBody.gltf", "characters/Hair_SimpleParted.gltf", outfit_file(body, Garb::Peasant)],
        Body::Other => ["characters/Superhero_Female_FullBody.gltf", "characters/Hair_Long.gltf", outfit_file(body, Garb::Peasant)],
    }
}

pub fn outfit_file(body: Body, garb: Garb) -> &'static str {
    match (body, garb) {
        (Body::Regular, Garb::Peasant | Garb::Robe) => "outfits/Male_Peasant.gltf",
        (Body::Regular, Garb::Ranger) => "outfits/Male_Ranger.gltf",
        (Body::Regular, Garb::Metal) => "outfits/Male_Ranger_Metal.gltf",
        (Body::Other, Garb::Peasant | Garb::Robe) => "outfits/Female_Peasant.gltf",
        (Body::Other, Garb::Ranger) => "outfits/Female_Ranger.gltf",
        (Body::Other, Garb::Metal) => "outfits/Female_Ranger_Metal.gltf",
    }
}

/// Spawns a character standing at `at` from its scene files (body, hair, outfit; see
/// `character_files`); `parts` is the parts' visibility, `extra` goes on the root.
pub fn spawn_character(
    commands: &mut Commands,
    mut load: impl FnMut(&str) -> Handle<WorldAsset>,
    files: [&str; 3],
    look: &Look,
    at: Transform,
    parts: Visibility,
    extra: impl Bundle,
) -> Entity {
    let t = tints(look);
    let root = commands.spawn((at, Visibility::default(), Character::default(), AnimationPlayer::default(), extra)).id();
    let inner = Transform::from_scale(Vec3::splat(BODY_SCALE * look.scale));
    let color = |[r, g, b]: [f32; 3]| Color::srgb(r, g, b);
    for (i, (file, tint)) in files.into_iter().zip([t.skin, t.hair, t.clothes]).enumerate() {
        // No hair: no file.
        if file.is_empty() {
            continue;
        }
        let scene = load(file);
        let e = commands.spawn((WorldAssetRoot(scene), inner, parts, ModelTint(color(tint)), SkinTint(color(t.skin)), PartOf(root), ChildOf(root))).id();
        if i == 2 {
            commands.entity(e).insert(OutfitPart);
        }
    }
    root
}

/// Spawns a rigged creature (one file, the people's skeleton) standing at `at`; it is
/// animated like a person. `scale` 1 draws it at a person's body scale.
pub fn spawn_monster(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    scale: f32,
    tint: [f32; 3],
    at: Transform,
    parts: Visibility,
    extra: impl Bundle,
) -> Entity {
    let root = commands.spawn((at, Visibility::default(), Character::default(), AnimationPlayer::default(), extra)).id();
    let inner = Transform::from_scale(Vec3::splat(BODY_SCALE * scale));
    let [r, g, b] = tint;
    commands.spawn((WorldAssetRoot(scene), inner, parts, ModelTint(Color::srgb(r, g, b)), PartOf(root), ChildOf(root)));
    root
}

/// An animal or monster with its own skeleton and clips (`beast` in models.txt).
#[derive(Component)]
pub struct Beast {
    file: String,
    /// Whether the walk clip (true) or the idle clip (false) is playing.
    walking: Option<bool>,
}

/// Spawns a beast standing at `at` from its scene: it plays its own idle and walk clips.
pub fn spawn_beast(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    file: &str,
    scale: f32,
    [r, g, b]: [f32; 3],
    at: Transform,
    extra: impl Bundle,
) -> Entity {
    let beast = Beast { file: file.to_string(), walking: None };
    let root = commands.spawn((at, Visibility::default(), Character::default(), beast, extra)).id();
    let inner = Transform::from_scale(Vec3::splat(scale));
    let e = commands.spawn((WorldAssetRoot(scene), inner, Visibility::Hidden, ModelTint(Color::srgb(r, g, b)), ChildOf(root))).id();
    if [r, g, b] != [1.0; 3] {
        commands.entity(e).insert(Recolour);
    }
    root
}

fn load_anims(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Anims { gltf: server.load(ANIMS_FILE), graph: None });
}

/// Builds the shared animation graph once the clips have loaded.
fn build_graph(mut anims: ResMut<Anims>, gltfs: Res<Assets<Gltf>>, mut graphs: ResMut<Assets<AnimationGraph>>, mut warned: Local<bool>) {
    if anims.graph.is_some() {
        return;
    }
    let Some(gltf) = gltfs.get(&anims.gltf) else { return };
    let clips: Option<Vec<Handle<AnimationClip>>> =
        ALL.iter().map(|a| gltf.named_animations.get(clip_name(*a)).cloned()).collect();
    match clips {
        Some(clips) => {
            let (graph, nodes) = AnimationGraph::from_clips(clips);
            anims.graph = Some((graphs.add(graph), nodes));
        }
        None if !*warned => {
            *warned = true;
            warn!("{ANIMS_FILE}: a clip is missing; characters stand still");
        }
        None => {}
    }
}
/// The tint that turns the hood's green cloth into the peasant shirt's brown: the ratio of
/// their textures' average colours, corrected by hood and shirt colours in screenshots.
const HOOD_TO_PEASANT: [f32; 3] = [1.58, 1.04, 2.08];

/// The look and files of a character in its class's outfit (the hood in place of the
/// hair), or as it was with no class. The outfit takes the class's colour, or keeps the
/// look's own clothes colour when `own_colour` (creatures coloured from their pictures).
pub fn class_dressed(look: Look, body: Body, class: Option<ClassLook>, own_colour: bool) -> (Look, [&'static str; 3]) {
    let mut files = character_files(body);
    let Some(class) = class else { return (look, files) };
    files[2] = outfit_file(body, class.garb);
    // `tints` divides the clothes by a human's: so a tint t is the clothes t * human.
    let human = look_for_race("human").clothes;
    let mut look = look;
    if !own_colour {
        look.clothes = [0, 1, 2].map(|i| class.tint[i] * human[i]);
    }
    if matches!(class.head, Some(Headwear::Hood | Headwear::Mask | Headwear::GlowEyes(_))) {
        files[1] = match body {
            Body::Regular => "outfits/Male_Ranger_Head_Hood.gltf",
            Body::Other => "outfits/Female_Ranger_Head_Hood.gltf",
        };
        // The hood takes the hair's tint as is: the outfit's tint, and over peasant clothes
        // also the change from the hood's green cloth to the peasant's brown.
        let cloth = if matches!(class.garb, Garb::Peasant | Garb::Robe) { HOOD_TO_PEASANT } else { [1.0; 3] };
        look.hair = [0, 1, 2].map(|i| look.clothes[i] / human[i] * cloth[i]);
    }
    // Bald, or the hat covers the whole top of the head.
    if matches!(class.head, Some(Headwear::Bald | Headwear::Mitre | Headwear::HornedCap)) {
        files[1] = "";
    }
    (look, files)
}

/// Another player's class headwear and carried things, put on once their bones exist
/// (its look's scale too). The outfit and hood are chosen at spawn (`class_dressed`).
#[derive(Component)]
pub struct ClassGear(pub ClassLook, pub f32);

/// The queries that walk a body's bones.
#[derive(SystemParam)]
struct Skeleton<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    parents: Query<'w, 's, (&'static ChildOf, &'static Transform)>,
    parts: Query<'w, 's, (), With<PartOf>>,
}

impl Skeleton<'_, '_> {
    /// The bone called `name` under `root`, if its scene has spawned.
    fn find(&self, root: Entity, name: &str) -> Option<Entity> {
        self.children.iter_descendants(root).find(|e| self.names.get(*e).is_ok_and(|n| n.as_str() == name))
    }
}

/// The assets that dressing adds to.
#[derive(SystemParam)]
struct Stock<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    leather: Res<'w, crate::hat::Leather>,
}

impl Stock<'_> {
    /// The tools for putting on a hat, spawning with `commands`.
    fn hat_kit<'a, 'w, 's>(&'a mut self, commands: &'a mut Commands<'w, 's>) -> crate::hat::HatKit<'a, 'w, 's> {
        crate::hat::HatKit { commands, leather: &self.leather, meshes: &mut self.meshes, materials: &mut self.materials }
    }

    /// The tools for putting a thing in a hand, spawning with `commands`.
    fn hand_kit<'a, 'w, 's>(&'a mut self, commands: &'a mut Commands<'w, 's>, server: &'a AssetServer) -> worn::HandKit<'a, 'w, 's> {
        worn::HandKit { commands, server, meshes: &mut self.meshes, materials: &mut self.materials }
    }
}

/// Facing 1 = north .. 8 = northwest, clockwise. The model looks along +Z unturned.
fn facing_rotation(facing: u8) -> Quat {
    let clockwise = (facing.clamp(1, 8) - 1) as f32 * std::f32::consts::FRAC_PI_4;
    Quat::from_rotation_y(std::f32::consts::PI - clockwise)
}

/// Chooses and plays each character's animation from its movement and events.
#[allow(clippy::type_complexity)]
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    anims: Res<Anims>,
    facing: Res<Facing>,
    mut events: ResMut<PlayerAnimEvents>,
    mut q: Query<(Entity, &mut Character, &mut AnimationPlayer, &mut Transform, Option<&Glide>, Has<AnimationGraphHandle>, Has<OwnCharacter>)>,
) {
    let Some((graph, nodes)) = &anims.graph else { return };
    let now = time.elapsed_secs();
    let own_events = std::mem::take(&mut events.0);
    for (e, mut c, mut player, mut t, glide, has_graph, own) in &mut q {
        if !has_graph {
            commands.entity(e).try_insert(AnimationGraphHandle(graph.clone()));
        }
        let step = glide.map_or(Vec3::ZERO, |g| g.target - t.translation);
        let moving = step.length() > 0.02;
        if moving != c.moving {
            c.moving = moving;
            c.state.on_event(if moving { AnimEvent::StepStarted } else { AnimEvent::StepEnded }, now);
        }
        if own {
            for ev in &own_events {
                c.state.on_event(*ev, now);
            }
            t.rotation = facing_rotation(facing.0);
        } else if moving {
            t.rotation = Quat::from_rotation_y(step.x.atan2(step.z));
        }
        let anim = c.state.current(now);
        if c.playing != Some(anim) {
            c.playing = Some(anim);
            player.stop_all();
            let index = ALL.iter().position(|a| *a == anim).unwrap_or(0);
            let active = player.play(nodes[index]);
            if looping(anim) {
                active.repeat();
            }
        }
    }
}

/// The player's race (from the character list, or the one just created) and Body.
fn set_own_look(mut own: ResMut<OwnLook>, session: Option<NonSend<Session>>) {
    let Some(session) = session else { return };
    let name = session.login.character();
    let listed = session.login.characters().iter().find(|c| c.name == name);
    let race = listed.map(|c| c.race.clone()).or_else(|| session.login.newchar.race.clone()).unwrap_or_default();
    let class = listed.map(|c| c.class.clone()).or_else(|| session.login.newchar.class.clone()).unwrap_or_default();
    *own = OwnLook { look: look_for_race(&race), body: load_body(&default_body_path(), name), class: class_look(&class) };
}

/// Creates the camera and light that draw the character preview into its image.
fn setup_preview(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut preview: ResMut<Preview>) {
    let mut image = Image::new_target_texture(PREVIEW_PX, PREVIEW_PX, TextureFormat::Rgba8UnormSrgb, None);
    image.asset_usage = RenderAssetUsages::default();
    preview.image = images.add(image);
    let layer = RenderLayers::layer(PREVIEW_LAYER);
    commands.spawn((
        Camera3d::default(),
        Camera { order: -1, is_active: false, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::Image(preview.image.clone().into()),
        // About 1.6 units of height in view: a troll (1.4) fits, a gnome looks small.
        Transform::from_translation(PREVIEW_AT + Vec3::new(0.0, 0.8, 1.9)).looking_at(PREVIEW_AT + Vec3::Y * 0.72, Vec3::Y),
        layer.clone(),
        PreviewCamera,
    ));
    commands.spawn((
        DirectionalLight { illuminance: 8_000.0, ..default() },
        Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        layer,
    ));
}

#[derive(Component)]
struct PreviewCamera;

/// Shows the selected race and Body on the new-character screen, slowly turning.
fn update_preview(
    mut commands: Commands,
    time: Res<Time>,
    server: Res<AssetServer>,
    mut preview: ResMut<Preview>,
    session: Option<NonSend<Session>>,
    mut camera: Single<&mut Camera, With<PreviewCamera>>,
    mut bodies: Query<(Entity, &mut Transform), With<PreviewBody>>,
) {
    let creating = session.as_ref().is_some_and(|s| *s.login.state() == LoginState::Creating);
    camera.is_active = creating;
    let race = session.as_ref().and_then(|s| s.login.newchar.race.clone()).unwrap_or_default();
    let want = creating.then(|| (race.clone(), preview.body));
    if preview.shown != want {
        for (e, _) in &bodies {
            commands.entity(e).despawn();
        }
        if want.is_some() {
            let at = Transform::from_translation(PREVIEW_AT);
            let load = |f: &str| server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{f}")));
            let files = character_files(preview.body);
            let extra = (PreviewBody, RenderLayers::layer(PREVIEW_LAYER));
            spawn_character(&mut commands, load, files, &look_for_race(&race), at, Visibility::Inherited, extra);
        }
        preview.shown = want;
    }
    for (_, mut t) in &mut bodies {
        t.rotate_y(time.delta_secs() * 0.6);
    }
}

fn clear_preview(mut commands: Commands, mut preview: ResMut<Preview>, bodies: Query<Entity, With<PreviewBody>>) {
    for e in &bodies {
        commands.entity(e).despawn();
    }
    preview.shown = None;
}

mod beasts;
mod dress;
mod worn;

use beasts::*;
pub use dress::Outfit;
use dress::*;
use worn::*;

#[cfg(test)]
mod tests;
