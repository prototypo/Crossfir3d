//! The 3D people: the player's character, other players, and the new-character preview.
//! One body (with hair and an outfit) for everyone, sized and coloured per race, moved
//! by the Universal Animation Library clips. All the files share one skeleton with the
//! same bone names, so a clip drives every part without retargeting.

use bevy::animation::{AnimatedBy, AnimationTargetId};
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::RenderTarget;
use bevy::gltf::{Gltf, GltfAssetLabel};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot, WorldInstanceReady};
use maccrossfire::login::LoginState;
use maccrossfire::anim::{clip_name, looping, Anim, AnimEvent, AnimState};
use maccrossfire::held::{held, helmet, worn, Garb, Hand};
use maccrossfire::races::{default_body_path, head_triangles, load_body, look_for_race, tints, Body, Look};
use std::collections::HashMap;

use bevy::mesh::skinning::SkinnedMesh;
use bevy::mesh::{Indices, VertexAttributeValues};

use crate::input::Facing;
use crate::world::{Glide, ModelTint, SkinTint};
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
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerAnimEvents>()
            .init_resource::<OwnLook>()
            .init_resource::<Preview>()
            .add_observer(bind_bones)
            .add_systems(Startup, (load_anims, setup_preview))
            .add_systems(Update, (build_graph, animate, animate_beasts, hold_items, wear_armour, wear_helmet, dress_people, freeze_statues))
            .add_systems(PostUpdate, hold_head_size.after(bevy::app::AnimationSystems).before(bevy::transform::TransformSystems::Propagate))
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
}

impl Default for OwnLook {
    fn default() -> Self {
        OwnLook { look: look_for_race("human"), body: Body::Regular }
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
struct OutfitPart;

/// What the own character wears now (see `maccrossfire::held::worn`).
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
        (Body::Regular, Garb::Peasant) => "outfits/Male_Peasant.gltf",
        (Body::Regular, Garb::Ranger) => "outfits/Male_Ranger.gltf",
        (Body::Regular, Garb::Metal) => "outfits/Male_Ranger_Metal.gltf",
        (Body::Other, Garb::Peasant) => "outfits/Female_Peasant.gltf",
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
    commands.spawn((WorldAssetRoot(scene), inner, Visibility::Hidden, ModelTint(Color::srgb(r, g, b)), ChildOf(root)));
    root
}

/// Clip names (after any "Armature|" prefix) to stand and to move with, best first.
const IDLE_CLIPS: [&str; 4] = ["Idle", "Flying_Idle", "Idle_2", "Flying"];
const WALK_CLIPS: [&str; 5] = ["Walk", "Fast_Flying", "Flying", "Gallop", "Run"];

/// A beast file's graph of its idle and walk clips, once its glTF has loaded.
type BeastClips = (Handle<AnimationGraph>, AnimationNodeIndex, AnimationNodeIndex);

/// Plays each beast's idle or walk clip on its own animation player, and turns it to
/// face the way it moves.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn animate_beasts(
    mut commands: Commands,
    server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut files: Local<HashMap<String, (Handle<Gltf>, Option<BeastClips>)>>,
    mut beasts: Query<(Entity, &mut Beast, &mut Transform, Option<&Glide>)>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer, Has<AnimationGraphHandle>)>,
) {
    for (root, mut beast, mut t, glide) in &mut beasts {
        let (gltf, clips) = files
            .entry(beast.file.clone())
            .or_insert_with(|| (server.load(format!("models/{}", beast.file)), None));
        if clips.is_none()
            && let Some(g) = gltfs.get(gltf.id())
        {
            let find = |wanted: &[&str]| {
                wanted.iter().find_map(|w| g.named_animations.iter().find(|(n, _)| n.rsplit('|').next() == Some(*w)).map(|(_, c)| c.clone()))
            };
            let (Some(idle), Some(walk)) = (find(&IDLE_CLIPS), find(&WALK_CLIPS)) else { continue };
            let (graph, nodes) = AnimationGraph::from_clips([idle, walk]);
            *clips = Some((graphs.add(graph), nodes[0], nodes[1]));
        }
        let Some((graph, idle, walk)) = clips.clone() else { continue };
        let Some(found) = children.iter_descendants(root).find(|e| players.contains(*e)) else { continue };
        let Ok((player_e, mut player, has_graph)) = players.get_mut(found) else { continue };
        if !has_graph {
            commands.entity(player_e).try_insert(AnimationGraphHandle(graph));
        }
        let step = glide.map_or(Vec3::ZERO, |g| g.target - t.translation);
        let walking = step.length() > 0.02;
        if walking {
            t.rotation = Quat::from_rotation_y(step.x.atan2(step.z));
        }
        if beast.walking != Some(walking) {
            beast.walking = Some(walking);
            player.stop_all();
            player.play(if walking { walk } else { idle }).repeat();
        }
    }
}

