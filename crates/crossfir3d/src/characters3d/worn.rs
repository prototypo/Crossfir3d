//! The own character's held items, armour and helmet, and other players' class gear.

use super::*;

/// What `hold_items` reads about the own character.
#[derive(SystemParam)]
pub(super) struct Holder<'w, 's> {
    inv: Res<'w, Inv>,
    own_look: Res<'w, OwnLook>,
    own: Query<'w, 's, (Entity, Option<&'static Holding>), With<OwnCharacter>>,
    held_models: Query<'w, 's, Entity, With<HeldModel>>,
    server: Res<'w, AssetServer>,
}

/// Puts what the class carries in each of the `empty` hands (side and bone).
fn hold_class_items(commands: &mut Commands, holder: &Holder, empty: impl Iterator<Item = (Hand, Entity)>, skeleton: &Skeleton, stock: &mut Stock) {
    let Some(class) = holder.own_look.class else { return };
    for (side, bone) in empty {
        if let Some(carry) = if side == Hand::Right { class.right } else { class.left } {
            let k = holder.own_look.look.scale / bone_scale(bone, &skeleton.parents, &skeleton.parts);
            put_in_hand(stock.hand_kit(commands, &holder.server), bone, side, carry, k);
        }
    }
}

/// Puts the applied weapon and shield (see `crossfir3d::held`) in the own character's
/// hands, on the hand bones, so they follow the animations.
pub(super) fn hold_items(mut commands: Commands, holder: Holder, skeleton: Skeleton, mut stock: Stock) {
    let Ok((root, holding)) = holder.own.single() else { return };
    let want = held(&holder.inv.0.carried());
    if holding.is_some_and(|h| h.0 == want) {
        return;
    }
    // The body scenes spawn a little later than the character: try again next frame.
    let (Some(right), Some(left)) = (skeleton.find(root, "hand_r"), skeleton.find(root, "hand_l")) else { return };
    // The character may be despawned (and remade) this frame: every command here is
    // skipped quietly if its entity is gone.
    for e in skeleton.children.iter_descendants(root).filter(|e| holder.held_models.contains(*e)) {
        commands.entity(e).try_despawn();
    }
    for (file, hand) in &want {
        let (side, hand) = (*hand, if *hand == Hand::Right { right } else { left });
        put_in_hand(stock.hand_kit(&mut commands, &holder.server), hand, side, Carry::Model(file), 1.0 / bone_scale(hand, &skeleton.parents, &skeleton.parts));
    }
    // A hand holding nothing holds what the class carries.
    let empty = [(Hand::Right, right), (Hand::Left, left)].into_iter().filter(|(side, _)| !want.iter().any(|(_, h)| h == side));
    hold_class_items(&mut commands, &holder, empty, &skeleton, &mut stock);
    commands.entity(root).try_insert(Holding(want));
}

/// What `wear_armour` reads about the own character's look.
#[derive(SystemParam)]
pub(super) struct Outfitter<'w> {
    inv: Res<'w, Inv>,
    own_look: Res<'w, OwnLook>,
    server: Res<'w, AssetServer>,
}

