//! Walking skeletons: a person's body, its skin hidden, with bones built in code on its own
//! bones, so they move with its animations.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::back::{merged, rod};

/// Bone colour, a little yellowed; and the dark of the eye and nose holes.
const BONE: [f32; 3] = [0.82, 0.76, 0.6];
const HOLE: [f32; 3] = [0.06, 0.05, 0.04];

/// A ball of radius `r`, stretched by `stretch` along (side, up, front).
fn ball(at: Vec3, r: f32, stretch: Vec3, frame: Quat) -> Mesh {
    Sphere::new(r).mesh().uv(16, 10).scaled_by(stretch).rotated_by(frame).translated_by(at)
}

/// A long bone from `a` to `b`, knobbly at both ends.
fn long(a: Vec3, b: Vec3, r: f32) -> Mesh {
    merged([rod(a, b, r), ball(a, r * 1.6, Vec3::ONE, Quat::IDENTITY), ball(b, r * 1.6, Vec3::ONE, Quat::IDENTITY)])
}

/// What every part of the body is built from: the joints, the body's own axes and the size.
struct Body<'a> {
    j: &'a HashMap<&'a str, Vec3>,
    up: Vec3,
    front: Vec3,
    side: Vec3,
    frame: Quat,
    s: f32,
}

type Parts = Vec<(&'static str, Mesh, bool)>;

/// The bones, in the same space as the joints `j` (by bone name), `up` and `front` the
/// body's own: (the bone each part moves with, its mesh, dark?).
pub fn bones(j: &HashMap<&str, Vec3>, up: Vec3, front: Vec3) -> Parts {
    let side = up.cross(front).normalize();
    let frame = Quat::from_mat3(&Mat3::from_cols(side, up, front));
    // Sizes are for a person; bigger or smaller people scale them.
    let s = j["pelvis"].distance(j["neck_01"]) / 0.55;
    let b = Body { j, up, front, side, frame, s };
    let mut out: Parts = Vec::new();
    for side_name in ["l", "r"] {
        limbs(&b, side_name, &mut out);
    }
    spine(&b, &mut out);
    ribs(&b, &mut out);
    pelvis(&b, &mut out);
    skull(&b, &mut out);
    out
}

/// Limbs, hands and feet of one side, each piece on the bone it starts from.
fn limbs(b: &Body, side_name: &str, out: &mut Parts) {
    let (j, s) = (b.j, b.s);
    let at = |n: &str| j[format!("{n}_{side_name}").as_str()];
    let name = |n: &str| -> &'static str { Box::leak(format!("{n}_{side_name}").into_boxed_str()) };
    for (a, b, r) in [
        ("clavicle", "upperarm", 0.012),
        ("upperarm", "lowerarm", 0.02),
        ("lowerarm", "hand", 0.016),
        ("thigh", "calf", 0.026),
        ("calf", "foot", 0.02),
        ("foot", "ball", 0.016),
        ("ball", "ball_leaf", 0.012),
    ] {
        out.push((name(a), long(at(a), at(b), r * s), false));
    }
    for f in ["thumb", "index", "middle", "ring", "pinky"] {
        let joint = |n: &str| at(&format!("{f}_{n}"));
        out.push((name("hand"), long(at("hand"), joint("01"), 0.006 * s), false));
        for (a, b) in [("01", "02"), ("02", "03"), ("03", "04_leaf")] {
            out.push((name(&format!("{f}_{a}")), long(joint(a), joint(b), 0.0055 * s), false));
        }
    }
}

/// The spine: a rod with a disc for each bone in it.
fn spine(b: &Body, out: &mut Parts) {
    let (j, s) = (b.j, b.s);
    for (a, b) in [("pelvis", "spine_01"), ("spine_01", "spine_02"), ("spine_02", "spine_03"), ("spine_03", "neck_01"), ("neck_01", "Head")] {
        let (pa, pb) = (j[a], j[b]);
        let along = (pb - pa).normalize();
        let discs = (0..3).map(|k| {
            let at = pa.lerp(pb, (k as f32 + 0.5) / 3.0);
            Cylinder::new(0.02 * s, 0.014 * s).mesh().build().rotated_by(Quat::from_rotation_arc(Vec3::Y, along)).translated_by(at)
        });
        let key: &'static str = match a { "pelvis" => "pelvis", "spine_01" => "spine_01", "spine_02" => "spine_02", "spine_03" => "spine_03", _ => "neck_01" };
        out.push((key, merged(std::iter::once(rod(pa, pb, 0.011 * s)).chain(discs)), false));
    }
}

/// The ribs: curved from the spine round each side to the breastbone, sloping down.
fn ribs(b: &Body, out: &mut Parts) {
    let Body { j, up, front, side, s, .. } = *b;
    let (low, high) = (j["spine_02"], j["spine_03"] + up * 0.08 * s);
    let mut ribs = Vec::new();
    let count = 6;
    for i in 0..count {
        let t = i as f32 / (count - 1) as f32;
        let centre = low.lerp(high, t) + front * 0.07 * s;
        let wide = s * (0.105 + 0.02 * (std::f32::consts::PI * t).sin()) * (1.0 - 0.3 * t * t);
        let deep = 0.08 * s;
        for sign in [1.0f32, -1.0] {
            let path: Vec<(Vec3, f32)> = (0..=12)
                .map(|k| {
                    let a = 0.3 + 2.4 * k as f32 / 12.0;
                    let at = centre + side * sign * wide * a.sin() - front * deep * a.cos() - up * 0.04 * s * (a / std::f32::consts::PI);
                    (at, 0.0075 * s)
                })
                .collect();
            ribs.push(crate::hat::mesh(&crate::hat::tube(&path), false, Vec2::ONE));
        }
    }
    // The breastbone, down the front where the ribs meet.
    let front_of = |p: Vec3| p + front * 0.07 * s + front * 0.08 * s * 0.74 - up * 0.03 * s;
    ribs.push(long(front_of(high), front_of(low.lerp(high, 0.3)), 0.012 * s));
    out.push(("spine_03", merged(ribs), false));
}

/// The pelvis: a ring round the hips, wide flat wings each side, the sacrum behind.
fn pelvis(b: &Body, out: &mut Parts) {
    let Body { j, up, front, side, frame, s } = *b;
    let hips = j["pelvis"];
    let mut pelvis = vec![
        Torus::new(0.055 * s, 0.075 * s).mesh().build().scaled_by(Vec3::new(1.2, 1.0, 0.9)).rotated_by(frame).translated_by(hips - up * 0.02 * s + front * 0.01 * s),
        ball(hips - front * 0.045 * s + up * 0.02 * s, 0.035 * s, Vec3::new(1.0, 1.4, 0.7), frame),
    ];
    for sign in [1.0f32, -1.0] {
        let wing = frame * Quat::from_rotation_y(sign * 0.7) * Quat::from_rotation_z(-sign * 0.3);
        pelvis.push(ball(hips + side * sign * 0.075 * s + up * 0.045 * s + front * 0.01 * s, 0.05 * s, Vec3::new(0.9, 1.0, 0.25), wing));
    }
    out.push(("pelvis", merged(pelvis), false));
}

/// The skull: a round crown, a narrower face, the jaw and teeth; dark eye and nose holes.
fn skull(b: &Body, out: &mut Parts) {
    let Body { j, up, front, side, frame, s } = *b;
    let head = j["Head"] + up * 0.12 * s;
    out.push((
        "Head",
        merged([
            ball(head, 0.1 * s, Vec3::new(0.85, 1.0, 1.05), frame),
            ball(head - up * 0.05 * s + front * 0.045 * s, 0.07 * s, Vec3::new(0.95, 0.9, 0.85), frame),
            Cuboid::new(0.085 * s, 0.03 * s, 0.07 * s).mesh().build().rotated_by(frame).translated_by(head - up * 0.115 * s + front * 0.045 * s),
            Cuboid::new(0.065 * s, 0.014 * s, 0.012 * s).mesh().build().rotated_by(frame).translated_by(head - up * 0.092 * s + front * 0.1 * s),
        ]),
        false,
    ));
    // Sunk into the skull, so only their flat fronts show.
    let hole = |at: Vec3, r: f32| ball(at, r * s, Vec3::new(1.0, 1.15, 0.3), frame);
    out.push((
        "Head",
        merged([
            hole(head - up * 0.02 * s + front * 0.097 * s + side * 0.034 * s, 0.024),
            hole(head - up * 0.02 * s + front * 0.097 * s - side * 0.034 * s, 0.024),
            hole(head - up * 0.058 * s + front * 0.1 * s, 0.011),
        ]),
        true,
    ));
}

/// A body to build bones on (in this tint) once its own bones exist.
#[derive(Component)]
pub struct Bony(pub [f32; 3]);

/// The queries and assets `build_skeletons` works with.
#[derive(SystemParam)]
pub struct SkeletonParams<'w, 's> {
    commands: Commands<'w, 's>,
    bony: Query<'w, 's, (Entity, &'static Bony, &'static GlobalTransform)>,
    children: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    globals: Query<'w, 's, &'static GlobalTransform>,
    skin: Query<'w, 's, (), With<Mesh3d>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

/// The named bones under `root`, and where each one is.
fn bone_ids_and_joints<'a>(
    root: Entity,
    children: &Query<&Children>,
    names: &'a Query<&Name>,
    globals: &Query<&GlobalTransform>,
) -> (HashMap<&'a str, Entity>, HashMap<&'a str, Vec3>) {
    let mut ids: HashMap<&str, Entity> = HashMap::new();
    for e in children.iter_descendants(root) {
        if let Ok(n) = names.get(e) {
            ids.insert(n.as_str(), e);
        }
    }
    let joints = ids.iter().filter_map(|(n, e)| Some((*n, globals.get(*e).ok()?.translation()))).collect();
    (ids, joints)
}

/// Whether the body's skin and bones are all in place at `at_y`.
fn placed(body: &[Entity], joints: &HashMap<&str, Vec3>, at_y: f32) -> bool {
    // The body scene spawns a little after the character, and its bones are placed a
    // frame later still: try again next frame.
    let placed = |n: &str| joints.get(n).is_some_and(|p| p.y > at_y + 0.3);
    !(body.is_empty() || !placed("Head") || !joints.contains_key("pinky_04_leaf_r"))
}

/// The bone and hole materials for a body tinted `[r, g, b]`.
fn bone_materials(materials: &mut Assets<StandardMaterial>, [r, g, b]: [f32; 3]) -> (Handle<StandardMaterial>, Handle<StandardMaterial>) {
    let bone = materials.add(StandardMaterial { base_color: Color::srgb(BONE[0] * r, BONE[1] * g, BONE[2] * b), perceptual_roughness: 0.55, ..default() });
    let hole = materials.add(StandardMaterial { base_color: Color::srgb(HOLE[0], HOLE[1], HOLE[2]), perceptual_roughness: 1.0, ..default() });
    (bone, hole)
}

/// Queues one bone piece as a child of the bone `e`, in the bone's own space.
fn spawn_piece(commands: &mut Commands, meshes: &mut Assets<Mesh>, (e, g): (Entity, &GlobalTransform), mesh: Mesh, material: Handle<StandardMaterial>) {
    // Into the bone's own space, so the piece moves with it.
    let local = Transform::from_matrix(Mat4::from(g.affine().inverse()));
    let part = (Mesh3d(meshes.add(mesh.transformed_by(local))), MeshMaterial3d(material), ChildOf(e));
    commands.queue_silenced(move |world: &mut World| {
        world.spawn(part);
    });
}

/// Hides each `Bony` body's skin and puts the bones on its bones.
pub fn build_skeletons(mut p: SkeletonParams) {
    for (root, Bony(tint), at) in &p.bony {
        let (ids, joints) = bone_ids_and_joints(root, &p.children, &p.names, &p.globals);
        let body: Vec<Entity> = p.children.iter_descendants(root).filter(|e| p.skin.contains(*e)).collect();
        if !placed(&body, &joints, at.translation().y) {
            continue;
        }
        for e in body {
            p.commands.entity(e).insert(Visibility::Hidden);
        }
        let turn = at.compute_transform().rotation;
        let (bone, hole) = bone_materials(&mut p.materials, *tint);
        for (name, mesh, dark) in bones(&joints, turn * Vec3::Y, turn * Vec3::Z) {
            let Some(&e) = ids.get(name) else { continue };
            let Ok(g) = p.globals.get(e) else { continue };
            spawn_piece(&mut p.commands, &mut p.meshes, (e, g), mesh, if dark { hole.clone() } else { bone.clone() });
        }
        p.commands.entity(root).remove::<Bony>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A person standing, facing +Z, its left at +X (metres).
    fn joints() -> HashMap<&'static str, Vec3> {
        let mut j: HashMap<&'static str, Vec3> = HashMap::new();
        for (name, at) in [
            ("pelvis", Vec3::new(0.0, 0.95, 0.0)),
            ("spine_01", Vec3::new(0.0, 1.05, 0.0)),
            ("spine_02", Vec3::new(0.0, 1.17, 0.0)),
            ("spine_03", Vec3::new(0.0, 1.3, 0.0)),
            ("neck_01", Vec3::new(0.0, 1.5, 0.0)),
            ("Head", Vec3::new(0.0, 1.58, 0.0)),
        ] {
            j.insert(name, at);
        }
        for (side, x) in [("l", 1.0f32), ("r", -1.0)] {
            let s = |n: &str| -> &'static str { Box::leak(format!("{n}_{side}").into_boxed_str()) };
            for (name, at) in [
                ("clavicle", Vec3::new(0.03, 1.45, 0.0)),
                ("upperarm", Vec3::new(0.18, 1.43, 0.0)),
                ("lowerarm", Vec3::new(0.2, 1.15, 0.0)),
                ("hand", Vec3::new(0.21, 0.9, 0.0)),
                ("thigh", Vec3::new(0.1, 0.92, 0.0)),
                ("calf", Vec3::new(0.11, 0.5, 0.0)),
                ("foot", Vec3::new(0.11, 0.08, 0.0)),
                ("ball", Vec3::new(0.11, 0.02, 0.12)),
                ("ball_leaf", Vec3::new(0.11, 0.02, 0.18)),
            ] {
                j.insert(s(name), at * Vec3::new(x, 1.0, 1.0));
            }
            for (k, f) in ["thumb", "index", "middle", "ring", "pinky"].iter().enumerate() {
                let x0 = 0.2 + 0.006 * k as f32;
                for (i, n) in ["01", "02", "03", "04_leaf"].iter().enumerate() {
                    j.insert(s(&format!("{f}_{n}")), Vec3::new(x * x0, 0.86 - 0.025 * i as f32, 0.03 - 0.012 * k as f32));
                }
            }
        }
        j
    }

    fn points(m: &Mesh) -> Vec<Vec3> {
        match m.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) => p.iter().map(|v| Vec3::from_array(*v)).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn every_piece_is_a_whole_mesh() {
        for (name, m, _) in bones(&joints(), Vec3::Y, Vec3::Z) {
            let n = m.count_vertices();
            for (id, values) in m.attributes() {
                assert_eq!(values.len(), n, "{name}: {id:?}");
            }
            let indices: Vec<usize> = m.indices().unwrap().iter().collect();
            assert!(indices.iter().all(|&i| i < n), "{name}: an index past the end");
        }
    }

    #[test]
    fn every_limb_and_finger_has_a_bone() {
        let parts = bones(&joints(), Vec3::Y, Vec3::Z);
        for name in ["upperarm_l", "lowerarm_r", "thigh_l", "calf_r", "foot_l", "index_02_r", "thumb_01_l", "hand_l"] {
            assert!(parts.iter().any(|p| p.0 == name), "{name}");
        }
    }

    #[test]
    fn the_ribs_curve_round_the_front_of_the_spine() {
        let parts = bones(&joints(), Vec3::Y, Vec3::Z);
        let ribs: Vec<Vec3> = parts.iter().filter(|p| p.0 == "spine_03").flat_map(|p| points(&p.1)).collect();
        assert!(ribs.iter().any(|p| p.z > 0.08) && ribs.iter().any(|p| p.x > 0.1) && ribs.iter().any(|p| p.x < -0.1));
        assert!(ribs.iter().all(|p| p.z > -0.06), "none far behind the spine");
    }

    #[test]
    fn the_skull_sits_on_the_neck_with_dark_eyes_in_front() {
        let parts = bones(&joints(), Vec3::Y, Vec3::Z);
        let skull: Vec<&(&str, Mesh, bool)> = parts.iter().filter(|p| p.0 == "Head").collect();
        let top = skull.iter().flat_map(|p| points(&p.1)).map(|p| p.y).fold(f32::MIN, f32::max);
        assert!(top > 1.75 && top < 1.85, "{top}");
        let eyes: Vec<Vec3> = skull.iter().filter(|p| p.2).flat_map(|p| points(&p.1)).collect();
        assert!(!eyes.is_empty() && eyes.iter().all(|p| p.z > 0.04), "dark eye holes in front");
    }
}
