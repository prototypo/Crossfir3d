//! What a `person` creature wears or carries beyond the shared body, and its bone shapes.

use super::*;

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
    /// A creature's body shape, and its horns or pointed ears (of this colour).
    pub build: Option<crossfir3d::models::Build>,
    pub horns: bool,
    pub ears: Option<[f32; 3]>,
}

/// A plain, rough material of this colour.
fn paint(materials: &mut Assets<StandardMaterial>, [r, g, b]: [f32; 3]) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial { base_color: Color::srgb(r, g, b), perceptual_roughness: 0.9, ..default() })
}

/// Hangs the skirt (if the outfit has one) from the waist, as wide as the body.
fn put_skirt(commands: &mut Commands, root: Entity, outfit: &Outfit, skirt: &Handle<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let Some(colour) = outfit.dress else { return };
    let k = outfit.scale;
    let t = Transform::from_xyz(0.0, 0.31 * k, 0.0).with_scale(Vec3::new(k * outfit.wide, k, k * outfit.wide));
    commands.entity(root).with_child((Mesh3d(skirt.clone()), MeshMaterial3d(paint(materials, colour)), t));
}

/// The shape of each named bone for this outfit: its build, and its head `head` times as big.
fn bone_shapes(outfit: &Outfit) -> HashMap<&'static str, BoneShape> {
    let mut shapes: HashMap<&str, BoneShape> = outfit.build.map_or_else(Vec::new, |b| b.bones()).into_iter().map(|(n, s, bend)| (n, BoneShape(Vec3::from_array(s), bend))).collect();
    shapes.entry("Head").or_insert(BoneShape(Vec3::ONE, 0.0)).0 *= outfit.head;
    shapes
}

/// Reshapes the body: widens its wide parts and gives the other bones their BoneShape.
fn reshape_body(commands: &mut Commands, body: &Skeleton, root: Entity, outfit: &Outfit) {
    let wide = outfit.wide;
    let reshaped = bone_shapes(outfit);
    for e in body.children.iter_descendants(root) {
        let shape = body.names.get(e).ok().and_then(|n| reshaped.get(n.as_str()));
        if body.parts.contains(e) && wide != 1.0 {
            commands.entity(e).queue_silenced(move |mut e: EntityWorldMut| {
                if let Some(mut t) = e.get_mut::<Transform>() {
                    t.scale *= Vec3::new(wide, 1.0, wide);
                }
            });
        } else if let Some(&shape) = shape.filter(|s| s.0 != Vec3::ONE || s.1 != 0.0) {
            commands.entity(e).try_insert(shape);
        }
    }
}

/// Puts the outfit's horns or pointed ears (if any) on the head bone.
fn put_headwear(commands: &mut Commands, body: &Skeleton, root: Entity, outfit: &Outfit, stock: &mut Stock) {
    let wear = match (outfit.horns, outfit.ears) {
        (true, _) => Some(Headwear::Horns),
        (_, Some(c)) => Some(Headwear::Ears(c)),
        _ => None,
    };
    if let Some(wear) = wear
        && let Some(head) = body.find(root, "Head")
    {
        put_on_head(commands, head, wear, outfit.scale / bone_scale(head, &body.parents, &body.parts), &stock.leather, &mut stock.meshes, &mut stock.materials, ());
    }
}

/// Puts a broom in the hand: a stick along the bone's +Y with the straw at the far end,
/// `k` times its size for a human.
fn put_broom(commands: &mut Commands, hand: Entity, k: f32, [stick, straw]: [&Handle<Mesh>; 2], materials: &mut Assets<StandardMaterial>) {
    const STRAW: [f32; 3] = [0.85, 0.7, 0.35];
    const HANDLE: [f32; 3] = [0.45, 0.3, 0.18];
    let at = |y: f32| Transform::from_xyz(0.0, y * k, 0.0).with_scale(Vec3::splat(k));
    let handle = (Mesh3d(stick.clone()), MeshMaterial3d(paint(materials, HANDLE)), at(HANDLE_LEN / 2.0 - 0.1));
    let bristles = (Mesh3d(straw.clone()), MeshMaterial3d(paint(materials, STRAW)), at(HANDLE_LEN - 0.05));
    commands.entity(hand).queue_silenced(move |mut e: EntityWorldMut| {
        e.with_child(handle).with_child(bristles);
    });
}

const HANDLE_LEN: f32 = 0.85;

/// Adds each Outfit once the body's bones exist. Sizes are for a human (scale 1, about
/// 1.08 tall): the skirt hangs from the waist nearly to the floor; the broom runs along
/// the hand bone's +Y, like a sword's blade, with the straw at the far end.
pub(super) fn dress_people(mut commands: Commands, people: Query<(Entity, &Outfit)>, body: Skeleton, mut stock: Stock, mut shapes: Local<Option<[Handle<Mesh>; 3]>>) {
    let [skirt, stick, straw] = shapes
        .get_or_insert_with(|| {
            [
                // Wide at the hem, so the legs stay inside it as they walk.
                stock.meshes.add(ConicalFrustum { radius_top: 0.12, radius_bottom: 0.34, height: 0.52 }),
                stock.meshes.add(Cylinder::new(0.015, HANDLE_LEN)),
                stock.meshes.add(ConicalFrustum { radius_top: 0.08, radius_bottom: 0.025, height: 0.2 }),
            ]
        })
        .clone();
    for (root, outfit) in &people {
        // The body scenes spawn a little later than the character: try again next frame.
        let Some(hand) = body.find(root, "hand_r") else { continue };
        put_skirt(&mut commands, root, outfit, &skirt, &mut stock.materials);
        reshape_body(&mut commands, &body, root, outfit);
        put_headwear(&mut commands, &body, root, outfit, &mut stock);
        if outfit.broom {
            put_broom(&mut commands, hand, outfit.scale / bone_scale(hand, &body.parents, &body.parts), [&stick, &straw], &mut stock.materials);
        }
        commands.entity(root).remove::<Outfit>();
    }
}

/// A bone drawn at this scale (a child's big head, a troll's big arms) and bent forward
/// this far (radians, about its own X).
#[derive(Component, Clone, Copy)]
pub(super) struct BoneShape(Vec3, f32);

/// The animations set every bone's scale and turn each frame: reshape them again after.
pub(super) fn hold_bone_shapes(mut bones: Query<(&mut Transform, &BoneShape)>) {
    for (mut t, BoneShape(scale, bend)) in &mut bones {
        t.scale = *scale;
        t.rotation *= Quat::from_rotation_x(*bend);
    }
}