/// Dresses the own character in the outfit for its applied body armour: the outfit part
/// is swapped for the other file and tint (its bones bind again when it spawns).
pub(super) fn wear_armour(
    mut commands: Commands,
    outfitter: Outfitter,
    own: Query<(Entity, Option<&Wearing>), With<OwnCharacter>>,
    outfits: Query<(Entity, &ChildOf, &Transform, &Visibility, &SkinTint), With<OutfitPart>>,
) {
    let Ok((root, wearing)) = own.single() else { return };
    let want = Wearing(worn(&outfitter.inv.0.carried()));
    // A new character starts in its own clothes: nothing to swap.
    if wearing.map_or(want.0.is_none(), |w| *w == want) {
        if wearing.is_none() {
            commands.entity(root).try_insert(want);
        }
        return;
    }
    let Some((old, _, &t, &vis, skin)) = outfits.iter().find(|o| o.1.parent() == root) else { return };
    let class = outfitter.own_look.class.map(|c| (c.garb, c.tint));
    let (garb, tint) = want.0.or(class).unwrap_or((Garb::Peasant, tints(&outfitter.own_look.look).clothes));
    let file = outfit_file(outfitter.own_look.body, garb);
    let scene = outfitter.server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{file}")));
    let [r, g, b] = tint;
    commands.entity(old).try_despawn();
    commands.spawn((WorldAssetRoot(scene), t, vis, ModelTint(Color::srgb(r, g, b)), SkinTint(skin.0), PartOf(root), ChildOf(root), OutfitPart));
    // What was built on the old outfit went with it: build it again on the new one.
    if let Some(dress) = crate::robe::Dress::of(outfitter.own_look.class, want.0.is_some()) {
        commands.entity(root).try_insert(dress);
    }
    commands.entity(root).try_insert(want);
}

/// The own character's applied helmet colour (see `crossfir3d::held::helmet`).
#[derive(Component, PartialEq)]
struct Helmeted(Option<[f32; 3]>);

/// A helmet shape (or class headwear) on the own character.
#[derive(Component, Clone)]
struct HelmetShape;

/// What `wear_helmet` reads about the own character.
#[derive(SystemParam)]
pub(super) struct Wearer<'w, 's> {
    inv: Res<'w, Inv>,
    own_look: Res<'w, OwnLook>,
    own: Query<'w, 's, (Entity, Option<&'static Helmeted>), With<OwnCharacter>>,
    worn_now: Query<'w, 's, Entity, With<HelmetShape>>,
}

/// A metal cap of this colour, scaled by `k`, with the cap mesh made once into `cap`.
fn metal_cap([r, g, b]: [f32; 3], k: f32, cap: &mut Option<Handle<Mesh>>, stock: &mut Stock) -> impl Bundle {
    let mesh = cap.get_or_insert_with(|| stock.meshes.add(Sphere::new(0.1))).clone();
    let metal = stock.materials.add(StandardMaterial { base_color: Color::srgb(r, g, b), perceptual_roughness: 0.6, metallic: 0.6, ..default() });
    let t = Transform::from_xyz(0.0, 0.15 * k, 0.0).with_scale(Vec3::new(1.15, 0.9, 1.25) * k);
    (Mesh3d(mesh), MeshMaterial3d(metal), t, HelmetShape)
}

/// Puts the applied helmet on the own character: a metal cap on the head bone, so it
/// follows the head.
pub(super) fn wear_helmet(mut commands: Commands, wearer: Wearer, skeleton: Skeleton, mut stock: Stock, mut cap: Local<Option<Handle<Mesh>>>) {
    let Ok((root, helmeted)) = wearer.own.single() else { return };
    let want = Helmeted(helmet(&wearer.inv.0.carried()));
    // With no helmet the class's headwear shows, so the first look is always made.
    if helmeted.is_some_and(|h| *h == want) {
        return;
    }
    // The body scenes spawn a little later than the character: try again next frame.
    let Some(head) = skeleton.find(root, "Head") else { return };
    for e in skeleton.children.iter_descendants(root).filter(|e| wearer.worn_now.contains(*e)) {
        commands.entity(e).try_despawn();
    }
    let k = wearer.own_look.look.scale / bone_scale(head, &skeleton.parents, &skeleton.parts);
    if let Some(colour) = want.0 {
        let cap = metal_cap(colour, k, &mut cap, &mut stock);
        commands.entity(head).queue_silenced(move |mut e: EntityWorldMut| {
            e.with_child(cap);
        });
    } else if let Some(wear) = wearer.own_look.class.and_then(|c| c.head) {
        crate::hat::put_on(stock.hat_kit(&mut commands), head, wear, k, HelmetShape);
    }
    commands.entity(root).try_insert(want);
}
/// Puts a class's starting hat and hand items on a character once its bones exist.
pub(super) fn gear_up(mut commands: Commands, server: Res<AssetServer>, geared: Query<(Entity, &ClassGear)>, skeleton: Skeleton, mut stock: Stock) {
    for (root, ClassGear(class, scale)) in &geared {
        // The body scenes spawn a little later than the character: try again next frame.
        let (Some(head), Some(right), Some(left)) = (skeleton.find(root, "Head"), skeleton.find(root, "hand_r"), skeleton.find(root, "hand_l")) else { continue };
        if let Some(wear) = class.head {
            crate::hat::put_on(stock.hat_kit(&mut commands), head, wear, scale / bone_scale(head, &skeleton.parents, &skeleton.parts), ());
        }
        for (carry, side, bone) in [(class.right, Hand::Right, right), (class.left, Hand::Left, left)] {
            if let Some(carry) = carry {
                put_in_hand(stock.hand_kit(&mut commands, &server), bone, side, carry, scale / bone_scale(bone, &skeleton.parents, &skeleton.parts));
            }
        }
        commands.entity(root).remove::<ClassGear>();
    }
}

/// A material of plain colour, glowing when `glow`.
pub(super) fn plain(materials: &mut Assets<StandardMaterial>, [r, g, b]: [f32; 3], glow: bool) -> Handle<StandardMaterial> {
    let color = Color::srgb(r, g, b);
    let emissive = if glow { LinearRgba::from(color) * 4.0 } else { LinearRgba::BLACK };
    materials.add(StandardMaterial { base_color: color, emissive, perceptual_roughness: 0.8, ..default() })
}

/// Out of the thumb side of the fist, leaning toward the fingers by `lean` (radians):
/// with the arm hanging, a positive lean points forward and down, a negative one up.
fn out_of_fist(fist: Vec3, lean: f32) -> (Vec3, Quat) {
    (fist, Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - lean))
}

