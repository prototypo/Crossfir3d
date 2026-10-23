//! Dressing a character: finds its outfit's bones and points, and puts the robe, the back
//! pieces and the rags on as skinned parts of the outfit.

use bevy::ecs::system::SystemParam;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

use super::{beads, sash, sash_weights, skirt, skirt_weights, with_joints, Dress, Rest};
use crate::back;
use crate::hat::{Grid, Leather, AROUND};
use crossfir3d::held::{Back, Over};

const SAFFRON: [f32; 3] = [0.98, 0.6, 0.12];
const SASH: [f32; 3] = [0.7, 0.2, 0.06];
const WOOD: [f32; 3] = [0.3, 0.17, 0.08];

/// The meshes to add to the outfit, each with its material.
type Wear = Vec<(Mesh, StandardMaterial)>;

/// Everything `dress_robes` reads or changes in a frame.
#[derive(SystemParam)]
pub struct DressParams<'w, 's> {
    commands: Commands<'w, 's>,
    robed: Query<'w, 's, (Entity, &'static Dress)>,
    children: Query<'w, 's, &'static Children>,
    names: Query<'w, 's, &'static Name>,
    outfits: Query<'w, 's, (), With<crate::characters3d::OutfitPart>>,
    skinned: Query<'w, 's, (&'static Mesh3d, &'static SkinnedMesh, &'static ChildOf)>,
    poses: Res<'w, Assets<SkinnedMeshInverseBindposes>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    leather: Res<'w, Leather>,
}

/// The indices, in the outfit's skin, of the bones the dressing skins to.
struct Bones {
    pelvis: usize,
    thigh_l: usize,
    thigh_r: usize,
    chest: usize,
    waist: usize,
    collar: usize,
}

/// What the dressing of one outfit works from: its rest pose, points, bones and materials.
struct Fit<'a> {
    rest: Rest,
    points: Vec<Vec3>,
    arm_points: Vec<Vec3>,
    bones: Bones,
    /// Each arm's (upper arm, lower arm, hand) bones.
    sleeves: Vec<[usize; 3]>,
    poses: &'a [Mat4],
    leather: &'a Leather,
}

/// A bone index as a skin joint index.
fn joint(i: usize) -> u16 {
    i as u16
}

/// A double-sided cloth material of colour `[r, g, b]` and the given roughness.
fn cloth([r, g, b]: [f32; 3], rough: f32) -> StandardMaterial {
    StandardMaterial { base_color: Color::srgb(r, g, b), perceptual_roughness: rough, double_sided: true, cull_mode: None, ..default() }
}

/// Skins all of a mesh to one bone.
fn on(bone: usize, m: Mesh) -> Mesh {
    let n = m.count_vertices();
    m.with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(vec![[joint(bone), 0, 0, 0]; n]))
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[1.0f32, 0.0, 0.0, 0.0]; n])
}

/// The position of the skin's joint `name`, if it has one.
fn slot(skin: &SkinnedMesh, names: &Query<&Name>, name: &str) -> Option<usize> {
    skin.joints.iter().position(|j| names.get(*j).is_ok_and(|n| n.as_str() == name))
}

/// The outfit's points, and (apart) its arms' points (they stand out sideways in the rest pose).
fn outfit_points(parts: &[Entity], skinned: &Query<(&Mesh3d, &SkinnedMesh, &ChildOf)>, names: &Query<&Name>, meshes: &Assets<Mesh>) -> (Vec<Vec3>, Vec<Vec3>) {
    let (mut points, mut arm_points) = (Vec::new(), Vec::new());
    for &e in parts {
        let Ok((mesh, ..)) = skinned.get(e) else { continue };
        let arms = names.get(e).is_ok_and(|n| n.as_str().contains("Arms"));
        if let Some(VertexAttributeValues::Float32x3(p)) = meshes.get(&mesh.0).and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION)) {
            (if arms { &mut arm_points } else { &mut points }).extend(p.iter().map(|v| Vec3::from_array(*v)));
        }
    }
    (points, arm_points)
}