fn load_anims(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(Anims { gltf: server.load(ANIMS_FILE), graph: None });
}

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

#[allow(clippy::too_many_arguments)]
/// Gives every bone of a spawned body part the id the clips use (its name path from
/// `Armature`), driven by the character's animation player.
fn bind_bones(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    parts: Query<&PartOf>,
    previews: Query<(), With<PreviewBody>>,
    children: Query<&Children>,
    names: Query<&Name>,
    skinned: Query<(&Mesh3d, &SkinnedMesh)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut heads: Local<HashMap<AssetId<Mesh>, Handle<Mesh>>>,
) {
    let Ok(PartOf(root)) = parts.get(ready.entity) else { return };
    // The free body is a whole body; under an outfit only its head may show.
    for e in children.iter_descendants(ready.entity) {
        let Ok((mesh, skin)) = skinned.get(e) else { continue };
        // Mesh parts are named after their material: "<mesh>.MI_Superhero_Male".
        if !names.get(e).is_ok_and(|n| n.as_str().contains("MI_Superhero")) {
            continue;
        }
        let head = match heads.get(&mesh.0.id()) {
            Some(h) => h.clone(),
            None => {
                let is_head: Vec<bool> =
                    skin.joints.iter().map(|j| names.get(*j).is_ok_and(|n| matches!(n.as_str(), "Head" | "neck_01"))).collect();
                let Some(head) = meshes.get(&mesh.0).and_then(|m| head_only(m, &is_head)) else { continue };
                let h = meshes.add(head);
                heads.insert(mesh.0.id(), h.clone());
                h
            }
        };
        commands.entity(e).try_insert(Mesh3d(head));
    }
    if previews.contains(*root) {
        for e in children.iter_descendants(ready.entity) {
            commands.entity(e).try_insert(RenderLayers::layer(PREVIEW_LAYER));
        }
    }
    let armature = children.iter_descendants(ready.entity).find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "Armature"));
    let Some(armature) = armature else { return };
    let mut stack = vec![(armature, vec![names.get(armature).unwrap().clone()])];
    while let Some((e, path)) = stack.pop() {
        commands.entity(e).try_insert((AnimationTargetId::from_names(path.iter()), AnimatedBy(*root)));
        for &c in children.get(e).into_iter().flatten() {
            if let Ok(n) = names.get(c) {
                stack.push((c, [path.clone(), vec![n.clone()]].concat()));
            }
        }
    }
}

fn head_only(mesh: &Mesh, is_head: &[bool]) -> Option<Mesh> {
    let Some(VertexAttributeValues::Uint16x4(joints)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) else { return None };
    let Some(VertexAttributeValues::Float32x4(weights)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) else { return None };
    let indices: Vec<u32> = mesh.indices()?.iter().map(|i| i as u32).collect();
    let mut head = mesh.clone();
    head.insert_indices(Indices::U32(head_triangles(&indices, joints, weights, is_head)));
    Some(head)
}

/// Puts the applied weapon and shield (see `maccrossfire::held`) in the own character's
/// hands, on the hand bones, so they follow the animations.
#[allow(clippy::too_many_arguments)]
fn hold_items(
    mut commands: Commands,
    inv: Res<Inv>,
    server: Res<AssetServer>,
    own: Query<(Entity, Option<&Holding>), With<OwnCharacter>>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<(&ChildOf, &Transform)>,
    parts: Query<(), With<PartOf>>,
    held_models: Query<Entity, With<HeldModel>>,
) {
    let Ok((root, holding)) = own.single() else { return };
    let want = held(&inv.0.carried());
    if holding.is_some_and(|h| h.0 == want) {
        return;
    }
    let bone = |name: &str| children.iter_descendants(root).find(|e| names.get(*e).is_ok_and(|n| n.as_str() == name));
    // The body scenes spawn a little later than the character: try again next frame.
    let (Some(right), Some(left)) = (bone("hand_r"), bone("hand_l")) else { return };
    // The character may be despawned (and remade) this frame: every command here is
    // skipped quietly if its entity is gone.
    for e in children.iter_descendants(root).filter(|e| held_models.contains(*e)) {
        commands.entity(e).try_despawn();
    }
    for (file, hand) in &want {
        let hand = if *hand == Hand::Right { right } else { left };
        let t = Transform::from_scale(Vec3::splat(WEAPON_SCALE / bone_scale(hand, &parents, &parts)));
        let scene = server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{file}")));
        commands.entity(hand).queue_silenced(move |mut e: EntityWorldMut| {
            e.with_child((WorldAssetRoot(scene), t, HeldModel));
        });
    }
    commands.entity(root).try_insert(Holding(want));
}

