//! What classes wear on their backs: capes and a fur cloak (skinned to the outfit's
//! bones, like the robe), and a quiver, a sack and a sheathed sword (fixed to the chest).
//! All in the outfit's rest pose: metres, +Y up, +Z to the front, the character's left at +X.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

use crate::hat::{Grid, AROUND};
use crate::robe::{reach, smooth, top, Rest};

/// Each point's weights for (chest, pelvis, left thigh, right thigh).
pub type Weights = Vec<Vec<[f32; 4]>>;

/// The middle of the body, front to back.
fn middle(rest: &Rest) -> f32 {
    (rest.chest.z + rest.pelvis.z) / 2.0
}

/// How far behind the middle the body's back is at `x`, `y`.
fn back_at(rest: &Rest, points: &[Vec3], x: f32, y: f32) -> f32 {
    middle(rest) - reach(points, Vec3::new(x, y, middle(rest)), Vec3::NEG_Z, 0.11)
}

/// A cape from over the shoulders down to `hem` (a height), hanging from the widest part of
/// the back and flaring a little. A thin double sheet, so a ring: across the outside from
/// the right to the left, back across the inside. `teeth`: a ragged hem this deep.
/// Rows go from the hem up, so its normals face out.
pub fn cape(rest: &Rest, points: &[Vec3], hem: f32, teeth: f32) -> Grid {
    let rows = 16;
    let half = AROUND / 2;
    let high = rest.neck.y - 0.04;
    let mut hang = vec![0.0f32; half];
    let mut grid: Grid = (0..=rows)
        .map(|i| {
            // 0 at the shoulders, 1 at the hem.
            let t = i as f32 / rows as f32;
            let y = high + (hem - high) * t;
            let centre = Vec3::new(0.0, y, middle(rest));
            // From the shoulder blades round the back; wider toward the hem.
            let spread = 0.38 + 0.08 * t;
            let outside: Vec<Vec3> = (0..half)
                .map(|s| {
                    let u = s as f32 / (half - 1) as f32;
                    // From the right (-X) round the back (-Z) to the left (+X).
                    let a = PI * (0.5 + spread * (1.0 - 2.0 * u));
                    let dir = Vec3::new(a.cos(), 0.0, -a.sin());
                    // Cloth hangs from the widest part above.
                    // Not out along the rest pose's sleeves.
                    hang[s] = hang[s].max(reach(points, centre, dir, 0.1).min(if dir.x.abs() > 0.6 { 0.21 } else { 1.0 }) + 0.04);
                    // Folds, deeper toward the hem.
                    let folds = 0.012 * t * (1.0 - (u * 7.0 * TAU).cos());
                    let mut p = centre + (hang[s] + 0.05 * t * t + folds) * dir;
                    if i == 0 {
                        p.y = p.y.max(top(points, p, rest.neck.y - 0.03) + 0.012);
                    }
                    if i == rows && s % 2 == 1 {
                        p.y += teeth;
                    }
                    p
                })
                .collect();
            let inside = outside.iter().rev().map(|p| *p - (*p - centre).with_y(0.0).normalize() * 0.006);
            outside.iter().copied().chain(inside).collect()
        })
        .collect();
    grid.reverse();
    grid
}

/// Weights: the chest down to the chest bone, then the hips, and the legs a little at the hem.
pub fn cape_weights(grid: &Grid, rest: &Rest) -> Weights {
    grid.iter()
        .map(|row| {
            row.iter()
                .map(|p| {
                    let chest = smooth((p.y - rest.pelvis.y) / (rest.chest.y - rest.pelvis.y));
                    let legs = 0.5 * smooth((rest.pelvis.y - 0.1 - p.y) / 0.3);
                    let left = smooth((p.x - rest.pelvis.x) / 0.16 + 0.5);
                    let hips = 1.0 - chest - legs;
                    [chest, hips, legs * left, legs * (1.0 - left)]
                })
                .collect()
        })
        .collect()
}

/// Several meshes as one.
fn merged(parts: impl IntoIterator<Item = Mesh>) -> Mesh {
    let mut parts = parts.into_iter();
    let mut out = parts.next().expect("at least one part");
    for m in parts {
        out.merge(&m).expect("the parts share attributes");
    }
    out
}