impl<'a> Fit<'a> {
    /// The fit of a skin, or none while it lacks bones the dressing needs.
    fn new(skin: &SkinnedMesh, names: &Query<&Name>, poses: &'a [Mat4], leather: &'a Leather, points: (Vec<Vec3>, Vec<Vec3>)) -> Option<Self> {
        let bones = ["pelvis", "thigh_l", "thigh_r", "spine_03", "spine_01", "neck_01", "upperarm_l", "clavicle_l", "foot_l"].map(|n| slot(skin, names, n));
        let [Some(pelvis), Some(thigh_l), Some(thigh_r), Some(chest), Some(waist), Some(neck), Some(shoulder), Some(collar), Some(foot)] = bones else { return None };
        let at = |i: usize| poses[i].inverse().w_axis.truncate();
        let rest = Rest { pelvis: at(pelvis), chest: at(chest), neck: at(neck), left_shoulder: at(shoulder), ankle_y: at(foot).y };
        let sleeves = ["l", "r"]
            .iter()
            .filter_map(|side| {
                let bone = |n: &str| slot(skin, names, &format!("{n}_{side}"));
                Some([bone("upperarm")?, bone("lowerarm")?, bone("hand")?])
            })
            .collect();
        let (points, arm_points) = points;
        Some(Fit { rest, points, arm_points, bones: Bones { pelvis, thigh_l, thigh_r, chest, waist, collar }, sleeves, poses, leather })
    }

    /// The position of a bone in the rest pose.
    fn at(&self, i: usize) -> Vec3 {
        self.poses[i].inverse().w_axis.truncate()
    }

    /// A cloth material of colour `c` with the leather texture and bumps.
    fn leathery(&self, c: [f32; 3], rough: f32) -> StandardMaterial {
        StandardMaterial {
            base_color_texture: Some(self.leather.colour.clone()),
            normal_map_texture: Some(self.leather.bumps.clone()),
            ..cloth(c, rough)
        }
    }

    /// Skins all of a mesh to the chest bone.
    fn on_chest(&self, m: Mesh) -> Mesh {
        on(self.bones.chest, m)
    }

    /// A `grid` of cloth hanging from the chest, over the pelvis and thighs.
    fn hanging(&self, grid: &Grid) -> Mesh {
        let b = &self.bones;
        with_joints(crate::hat::mesh(grid, false, Vec2::new(6.0, 6.0)), &back::cape_weights(grid, &self.rest), false, [joint(b.chest), joint(b.pelvis), joint(b.thigh_l), joint(b.thigh_r)])
    }

    /// A cape hanging to height `hem`, with hem teeth of size `teeth`.
    fn cape(&self, hem: f32, teeth: f32) -> Mesh {
        self.hanging(&back::cape(&self.rest, &self.points, hem, teeth))
    }

    /// Back pieces that move with the chest only.
    fn rigid(&self, parts: back::Parts) -> Wear {
        parts.into_iter().map(|(m, c, leathered)| (self.on_chest(m), if leathered { self.leathery(c, 0.75) } else { cloth(c, 0.6) })).collect()
    }

    /// What is on the back: a cape, fur, a quiver, a sack or a scabbard.
    fn back_piece(&self, back: Option<Back>) -> Wear {
        let rest = &self.rest;
        match back {
            Some(Back::Cape(c)) => vec![(self.cape(rest.pelvis.y - 0.5, 0.0), cloth(c, 0.9))],
            Some(Back::Fur) => vec![(self.cape(rest.chest.y - 0.12, 0.05), self.leathery([0.42, 0.3, 0.2], 1.0))],
            Some(Back::Quiver) => self.rigid(back::quiver(rest, &self.points)),
            Some(Back::Sack) => self.rigid(back::sack(rest, &self.points)),
            Some(Back::Scabbard) => self.rigid(back::scabbard(rest, &self.points)),
            None => Vec::new(),
        }
    }

    /// What is worn over the clothes: an apron, a tabard or a sash.
    fn over_piece(&self, over: Option<Over>) -> Wear {
        let (rest, points) = (&self.rest, &self.points);
        match over {
            Some(Over::Apron) => {
                let grid = back::apron(rest, points);
                let brown = [0.4, 0.26, 0.15];
                vec![(self.hanging(&grid), self.leathery(brown, 0.7)), (self.on_chest(back::apron_straps(rest, &grid)), self.leathery(brown, 0.7))]
            }
            Some(Over::Tabard(c)) => {
                let panels = back::tabard(rest, points);
                let mut wear = vec![(self.on_chest(back::tabard_straps(rest, points, &panels)), cloth(c, 0.9))];
                wear.extend(panels.iter().map(|g| (self.hanging(g), cloth(c, 0.9))));
                wear
            }
            Some(Over::Sash(c)) => {
                let grid = back::sash(rest, points);
                let pelvis = self.bones.pelvis;
                vec![
                    (on(pelvis, crate::hat::mesh(&grid, false, Vec2::new(4.0, 0.5))), cloth(c, 0.85)),
                    (on(pelvis, back::sash_ends(rest, points)), cloth(c, 0.85)),
                ]
            }
            None => Vec::new(),
        }
    }