/// How a hand holds a model file, by what the file is named.
fn model_grip(file: &str, palm: f32, fist: Vec3) -> (Vec3, Quat) {
    if file.contains("Shield") {
        // Strapped to the back of the forearm, face out, top up the arm.
        let out = Vec3::X * -palm;
        (Vec3::new(0.07 * -palm, -0.08, 0.0), Quat::from_mat3(&Mat3::from_cols(Vec3::NEG_Y.cross(out), Vec3::NEG_Y, out)))
    } else if file.contains("Bow") {
        out_of_fist(fist, 0.35)
    } else if file.contains("Spear") || file.contains("Scythe") {
        out_of_fist(fist, -0.9)
    } else if file.contains("Axe_Double") {
        // This model's handle runs up x = 0.67, off its origin.
        let (at, turn) = out_of_fist(fist, 0.6);
        (at - turn * Vec3::X * 0.67 * WEAPON_SCALE, turn)
    } else {
        out_of_fist(fist, 0.6)
    }
}

/// How a hand holds a thing: where its grip sits and how it turns. The hand bone's +Y runs
/// along the fingers and +Z to the thumb; the palm faces -X on the right hand, +X on the left.
/// A thing's own +Y is its blade, shaft or top; +Z is a shield's face.
pub(super) fn grip(side: Hand, carry: &Carry) -> (Vec3, Quat) {
    let palm = if side == Hand::Right { -1.0 } else { 1.0 };
    // In the curled fingers, a little toward the palm.
    let fist = Vec3::new(0.02 * palm, 0.085, 0.0);
    match carry {
        Carry::Model(file) => model_grip(file, palm, fist),
        // Tilted out, away from the body, so its top stands clear of a hat's brim.
        Carry::Staff(_) => {
            let (at, turn) = out_of_fist(fist, -0.9);
            (at, Quat::from_rotation_y(-palm * 0.3) * turn)
        }
        Carry::Potion => out_of_fist(fist, -0.9),
    }
}

