//! The monk's robe: a long skirt from the waist to the ankles, a sash from the left shoulder
//! to the right hip, and prayer beads. They are skinned to the outfit's own bones, so they
//! bend with it, and shaped around the outfit's points in its rest pose (metres, +Y up,
//! +Z to the front, the character's left at +X).

use bevy::prelude::*;

use crate::hat::{Grid, AROUND};

/// Where the outfit's bones are in its rest pose.
pub struct Rest {
    pub pelvis: Vec3,
    pub chest: Vec3,
    pub neck: Vec3,
    pub left_shoulder: Vec3,
    pub ankle_y: f32,
}

/// Each point's weights for its mesh's four joints, adding up to 1.
type Weights = Vec<Vec<[f32; 4]>>;

/// How far out from `centre` the outfit reaches at `centre`'s height in direction `dir`
/// (its points within 3.5 cm of that height and about 12° of that direction), or `none`.
pub(crate) fn reach(points: &[Vec3], centre: Vec3, dir: Vec3, none: f32) -> f32 {
    points
        .iter()
        .filter(|p| (p.y - centre.y).abs() < 0.035)
        .map(|p| Vec3::new(p.x - centre.x, 0.0, p.z - centre.z))
        .filter(|d| d.normalize_or_zero().dot(dir) > 0.978)
        .map(|d| d.length())
        .fold(none, f32::max)
}

/// The highest of the outfit's points within 2.5 cm across of `at` and below `under`.
pub(crate) fn top(points: &[Vec3], at: Vec3, under: f32) -> f32 {
    points.iter().filter(|p| p.y < under && p.xz().distance(at.xz()) < 0.025).map(|p| p.y).fold(f32::MIN, f32::max)
}

pub(crate) fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The skirt, rows from the hem up to the waist (so its normals face out).
fn skirt(rest: &Rest, points: &[Vec3]) -> Grid {
    let rows = 14;
    let (top, hem) = (rest.pelvis.y + 0.07, rest.ankle_y + 0.07);
    (0..=rows)
        .rev()
        .map(|i| {
            // 0 at the waist, 1 at the hem.
            let t = i as f32 / rows as f32;
            let centre = Vec3::new(0.0, top + (hem - top) * t, rest.pelvis.z);
            let radii: Vec<f32> = (0..AROUND)
                .map(|j| {
                    let dir = crate::hat::ring(j);
                    let waist = reach(points, Vec3::new(0.0, top, rest.pelvis.z), dir, 0.16) + 0.025;
                    // A-line: wider toward the hem, never inside the legs or the tunic's skirt.
                    (waist + 0.06 * t).max(reach(points, centre, dir, 0.0) + 0.055)
                })
                .collect();
            // Smoothed round the ring: cloth hangs in folds, not lumps.
            let near = |j: usize, d: usize| radii[(j + AROUND - d) % AROUND].max(radii[(j + d) % AROUND]);
            (0..AROUND).map(|j| centre + radii[j].max((near(j, 1) + near(j, 2)) / 2.0 - 0.004) * crate::hat::ring(j)).collect()
        })
        .collect()
}

/// Weights over (pelvis, left thigh, right thigh): stiff at the waist, following each leg below.
fn skirt_weights(grid: &Grid, rest: &Rest) -> Weights {
    let rows = grid.len() - 1;
    grid.iter()
        .enumerate()
        .map(|(i, row)| {
            // The pelvis keeps a quarter even at the hem, so the cloth between the legs
            // stretches less.
            // From just below the belt, as the tunic under it does.
            let legs = 0.75 * smooth((1.0 - i as f32 / rows as f32) / 0.3);
            row.iter()
                .map(|p| {
                    let left = smooth((p.x - rest.pelvis.x) / 0.16 + 0.5);
                    [1.0 - legs, legs * left, legs * (1.0 - left), 0.0]
                })
                .collect()
        })
        .collect()
}