    /// A creature's rags of colour `c`, with sleeves over the shirt's own (its shoulder tabs and cuffs).
    fn rags(&mut self, c: [f32; 3]) -> Wear {
        let mut wear: Wear = back::rags(&self.rest, &self.points).iter().map(|g| (self.hanging(g), self.leathery(c, 1.0))).collect();
        self.arm_points.extend(&self.points);
        for [upper, lower, hand] in &self.sleeves {
            // The upper arm to the elbow, and on over the shirt sleeve's end below it.
            for (from, to, long) in [(*upper, *lower, 0.97), (*lower, *hand, 0.45)] {
                let grid = back::sleeve(&self.arm_points, self.at(from), self.at(to), long);
                let weights = vec![vec![[1.0, 0.0, 0.0, 0.0]; AROUND]; grid.len()];
                wear.push((with_joints(crate::hat::mesh(&grid, false, Vec2::new(3.0, 1.0)), &weights, false, [joint(from), 0, 0, 0]), self.leathery(c, 1.0)));
            }
        }
        wear
    }

    /// The robe: skirt, sash and prayer beads.
    fn robe(&self) -> Wear {
        let (rest, points, b) = (&self.rest, &self.points, &self.bones);
        let skirt_grid = skirt(rest, points);
        let sash_grid = sash(rest, points);
        let skirt_mesh = with_joints(crate::hat::mesh(&skirt_grid, false, Vec2::new(4.0, 2.0)), &skirt_weights(&skirt_grid, rest), false, [joint(b.pelvis), joint(b.thigh_l), joint(b.thigh_r), 0]);
        let sash_mesh = with_joints(crate::hat::mesh(&sash_grid, false, Vec2::new(6.0, 0.3)), &sash_weights(&sash_grid, rest), false, [joint(b.chest), joint(b.waist), joint(b.pelvis), joint(b.collar)]);
        vec![(skirt_mesh, cloth(SAFFRON, 0.95)), (sash_mesh, cloth(SASH, 0.9)), (self.on_chest(beads(rest, points)), cloth(WOOD, 0.5))]
    }

    /// Everything `dress` asks for, back first.
    fn wear(&mut self, dress: &Dress) -> Wear {
        let mut wear = self.back_piece(dress.back);
        wear.extend(self.over_piece(dress.over));
        if let Some(c) = dress.rags {
            wear.extend(self.rags(c));
        }
        if dress.robe {
            wear.extend(self.robe());
        }
        wear
    }
}

/// Puts the robe, sash and beads, and the back pieces, on each `Dress`ed character, as
/// skinned parts of its outfit.
pub fn dress_robes(p: DressParams) {
    let DressParams { mut commands, robed, children, names, outfits, skinned, poses, mut meshes, mut materials, leather } = p;
    for (root, dress) in &robed {
        // The outfit's scene spawns a little after the character: try again next frame.
        let Some(outfit) = children.get(root).into_iter().flatten().copied().find(|c| outfits.contains(*c)) else { continue };
        let parts: Vec<_> = children.iter_descendants(outfit).filter(|e| skinned.contains(*e)).collect();
        let Some(&first) = parts.first() else { continue };
        let Ok((_, skin, parent)) = skinned.get(first) else { continue };
        let Some(poses) = poses.get(&skin.inverse_bindposes) else { continue };
        let points = outfit_points(&parts, &skinned, &names, &meshes);
        let Some(mut fit) = Fit::new(skin, &names, poses, &leather, points) else {
            commands.entity(root).remove::<Dress>();
            continue;
        };
        for (mesh, material) in fit.wear(dress) {
            let part = (Mesh3d(meshes.add(mesh)), MeshMaterial3d(materials.add(material)), skin.clone(), ChildOf(parent.parent()));
            commands.queue_silenced(move |world: &mut World| {
                world.spawn(part);
            });
        }
        commands.entity(root).remove::<Dress>();
    }
}