/// The tools `put_in_hand` spawns with.
pub(super) struct HandKit<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub server: &'a AssetServer,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
}

/// Puts a carried thing in a hand bone: a model file, or a built staff or potion.
/// `k` is the bone's scale for a human.
pub(super) fn put_in_hand(kit: HandKit, hand: Entity, side: Hand, carry: Carry, k: f32) {
    let HandKit { commands, server, meshes, materials } = kit;
    let (at_fist, turn) = grip(side, &carry);
    // `along`: how far along the thing's own +Y from the grip.
    let at = |along: f32| Transform::from_translation((at_fist + turn * Vec3::Y * along) * k).with_rotation(turn).with_scale(Vec3::splat(k));
    let shapes: Vec<(Mesh, Transform, [f32; 3], bool)> = match carry {
        Carry::Model(file) => {
            let t = at(0.0).with_scale(Vec3::splat(WEAPON_SCALE * k));
            let scene = server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/{file}")));
            commands.entity(hand).queue_silenced(move |mut e: EntityWorldMut| {
                e.with_child((WorldAssetRoot(scene), t, HeldModel));
            });
            return;
        }
        // Held a quarter of the way up, its top on the end, above a hat.
        Carry::Staff(top) => {
            let mut out = vec![(Cylinder::new(0.018, 1.35).into(), at(0.305), [0.45, 0.3, 0.18], false)];
            out.extend(staff_top(top).into_iter().map(|(m, c, glow)| (m, at(0.98), c, glow)));
            out
        }
        Carry::Potion => vec![
            (Sphere::new(0.045).into(), at(0.02), [0.3, 0.75, 0.35], false),
            (Cylinder::new(0.015, 0.05).into(), at(0.08), [0.85, 0.85, 0.9], false),
        ],
    };
    for (mesh, t, color, glow) in shapes {
        let shape = (Mesh3d(meshes.add(mesh)), MeshMaterial3d(plain(materials, color, glow)), t, HeldModel);
        commands.entity(hand).queue_silenced(move |mut e: EntityWorldMut| {
            e.with_child(shape);
        });
    }
}

use crate::back::{merged, rod};
use std::f32::consts::{FRAC_PI_2, TAU};

type Top = Vec<(Mesh, [f32; 3], bool)>;
const WOOD: [f32; 3] = [0.45, 0.3, 0.18];
const BRASS: [f32; 3] = [0.8, 0.6, 0.25];

/// A sphere of radius `r` centred `y` up the staff.
fn orb(r: f32, y: f32) -> Mesh {
    Sphere::new(r).mesh().uv(24, 16).translated_by(Vec3::Y * y)
}

/// Three of `make`, a third of a turn apart round the staff.
fn three(make: &dyn Fn(Vec3) -> Mesh) -> Mesh {
    merged((0..3).map(|i| make(Vec3::new((TAU * i as f32 / 3.0).cos(), 0.0, (TAU * i as f32 / 3.0).sin()))))
}

/// The top of a staff, from its end (at the origin) up +Y: (mesh, colour, glows?).
pub(super) fn staff_top(top: crossfir3d::held::StaffTop) -> Top {
    use crossfir3d::held::StaffTop;
    match top {
        StaffTop::Plain => Vec::new(),
        StaffTop::Cradle(c) => cradle_top(c),
        StaffTop::Flame(c) => flame_top(c),
        StaffTop::Crystal(c) => crystal_top(c),
        StaffTop::Hoop(c) => hoop_top(c),
        StaffTop::Horns(c) => horns_top(c),
        StaffTop::Sun(c) => sun_top(c),
    }
}

/// Three curled wooden prongs round a glowing orb.
fn cradle_top(c: [f32; 3]) -> Top {
    vec![
        (three(&|out| {
            let (a, b, tip) = (out * 0.012, out * 0.05 + Vec3::Y * 0.06, out * 0.022 + Vec3::Y * 0.13);
            merged([rod(a, b, 0.008), rod(b, tip, 0.006), Sphere::new(0.008).mesh().build().translated_by(b)])
        }), WOOD, false),
        (orb(0.042, 0.085), c, true),
    ]
}