/// The sash: a band round the body, over the left shoulder and under the right arm to the hip.
fn sash(rest: &Rest, points: &[Vec3]) -> Grid {
    let high = rest.left_shoulder.y + 0.02;
    let low = rest.pelvis.y + 0.08;
    let width = 0.12;
    [-0.5, 0.0, 0.5]
        .iter()
        .map(|across| {
            (0..AROUND)
                .map(|j| {
                    let dir = crate::hat::ring(j);
                    // A tilted plane: high at the left (+X), low at the right.
                    let y = (high + low) / 2.0 + (high - low) / 2.0 * dir.x + across * width;
                    let centre = Vec3::new(0.0, y, rest.chest.z);
                    // Close round the chest, not out along the shoulder; there it lies on top.
                    let mut p = centre + (reach(points, centre, dir, 0.15) + 0.012).min(0.17) * dir;
                    if p.y > rest.chest.y {
                        p.y = p.y.max(top(points, p, rest.neck.y - 0.03) + 0.012);
                    }
                    p
                })
                .collect()
        })
        .collect()
}

/// Weights over (chest, waist, pelvis, left collarbone) by height.
fn sash_weights(grid: &Grid, rest: &Rest) -> Weights {
    grid.iter()
        .map(|row| {
            row.iter()
                .map(|p| {
                    let pelvis = 1.0 - smooth((p.y - rest.pelvis.y) / 0.15);
                    let chest = smooth((p.y - rest.pelvis.y - 0.15) / (rest.chest.y - rest.pelvis.y - 0.15));
                    let collar = 0.5 * smooth((p.y - rest.chest.y) / 0.12) * smooth(p.x / 0.1);
                    let rest_of = 1.0 - pelvis;
                    [rest_of * chest * (1.0 - collar), rest_of * (1.0 - chest), pelvis, rest_of * chest * collar]
                })
                .collect()
        })
        .collect()
}

/// Prayer beads: round the back of the neck, hanging down onto the chest in front.
fn beads(rest: &Rest, points: &[Vec3]) -> Mesh {
    let count = 36;
    let mut out: Option<Mesh> = None;
    for k in 0..count {
        let dir = crate::hat::ring(k * AROUND / count);
        let front = dir.z.max(0.0).powi(2);
        // A bigger bead at the bottom.
        let size = if k == count / 4 { 0.022 } else { 0.014 };
        let neck = Vec3::new(0.0, rest.neck.y, rest.neck.z + 0.02);
        let at = if front < 0.25 {
            // Lying on the collar and shoulders.
            let p = neck + 0.09 * dir;
            let surface = top(points, p, rest.neck.y);
            Vec3::new(p.x, if surface > 0.0 { surface + size } else { rest.neck.y - 0.035 }, p.z)
        } else {
            // Hanging in a U down the front of the chest, no wider than the neck.
            let centre = neck - Vec3::Y * (0.035 + 0.17 * front);
            let toward = Vec3::new(0.6 * dir.x, 0.0, 1.0).normalize();
            centre + (reach(points, centre, toward, 0.1) + size) * toward
        };
        let bead = Sphere::new(size).mesh().ico(1).expect("a small ico sphere").translated_by(at);
        match &mut out {
            Some(m) => m.merge(&bead).expect("the beads share attributes"),
            None => out = Some(bead),
        }
    }
    out.expect("the beads")
}

/// Adds skinning to a mesh made from `grid` by `hat::mesh` (same vertex order).
fn with_joints(mesh: Mesh, weights: &Weights, closed: bool, joints: [u16; 4]) -> Mesh {
    use bevy::mesh::VertexAttributeValues;
    let rows = weights.len() + closed as usize;
    let mut w = Vec::new();
    for i in 0..rows {
        for j in 0..=AROUND {
            w.push(weights[i % weights.len()][j % AROUND]);
        }
    }
    mesh.with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(vec![joints; w.len()]))
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, w)
}