/// Dresses the own character in the outfit for its applied body armour: the outfit part
/// is swapped for the other file and tint (its bones bind again when it spawns).
fn wear_armour(
    mut commands: Commands,
    inv: Res<Inv>,
    own_look: Res<OwnLook>,
    server: Res<AssetServer>,
    own: Query<(Entity, Option<&Wearing>), With<OwnCharacter>>,
    outfits: Query<(Entity, &ChildOf, &Transform, &Visibility, &SkinTint), With<OutfitPart>>,
) {
    let Ok((root, wearing)) = own.single() else { return };
    let want = Wearing(worn(&inv.0.carried()));
    // A new character starts in its own clothes: nothing to swap.
    if wearing.map_or(want.0.is_none(), |w| *w == want) {
        if wearing.is_none() {
            commands.entity(root).try_insert(want);
        }
        return;
    }
    let Some((old, _, &t, &vis, skin)) = outfits.iter().find(|o| o.1.parent() == root) else { return };
    let (garb, tint) = want.0.unwrap_or((Garb::Peasant, tints(&own_look.look).clothes));
    let file = outfit_file(own_look.body, garb);
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{file}")));
    let [r, g, b] = tint;
    commands.entity(old).try_despawn();
    commands.spawn((WorldAssetRoot(scene), t, vis, ModelTint(Color::srgb(r, g, b)), SkinTint(skin.0), PartOf(root), ChildOf(root), OutfitPart));
    commands.entity(root).try_insert(want);
}

/// The own character's applied helmet colour (see `maccrossfire::held::helmet`).
#[derive(Component, PartialEq)]
struct Helmeted(Option<[f32; 3]>);

/// A helmet shape on the own character.
#[derive(Component)]
struct HelmetShape;

/// Puts the applied helmet on the own character: a metal cap on the head bone, so it
/// follows the head.
#[allow(clippy::too_many_arguments)]
fn wear_helmet(
    mut commands: Commands,
    inv: Res<Inv>,
    own_look: Res<OwnLook>,
    own: Query<(Entity, Option<&Helmeted>), With<OwnCharacter>>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<(&ChildOf, &Transform)>,
    parts: Query<(), With<PartOf>>,
    worn_now: Query<Entity, With<HelmetShape>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cap: Local<Option<Handle<Mesh>>>,
) {
    let Ok((root, helmeted)) = own.single() else { return };
    let want = Helmeted(helmet(&inv.0.carried()));
    if helmeted.map_or(want.0.is_none(), |h| *h == want) {
        return;
    }
    // The body scenes spawn a little later than the character: try again next frame.
    let Some(head) = children.iter_descendants(root).find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "Head")) else { return };
    for e in children.iter_descendants(root).filter(|e| worn_now.contains(*e)) {
        commands.entity(e).try_despawn();
    }
    if let Some([r, g, b]) = want.0 {
        let cap = cap.get_or_insert_with(|| meshes.add(Sphere::new(0.1))).clone();
        let metal = materials.add(StandardMaterial { base_color: Color::srgb(r, g, b), perceptual_roughness: 0.6, metallic: 0.6, ..default() });
        let k = own_look.look.scale / bone_scale(head, &parents, &parts);
        let t = Transform::from_xyz(0.0, 0.15 * k, 0.0).with_scale(Vec3::new(1.15, 0.9, 1.25) * k);
        let cap = (Mesh3d(cap), MeshMaterial3d(metal), t, HelmetShape);
        commands.entity(head).queue_silenced(move |mut e: EntityWorldMut| {
            e.with_child(cap);
        });
    }
    commands.entity(root).try_insert(want);
}

/// A bone's own scale inside its body part (the armature may be scaled).
fn bone_scale(bone: Entity, parents: &Query<(&ChildOf, &Transform)>, parts: &Query<(), With<PartOf>>) -> f32 {
    let mut scale = 1.0;
    let mut e = bone;
    while let Ok((ChildOf(parent), t)) = parents.get(e) {
        if parts.contains(e) {
            break;
        }
        scale *= t.scale.x;
        e = *parent;
    }
    scale
}

/// What a `person` creature wears or carries beyond the shared body: a long skirt of
/// this colour (the free outfits have none), a broom in the right hand, metal armour
/// (the outfit file, chosen at spawn); and its shape: `wide` times as broad, its head
/// `head` times as big.
#[derive(Component, Clone, Copy)]
pub struct Outfit {
    pub scale: f32,
    pub dress: Option<[f32; 3]>,
    pub broom: bool,
    pub armour: bool,
    pub wide: f32,
    pub head: f32,
}