/// A rod of `radius` from `a` to `b`.
fn rod(a: Vec3, b: Vec3, radius: f32) -> Mesh {
    Cylinder::new(radius, a.distance(b)).mesh().resolution(12).build().rotated_by(Quat::from_rotation_arc(Vec3::Y, (b - a).normalize())).translated_by((a + b) / 2.0)
}

/// From low on the left of the back to over the right shoulder, `out` from the back.
fn slant(rest: &Rest, points: &[Vec3], out: f32) -> (Vec3, Vec3) {
    let low = Vec3::new(0.09, rest.pelvis.y + 0.14, 0.0);
    let high = Vec3::new(-0.1, rest.neck.y - 0.02, 0.0);
    let on = |p: Vec3| p.with_z(back_at(rest, points, p.x, p.y) - out);
    (on(low), on(high))
}

/// (mesh, colour, leather?) for each rigid piece.
pub type Parts = Vec<(Mesh, [f32; 3], bool)>;

/// A leather quiver, arrows standing out of it over the right shoulder.
pub fn quiver(rest: &Rest, points: &[Vec3]) -> Parts {
    let (low, high) = slant(rest, points, 0.05);
    let along = (high - low).normalize();
    let side = along.cross(Vec3::Z).normalize();
    let mut arrows = Vec::new();
    let mut feathers = Vec::new();
    for k in 0..5 {
        let off = side * 0.022 * (k as f32 - 2.0) + Vec3::Z * 0.012 * ((k % 2) as f32 - 0.5);
        let end = high + along * (0.1 + 0.015 * (k % 3) as f32) + off;
        arrows.push(rod(high - along * 0.1 + off, end, 0.004));
        feathers.push(Cuboid::new(0.002, 0.07, 0.02).mesh().build().rotated_by(Quat::from_rotation_arc(Vec3::Y, along)).translated_by(end - along * 0.04));
    }
    vec![
        (rod(low, high, 0.045), [0.45, 0.27, 0.13], true),
        (rod(high - along * 0.01, high + along * 0.01, 0.05), [0.25, 0.14, 0.07], true),
        (merged(arrows), [0.55, 0.4, 0.22], false),
        (merged(feathers), [0.9, 0.88, 0.82], false),
    ]
}

/// A full sack on the back, tied at the neck, hanging from a rope over the right shoulder.
pub fn sack(rest: &Rest, points: &[Vec3]) -> Parts {
    let y = rest.chest.y - 0.3;
    // Round below, gathered and tied at the top, flatter against the back.
    let grid = crate::hat::rings(16, 0.0, |t| {
        let r = if t < 0.8 { 0.17 * (std::f32::consts::PI * (0.08 + 0.92 * t / 0.8)).sin().sqrt() } else { 0.03 + 0.1 * (t - 0.8) };
        (0.45 * t, r, r * 0.7)
    });
    let lean = Quat::from_rotation_z(0.35);
    let at = Vec3::new(0.0, y, back_at(rest, points, 0.0, y) - 0.12);
    let grid: Grid = grid.into_iter().map(|row| row.into_iter().map(|p| at + lean * p).collect()).collect();
    let neck = at + lean * Vec3::Y * 0.37;
    // Over the right shoulder and down the chest in front.
    let shoulder = Vec3::new(-0.13, rest.left_shoulder.y + 0.04, rest.left_shoulder.z);
    let y = rest.chest.y + 0.05;
    let front = Vec3::new(-0.11, y, middle(rest) + reach(points, Vec3::new(-0.11, y, middle(rest)), Vec3::Z, 0.12) + 0.015);
    let rope = merged([rod(neck, shoulder, 0.012), rod(shoulder, front, 0.012), Sphere::new(0.012).mesh().build().translated_by(shoulder)]);
    vec![
        (crate::hat::mesh(&grid, false, Vec2::new(3.0, 2.0)), [0.62, 0.5, 0.32], true),
        (rod(neck - lean * Vec3::Y * 0.01, neck + lean * Vec3::Y * 0.02, 0.036), [0.35, 0.25, 0.12], false),
        (rope, [0.35, 0.25, 0.12], false),
    ]
}