/// What to put on a character once its outfit's bones exist: a robe, and what is on its back.
#[derive(Component)]
pub struct Dress {
    pub robe: bool,
    pub back: Option<crossfir3d::held::Back>,
    pub over: Option<crossfir3d::held::Over>,
    /// A creature's rags, of this colour.
    pub rags: Option<[f32; 3]>,
}

impl Dress {
    /// What a class puts on its outfit: over armour, only what is on its back. None: nothing.
    pub fn of(class: Option<crossfir3d::held::ClassLook>, armour: bool) -> Option<Dress> {
        let class = class?;
        let dress = Dress { robe: !armour && class.garb == crossfir3d::held::Garb::Robe, back: class.back, over: class.over.filter(|_| !armour), rags: None };
        (dress.robe || dress.back.is_some() || dress.over.is_some()).then_some(dress)
    }
}

mod dress;
pub use dress::dress_robes;

/// The male peasant outfit's bones.
#[cfg(test)]
pub(crate) fn peasant_rest() -> Rest {
    Rest {
        pelvis: Vec3::new(0.0, 0.949, -0.043),
        chest: Vec3::new(0.0, 1.311, 0.007),
        neck: Vec3::new(0.0, 1.52, -0.041),
        left_shoulder: Vec3::new(0.192, 1.456, -0.065),
        ankle_y: 0.086,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armour_keeps_the_back_but_covers_the_clothes() {
        use crossfir3d::held::{class_look, Back, Over};
        let monk = class_look("monk");
        assert!(Dress::of(monk, false).is_some_and(|d| d.robe));
        assert!(Dress::of(monk, true).is_none(), "no robe over armour");
        let warrior = Dress::of(class_look("warrior"), true).unwrap();
        assert!(matches!(warrior.back, Some(Back::Cape(_))), "a cape over armour");
        assert_eq!(Dress::of(class_look("alchemist"), false).unwrap().over, Some(Over::Apron));
        assert!(Dress::of(class_look("alchemist"), true).is_none(), "no apron over armour");
        assert!(Dress::of(class_look("wizard"), false).is_none());
        assert!(Dress::of(None, false).is_none());
    }
    use super::peasant_rest as rest;

    #[test]
    fn the_skirt_falls_from_waist_to_ankles_and_flares() {
        let s = skirt(&rest(), &[]);
        let (top, hem) = (s.last().unwrap(), &s[0]);
        assert!(top[0].y > 0.95 && hem[0].y < 0.2 && hem[0].y > 0.086);
        let width = |row: &[Vec3]| row.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        assert!(width(hem) > width(top) + 0.05);
    }

    #[test]
    fn the_skirt_follows_the_legs_below_the_waist() {
        let s = skirt(&rest(), &[]);
        let w = skirt_weights(&s, &rest());
        for row in &w {
            for p in row {
                assert!((p.iter().sum::<f32>() - 1.0).abs() < 1e-5);
            }
        }
        assert_eq!(w.last().unwrap()[0], [1.0, 0.0, 0.0, 0.0], "the waist moves with the hips");
        let hem = &w[0];
        assert!(hem[0][1] > 0.6, "the left side of the hem follows the left thigh");
        assert!(hem[AROUND / 2][2] > 0.6, "the right side the right thigh");
    }

    #[test]
    fn the_robe_faces_out() {
        let s = skirt(&rest(), &[]);
        let n = crate::hat::normals(&s, false);
        let mid = s.len() / 2;
        for (p, n) in s[mid].iter().zip(&n[mid]) {
            assert!(n.dot(Vec3::new(p.x, 0.0, p.z + 0.043)) > 0.0, "at {p}");
        }
    }

    #[test]
    fn the_sash_crosses_from_left_shoulder_to_right_hip() {
        let s = sash(&rest(), &[]);
        let middle = &s[s.len() / 2];
        let (left, right) = (middle[0], middle[AROUND / 2]);
        assert!(left.x > 0.1 && left.y > 1.4, "over the left shoulder: {left}");
        assert!(right.x < -0.1 && right.y < 1.1, "at the right hip: {right}");
    }
}