/// A brass cup holding a glowing flame.
fn flame_top(c: [f32; 3]) -> Top {
    // A teardrop, round below and pointed above, twisting a little.
    let flame = crate::hat::rings(16, 0.0, |t| {
        let r = 0.045 * (std::f32::consts::PI * t.max(0.02)).sin().powf(0.5) * (1.0 - t).powf(0.6);
        (0.03 + 0.15 * t, r, r)
    });
    vec![
        (Cylinder::new(0.034, 0.035).mesh().build().translated_by(Vec3::Y * 0.0175), BRASS, false),
        (crate::hat::mesh(&flame, false, Vec2::ONE), c, true),
    ]
}

/// Three brass claws holding a glowing double-pointed crystal.
fn crystal_top(c: [f32; 3]) -> Top {
    vec![
        (three(&|out| rod(out * 0.012, out * 0.03 + Vec3::Y * 0.05, 0.006)), BRASS, false),
        (
            merged([
                Cone::new(0.032, 0.12).mesh().resolution(6).build().translated_by(Vec3::Y * 0.12),
                Cone::new(0.032, 0.05).mesh().resolution(6).build().rotated_by(Quat::from_rotation_x(std::f32::consts::PI)).translated_by(Vec3::Y * 0.035),
            ]),
            c,
            true,
        ),
    ]
}

/// A brass ring with a glowing orb in it.
fn hoop_top(c: [f32; 3]) -> Top {
    vec![
        (rod(Vec3::ZERO, Vec3::Y * 0.03, 0.012), BRASS, false),
        (Torus::new(0.055, 0.065).mesh().build().rotated_by(Quat::from_rotation_x(FRAC_PI_2)).translated_by(Vec3::Y * 0.09), BRASS, false),
        (orb(0.03, 0.09), c, true),
    ]
}

/// A pair of curved horns round a glowing orb.
fn horns_top(c: [f32; 3]) -> Top {
    let horn = |side: f32| {
        let path: Vec<(Vec3, f32)> = (0..=10)
            .map(|i| {
                let t = i as f32 / 10.0;
                (Vec3::new(side * (0.015 + 0.06 * t - 0.03 * t * t * t), 0.16 * t, 0.0), 0.014 * (1.0 - t) + 0.002)
            })
            .collect();
        crate::hat::mesh(&crate::hat::tube(&path), false, Vec2::ONE)
    };
    vec![(merged([horn(1.0), horn(-1.0)]), [0.2, 0.15, 0.12], false), (orb(0.03, 0.07), c, true)]
}

/// A glowing orb ringed with golden rays.
fn sun_top(c: [f32; 3]) -> Top {
    // Rays out from the orb every way, so it is a star from any side.
    let rays = merged((0..14).map(|i| {
        // Spread evenly over a ball (a golden-angle spiral).
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / 14.0;
        let a = i as f32 * 2.4;
        let out = Vec3::new((1.0 - y * y).sqrt() * a.cos(), y, (1.0 - y * y).sqrt() * a.sin());
        Cone::new(0.012, 0.05).mesh().resolution(8).build().rotated_by(Quat::from_rotation_arc(Vec3::Y, out)).translated_by(Vec3::Y * 0.09 + out * 0.055)
    }));
    vec![(merged([rays, rod(Vec3::ZERO, Vec3::Y * 0.06, 0.012)]), [0.95, 0.75, 0.3], false), (orb(0.035, 0.09), c, true)]
}

/// A bone's own scale inside its body part (the armature may be scaled).
pub(super) fn bone_scale(bone: Entity, parents: &Query<(&ChildOf, &Transform)>, parts: &Query<(), With<PartOf>>) -> f32 {
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