/// A black sheath across the back, the hilt standing over the right shoulder.
pub fn scabbard(rest: &Rest, points: &[Vec3]) -> Parts {
    let (low, high) = slant(rest, points, 0.03);
    let along = (high - low).normalize();
    let guard = Cylinder::new(0.035, 0.008).mesh().build().rotated_by(Quat::from_rotation_arc(Vec3::Y, along)).translated_by(high + along * 0.005);
    vec![
        (rod(low, high, 0.022), [0.06, 0.06, 0.07], true),
        (guard, [0.5, 0.42, 0.25], false),
        (rod(high + along * 0.01, high + along * 0.16, 0.014), [0.15, 0.08, 0.06], true),
        (Sphere::new(0.018).mesh().build().translated_by(high + along * 0.17), [0.5, 0.42, 0.25], false),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robe::peasant_rest as rest;

    #[test]
    fn a_cape_hangs_behind_from_the_shoulders() {
        let c = cape(&rest(), &[], 0.45, 0.0);
        let (hem, high) = (&c[0], c.last().unwrap());
        assert!(high[0].y > rest().chest.y + 0.1 && hem[0].y < 0.5);
        assert!(c.iter().flatten().all(|p| p.z < middle(&rest())), "all behind the middle of the body");
        let width = |row: &[Vec3]| row.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        assert!(width(hem) > width(high), "wider at the hem");
    }

    #[test]
    fn a_cape_hangs_from_the_back_not_the_sleeves() {
        // A rest-pose sleeve, out sideways at the shoulders.
        let sleeve: Vec<Vec3> = (0..20).map(|i| 0.2 + 0.02 * i as f32).map(|r| Vec3::new(0.84 * r, 1.45, middle(&rest()) - 0.54 * r)).collect();
        let c = cape(&rest(), &sleeve, 0.45, 0.0);
        let widest = c.iter().flatten().map(|p| p.x.abs()).fold(0.0, f32::max);
        assert!(widest < 0.35, "not out to the sleeve: {widest}");
    }

    #[test]
    fn the_sack_hangs_on_the_back_from_a_rope_over_the_right_shoulder() {
        let parts = sack(&rest(), &[]);
        let points: Vec<Vec3> = parts
            .iter()
            .flat_map(|(m, ..)| match m.attribute(Mesh::ATTRIBUTE_POSITION) {
                Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) => p.iter().map(|v| Vec3::from_array(*v)).collect(),
                _ => Vec::new(),
            })
            .collect();
        assert!(points.iter().any(|p| p.z > rest().chest.z + 0.05 && p.x < 0.0), "the rope comes over to the front");
        let deepest = points.iter().map(|p| p.z).fold(f32::MAX, f32::min);
        assert!(deepest > middle(&rest()) - 0.4, "close to the back: {deepest}");
    }

    #[test]
    fn a_cape_faces_out() {
        let c = cape(&rest(), &[], 0.45, 0.0);
        let n = crate::hat::normals(&c, false);
        let mid = c.len() / 2;
        assert!(n[mid][AROUND / 4].z < -0.9, "the outside faces back: {}", n[mid][AROUND / 4]);
    }

    #[test]
    fn a_cape_moves_with_the_chest_above_and_the_hips_below() {
        let c = cape(&rest(), &[], 0.45, 0.0);
        let w = cape_weights(&c, &rest());
        assert!(w.iter().flatten().all(|p| (p.iter().sum::<f32>() - 1.0).abs() < 1e-5 && p.iter().all(|v| *v >= 0.0)));
        assert!(w.last().unwrap()[0][0] > 0.99, "the chest at the top");
        assert!(w[0][0][1] > 0.4, "the hips at the hem");
    }

    #[test]
    fn the_quiver_and_sword_stand_over_the_right_shoulder() {
        for parts in [quiver(&rest(), &[]), scabbard(&rest(), &[])] {
            let highest = parts
                .iter()
                .filter_map(|(m, ..)| match m.attribute(Mesh::ATTRIBUTE_POSITION) {
                    Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) => p.iter().map(|v| Vec3::from_array(*v)).max_by(|a, b| a.y.total_cmp(&b.y)),
                    _ => None,
                })
                .max_by(|a, b| a.y.total_cmp(&b.y))
                .unwrap();
            assert!(highest.x < -0.05 && highest.y > rest().left_shoulder.y + 0.05, "{highest}");
        }
    }
}