/// Adds each Outfit once the body's bones exist. Sizes are for a human (scale 1, about
/// 1.08 tall): the skirt hangs from the waist nearly to the floor; the broom runs along
/// the hand bone's +Y, like a sword's blade, with the straw at the far end.
#[allow(clippy::too_many_arguments)]
fn dress_people(
    mut commands: Commands,
    people: Query<(Entity, &Outfit)>,
    children: Query<&Children>,
    names: Query<&Name>,
    parents: Query<(&ChildOf, &Transform)>,
    parts: Query<(), With<PartOf>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shapes: Local<Option<[Handle<Mesh>; 3]>>,
) {
    const STRAW: [f32; 3] = [0.85, 0.7, 0.35];
    const HANDLE: [f32; 3] = [0.45, 0.3, 0.18];
    const HANDLE_LEN: f32 = 0.85;
    let [skirt, stick, straw] = shapes
        .get_or_insert_with(|| {
            [
                // Wide at the hem, so the legs stay inside it as they walk.
                meshes.add(ConicalFrustum { radius_top: 0.12, radius_bottom: 0.34, height: 0.52 }),
                meshes.add(Cylinder::new(0.015, HANDLE_LEN)),
                meshes.add(ConicalFrustum { radius_top: 0.08, radius_bottom: 0.025, height: 0.2 }),
            ]
        })
        .clone();
    let mut paint = |[r, g, b]: [f32; 3]| materials.add(StandardMaterial { base_color: Color::srgb(r, g, b), perceptual_roughness: 0.9, ..default() });
    for (root, outfit) in &people {
        // The body scenes spawn a little later than the character: try again next frame.
        let Some(hand) = children.iter_descendants(root).find(|e| names.get(*e).is_ok_and(|n| n.as_str() == "hand_r")) else { continue };
        let k = outfit.scale;
        if let Some(colour) = outfit.dress {
            let t = Transform::from_xyz(0.0, 0.31 * k, 0.0).with_scale(Vec3::new(k * outfit.wide, k, k * outfit.wide));
            commands.entity(root).with_child((Mesh3d(skirt.clone()), MeshMaterial3d(paint(colour)), t));
        }
        let (wide, head) = (outfit.wide, outfit.head);
        for e in children.iter_descendants(root) {
            let is_head = names.get(e).is_ok_and(|n| n.as_str() == "Head");
            if parts.contains(e) && wide != 1.0 {
                commands.entity(e).queue_silenced(move |mut e: EntityWorldMut| {
                    if let Some(mut t) = e.get_mut::<Transform>() {
                        t.scale *= Vec3::new(wide, 1.0, wide);
                    }
                });
            } else if is_head && head != 1.0 {
                commands.entity(e).try_insert(BigHead(head));
            }
        }
        if outfit.broom {
            let k = k / bone_scale(hand, &parents, &parts);
            let at = |y: f32| Transform::from_xyz(0.0, y * k, 0.0).with_scale(Vec3::splat(k));
            let handle = (Mesh3d(stick.clone()), MeshMaterial3d(paint(HANDLE)), at(HANDLE_LEN / 2.0 - 0.1));
            let bristles = (Mesh3d(straw.clone()), MeshMaterial3d(paint(STRAW)), at(HANDLE_LEN - 0.05));
            commands.entity(hand).queue_silenced(move |mut e: EntityWorldMut| {
                e.with_child(handle).with_child(bristles);
            });
        }
        commands.entity(root).remove::<Outfit>();
    }
}

/// A head bone drawn this many times as big (a child's head is big for its body).
#[derive(Component)]
struct BigHead(f32);

/// The animations set every bone's scale each frame: set the big heads' again after them.
fn hold_head_size(mut heads: Query<(&mut Transform, &BigHead)>) {
    for (mut t, BigHead(k)) in &mut heads {
        t.scale = Vec3::splat(*k);
    }
}

/// Facing 1 = north .. 8 = northwest, clockwise. The model looks along +Z unturned.
fn facing_rotation(facing: u8) -> Quat {
    let clockwise = (facing.clamp(1, 8) - 1) as f32 * std::f32::consts::FRAC_PI_4;
    Quat::from_rotation_y(std::f32::consts::PI - clockwise)
}

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
    let listed = session.login.characters().iter().find(|c| c.name == name).map(|c| c.race.clone());
    let race = listed.or_else(|| session.login.newchar.race.clone()).unwrap_or_default();
    *own = OwnLook { look: look_for_race(&race), body: load_body(&default_body_path(), name) };
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facing_north_looks_down_minus_z() {
        let f = |d| facing_rotation(d) * Vec3::Z;
        assert!(f(1).abs_diff_eq(Vec3::NEG_Z, 1e-5));
        assert!(f(3).abs_diff_eq(Vec3::X, 1e-5));
        assert!(f(5).abs_diff_eq(Vec3::Z, 1e-5));
    }
}
