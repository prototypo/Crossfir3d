//! What classes wear on their backs and over their clothes: capes, a fur cloak, aprons,
//! tabards and sashes (skinned to the outfit's bones, like the robe), and a quiver, a
//! sack and a sheathed sword (fixed to the chest).
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

/// A cape from over the shoulders down to `hem` (a height), with a ragged hem `teeth` deep.
pub fn cape(rest: &Rest, points: &[Vec3], hem: f32, teeth: f32) -> Grid {
    panel(rest, points, (rest.neck.y - 0.04, hem), (0.38, 0.46), false, teeth, 0.0)
}

/// A leather apron from the chest to the knees, wrapping a little round the hips.
pub fn apron(rest: &Rest, points: &[Vec3]) -> Grid {
    panel(rest, points, (rest.chest.y + 0.08, rest.pelvis.y - 0.38), (0.26, 0.3), true, 0.0, 0.0)
}

/// A tabard: a straight panel in front from the collar and one behind from the shoulder
/// blades, down to the knees.
pub fn tabard(rest: &Rest, points: &[Vec3]) -> [Grid; 2] {
    let hem = rest.pelvis.y - 0.42;
    [(true, rest.neck.y - 0.07), (false, rest.left_shoulder.y - 0.06)].map(|(front, high)| panel(rest, points, (high, hem), (0.26, 0.26), front, 0.0, 0.0))
}

/// The unit direction `a` radians round the X axis: 0 to the front (+Z), a half turn to the back.
fn out(a: f32) -> Vec3 {
    Vec3::new(0.0, a.sin(), a.cos())
}

/// How far out the clothes are at each of the 25 angle steps round `axis` (the head and neck left out); 0 where none.
fn cloth_profile(points: &[Vec3], axis: Vec3, x: f32, neck_y: f32) -> Vec<f32> {
    let steps = 24;
    (0..=steps)
        .map(|k| {
            let dir = out(PI * k as f32 / steps as f32);
            points
                .iter()
                .filter(|p| (p.x - x).abs() < 0.03 && p.y < neck_y - 0.03)
                .map(|p| (*p - axis).with_x(0.0))
                .filter(|v| v.normalize_or_zero().dot(dir) > 0.966)
                .map(|v| v.length())
                .fold(0.0, f32::max)
        })
        .collect()
}

/// Fills the zero gaps in a profile by linear interpolation, copying the nearest value at the ends (0.15 if all are empty).
fn fill_gaps(far: &mut [f32]) {
    let known: Vec<usize> = (0..far.len()).filter(|&k| far[k] > 0.0).collect();
    for k in 0..far.len() {
        if far[k] == 0.0 {
            let before = known.iter().rev().find(|&&i| i < k);
            let after = known.iter().find(|&&i| i > k);
            far[k] = match (before, after) {
                (Some(&i), Some(&j)) => far[i] + (far[j] - far[i]) * (k - i) as f32 / (j - i) as f32,
                (Some(&i), None) | (None, Some(&i)) => far[i],
                (None, None) => 0.15,
            };
        }
    }
}

/// The profile's value at angle `a`, interpolated between steps.
fn profile_at(far: &[f32], a: f32) -> f32 {
    let steps = far.len() - 1;
    let k = (a / PI * steps as f32).clamp(0.0, steps as f32);
    let (i, f) = (k.floor() as usize, k.fract());
    far[i] + (far[(i + 1).min(steps)] - far[i]) * f
}

/// The strap's 25 points from `a` over the cloth to `b`, round `axis`.
fn strap_path(far: &[f32], axis: Vec3, a: Vec3, b: Vec3) -> Vec<Vec3> {
    let angle = |p: Vec3| (p.y - axis.y).atan2(p.z - axis.z);
    let (from, to) = (angle(a), angle(b));
    let ends = [(a - axis).with_x(0.0).length(), (b - axis).with_x(0.0).length()];
    (0..=24)
        .map(|k| {
            let t = k as f32 / 24.0;
            let a = from + (to - from) * t;
            // Just above the cloth, out to the panels at the ends.
            let lie = profile_at(far, a) + 0.006;
            let end = if t < 0.5 { ends[0] } else { ends[1] };
            let r = lie + (end - lie) * smooth(1.0 - t.min(1.0 - t) / 0.15);
            axis + r * out(a)
        })
        .collect()
}

/// A thin cuboid along each segment of the path.
fn strap_segments(path: &[Vec3]) -> Vec<Mesh> {
    path.windows(2)
        .map(|w| {
            let along = w[1] - w[0];
            let length = along.length() + 0.004;
            let along = along.normalize();
            let turn = Quat::from_mat3(&Mat3::from_cols(Vec3::X, along, Vec3::X.cross(along)));
            Cuboid::new(0.024, length, 0.003).mesh().build().rotated_by(turn).translated_by((w[0] + w[1]) / 2.0)
        })
        .collect()
}

/// Thin flat straps over the shoulders, joining the tabard's front and back at their top
/// corners, just above the clothes.
pub fn tabard_straps(rest: &Rest, points: &[Vec3], [front, back]: &[Grid; 2]) -> Mesh {
    let (front, back) = (front.last().expect("a top row"), back.last().expect("a top row"));
    // The front's top row runs left to right, the back's right to left.
    let pairs = [(front[0], back[AROUND / 2 - 1]), (front[AROUND / 2 - 1], back[0])];
    merged(pairs.into_iter().flat_map(|(a, b)| {
        // Along the panels' inner edges; angles round a line along X through the upper chest,
        // 0 to the front, a half turn to the back.
        let x = (a.x + b.x) / 2.0 - a.x.signum() * 0.015;
        let axis = Vec3::new(x, rest.chest.y + 0.02, middle(rest) - 0.01);
        // How far out the clothes are at each angle, gaps filled in.
        let mut far = cloth_profile(points, axis, x, rest.neck.y);
        fill_gaps(&mut far);
        strap_segments(&strap_path(&far, axis, a, b))
    }))
}

/// A creature's rags: torn cloth all round from the neck to the thighs, front and back.
pub fn rags(rest: &Rest, points: &[Vec3]) -> [Grid; 2] {
    // Up to the neck, over the shirt's collar.
    let span = (rest.neck.y, rest.pelvis.y - 0.3);
    // Loose: a hunched creature's chest pulls the cloth in at the waist.
    [true, false].map(|front| panel(rest, points, span, (0.5, 0.5), front, 0.12, 0.035))
}

/// A ragged sleeve round a limb bone, from just before `shoulder` toward `elbow`, `long`
/// of the way, its end torn. Rows go from the shoulder out.
pub fn sleeve(points: &[Vec3], shoulder: Vec3, elbow: Vec3, long: f32) -> Grid {
    let along = (elbow - shoulder).normalize();
    // Round the arm, so that the rows' normals face out.
    let u = along.any_orthonormal_vector();
    let v = u.cross(along);
    let rows = 8;
    let (from, to) = (-0.05, long * shoulder.distance(elbow));
    (0..=rows)
        .map(|i| {
            let t = i as f32 / rows as f32;
            let centre = shoulder + along * (from + (to - from) * t);
            (0..AROUND)
                .map(|j| {
                    let a = std::f32::consts::TAU * j as f32 / AROUND as f32;
                    let dir = u * a.cos() + v * a.sin();
                    // The arm's own reach this way, near this ring.
                    let arm = points
                        .iter()
                        .map(|p| *p - centre)
                        .filter(|d| d.dot(along).abs() < 0.025)
                        .map(|d| d.reject_from(along))
                        .filter(|d| d.normalize_or_zero().dot(dir) > 0.966)
                        .map(|d| d.length())
                        .fold(0.05, f32::max)
                        .min(0.12);
                    let torn = if i == rows { 0.06 * (j * 37 % 11) as f32 / 10.0 } else { 0.0 };
                    centre + dir * (arm + 0.015) - along * torn
                })
                .collect()
        })
        .collect()
}

/// Cloth from `high` down to `hem` (heights), in front of the body or behind it, hanging
/// from the widest part above and flaring a little; `spread` is how far round the body it
/// reaches at the top and the hem (in half turns). A thin double sheet, so a ring: across
/// the outside from the right to the left, back across the inside. `teeth`: a ragged hem
/// this deep. Rows go from the hem up, so its normals face out.
/// `loose`: this much further out from the body.
fn panel(rest: &Rest, points: &[Vec3], (high, hem): (f32, f32), spread: (f32, f32), front: bool, teeth: f32, loose: f32) -> Grid {
    let rows = 16;
    let half = AROUND / 2;
    let side = if front { 1.0 } else { -1.0 };
    let mut hang = vec![0.0f32; half];
    let mut grid: Grid = (0..=rows)
        .map(|i| {
            // 0 at the top, 1 at the hem.
            let t = i as f32 / rows as f32;
            let y = high + (hem - high) * t;
            let centre = Vec3::new(0.0, y, middle(rest));
            let spread = spread.0 + (spread.1 - spread.0) * t;
            let outside: Vec<Vec3> = (0..half)
                .map(|s| {
                    // Behind, from the right (-X) round to the left (+X); in front, the other
                    // way, so the outside always faces out.
                    let u = s as f32 / (half - 1) as f32;
                    let u = if front { 1.0 - u } else { u };
                    let a = PI * (0.5 + spread * (1.0 - 2.0 * u));
                    let dir = Vec3::new(a.cos(), 0.0, side * a.sin());
                    // Cloth hangs from the widest part above.
                    // Not out along the rest pose's sleeves; clear of the buttons and buckle in front.
                    // Loose cloth is still tight round the neck, so nothing shows inside it.
                    let clear = if i == 0 && loose > 0.0 { 0.015 } else { (if front { 0.055 } else { 0.04 }) + loose };
                    hang[s] = hang[s].max(reach(points, centre, dir, 0.1).min(if dir.x.abs() > 0.6 { 0.21 } else { 1.0 }) + clear);
                    // Folds, deeper toward the hem.
                    let folds = 0.012 * t * (1.0 - (u * 7.0 * TAU).cos());
                    let mut p = centre + (hang[s] + 0.05 * t * t + folds) * dir;
                    // Over the shoulders.
                    if i == 0 && high > rest.chest.y + 0.1 {
                        p.y = p.y.max(top(points, p, rest.neck.y - 0.03) + 0.012);
                    }
                    // Torn unevenly.
                    if i == rows {
                        p.y += teeth * (s * 37 % 11) as f32 / 10.0;
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

/// A wide band round the waist over the belt, rows from the bottom up.
pub fn sash(rest: &Rest, points: &[Vec3]) -> Grid {
    (0..=3)
        .map(|i| {
            let centre = Vec3::new(0.0, rest.pelvis.y + 0.04 + 0.03 * i as f32, middle(rest));
            (0..AROUND).map(|j| centre + (reach(points, centre, crate::hat::ring(j), 0.15) + 0.02) * crate::hat::ring(j)).collect()
        })
        .collect()
}

/// The sash's knot and two ends, hanging at the left hip.
pub fn sash_ends(rest: &Rest, points: &[Vec3]) -> Mesh {
    let y = rest.pelvis.y + 0.08;
    let dir = Vec3::new(0.8, 0.0, 0.6).normalize();
    let knot = Vec3::new(0.0, y, middle(rest)) + (reach(points, Vec3::new(0.0, y, middle(rest)), dir, 0.15) + 0.035) * dir;
    let end = |lean: f32, long: f32| {
        Cuboid::new(0.055, long, 0.008)
            .mesh()
            .build()
            .translated_by(Vec3::Y * -long / 2.0)
            .rotated_by(Quat::from_rotation_arc(Vec3::Z, dir) * Quat::from_rotation_z(lean))
            .translated_by(knot)
    };
    merged([Sphere::new(1.0).mesh().uv(12, 8).scaled_by(Vec3::new(0.04, 0.035, 0.03)).translated_by(knot), end(0.15, 0.26), end(-0.2, 0.2)])
}

/// The apron's straps: from its top corners round the back of the neck.
pub fn apron_straps(rest: &Rest, apron: &Grid) -> Mesh {
    let high = apron.last().expect("the apron's top row");
    let neck = Vec3::new(0.0, rest.neck.y - 0.01, rest.neck.z - 0.06);
    merged([0, AROUND / 2 - 1].map(|s| {
        let corner = high[s];
        let over = Vec3::new(corner.x.signum() * 0.075, rest.neck.y - 0.03, rest.neck.z + 0.01);
        merged([rod(corner, over, 0.008), rod(over, neck, 0.008)])
    }))
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
/// Tangents are dropped: only some parts have them, and a merge keeps a part's own
/// attributes even when the others lack them, so they would no longer line up.
pub(crate) fn merged(parts: impl IntoIterator<Item = Mesh>) -> Mesh {
    let mut parts = parts.into_iter().map(|m| m.with_removed_attribute(Mesh::ATTRIBUTE_TANGENT));
    let mut out = parts.next().expect("at least one part");
    for m in parts {
        out.merge(&m).expect("the parts share attributes");
    }
    out
}

/// A rod of `radius` from `a` to `b`.
pub(crate) fn rod(a: Vec3, b: Vec3, radius: f32) -> Mesh {
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
    fn an_apron_hangs_in_front_and_faces_out() {
        let a = apron(&rest(), &[]);
        assert!(a.iter().flatten().all(|p| p.z > middle(&rest())), "all in front");
        assert!(a.last().unwrap()[0].y < rest().neck.y && a[0][0].y < rest().pelvis.y - 0.3);
        let n = crate::hat::normals(&a, false);
        let mid = a.len() / 2;
        assert!(n[mid][AROUND / 4].z > 0.9, "the outside faces forward: {}", n[mid][AROUND / 4]);
    }

    #[test]
    fn a_tabard_hangs_front_and_back() {
        let [front, back] = tabard(&rest(), &[]);
        assert!(front.iter().flatten().all(|p| p.z > middle(&rest())));
        assert!(back.iter().flatten().all(|p| p.z < middle(&rest())));
        assert!(back.last().unwrap()[0].y < rest().left_shoulder.y - 0.03, "not showing over the shoulders");
    }

    #[test]
    fn tabard_straps_lie_just_on_the_shoulders() {
        // A round shoulder on each side, 14 cm round a line along X at chest height.
        let axis = |x: f32| Vec3::new(x, rest().chest.y + 0.02, middle(&rest()) - 0.01);
        let shoulder: Vec<Vec3> = [-0.12f32, -0.1, 0.1, 0.12]
            .iter()
            .flat_map(|&x| (0..=36).map(move |k| axis(x) + 0.14 * Vec3::new(0.0, (PI * k as f32 / 36.0).sin(), (PI * k as f32 / 36.0).cos())))
            .collect();
        let t = tabard(&rest(), &shoulder);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) = tabard_straps(&rest(), &shoulder, &t).attribute(Mesh::ATTRIBUTE_POSITION).cloned() else { panic!() };
        let p: Vec<Vec3> = p.iter().map(|v| Vec3::from_array(*v)).collect();
        assert!(p.iter().any(|v| v.x > 0.08) && p.iter().any(|v| v.x < -0.08), "one on each side");
        // Over the top, each strap is just above the cloth.
        for v in p.iter().filter(|v| v.y > rest().chest.y + 0.1 && (v.z - axis(0.0).z).abs() < 0.08) {
            let off = (*v - axis(v.x)).with_x(0.0).length() - 0.14;
            assert!(off > 0.0 && off < 0.015, "{v}: {off}");
        }
        assert!(p.iter().map(|v| v.y).fold(f32::MIN, f32::max) > rest().chest.y + 0.14, "over the top");
    }

    #[test]
    fn rags_wrap_all_round_with_a_torn_hem() {
        let [front, back] = rags(&rest(), &[]);
        let all: Vec<Vec3> = front.iter().chain(&back).flatten().copied().collect();
        assert!(all.iter().any(|p| p.z > middle(&rest()) + 0.1) && all.iter().any(|p| p.z < middle(&rest()) - 0.1));
        assert!(all.iter().any(|p| p.x > 0.12) && all.iter().any(|p| p.x < -0.12), "round the sides too");
        let hem: Vec<f32> = front[0].iter().map(|p| p.y).collect();
        let (low, high) = (hem.iter().cloned().fold(f32::MAX, f32::min), hem.iter().cloned().fold(f32::MIN, f32::max));
        assert!(high - low > 0.05, "a torn hem");
        assert!(low < rest().pelvis.y - 0.2, "down over the thighs");
    }

    #[test]
    fn a_ragged_sleeve_wraps_the_upper_arm_and_faces_out() {
        let (shoulder, elbow) = (Vec3::new(0.19, 1.45, -0.06), Vec3::new(0.45, 1.42, -0.07));
        let g = sleeve(&[], shoulder, elbow, 0.97);
        let along = (elbow - shoulder).normalize();
        let n = crate::hat::normals(&g, false);
        let mid = g.len() / 2;
        for (p, n) in g[mid].iter().zip(&n[mid]) {
            let out = (*p - shoulder).reject_from(along);
            assert!(out.length() > 0.04 && n.dot(out) > 0.0, "round the arm, facing out: {p}");
        }
        let ends: Vec<f32> = g.last().unwrap().iter().map(|p| (*p - shoulder).dot(along)).collect();
        assert!(ends.iter().cloned().fold(f32::MIN, f32::max) - ends.iter().cloned().fold(f32::MAX, f32::min) > 0.02, "a torn end");
        assert!(g[0].iter().all(|p| (*p - shoulder).dot(along) < 0.0), "starting over the shoulder");
    }

    #[test]
    fn the_sash_rings_the_waist() {
        let s = sash(&rest(), &[]);
        let all: Vec<Vec3> = s.iter().flatten().copied().collect();
        assert!(all.iter().all(|p| p.y > rest().pelvis.y - 0.02 && p.y < rest().pelvis.y + 0.2));
        assert!(all.iter().any(|p| p.x > 0.1) && all.iter().any(|p| p.x < -0.1));
        assert!(all.iter().any(|p| p.z > middle(&rest()) + 0.1) && all.iter().any(|p| p.z < middle(&rest()) - 0.1));
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

    #[test]
    fn gaps_in_the_profile_are_filled() {
        let mut far = [0.0, 0.2, 0.0, 0.0, 0.5, 0.0];
        fill_gaps(&mut far);
        assert!((far[2] - 0.3).abs() < 1e-6 && (far[3] - 0.4).abs() < 1e-6);
        assert!(far[0] == 0.2 && far[5] == 0.5, "the ends copy their nearest");
        let mut none = [0.0; 4];
        fill_gaps(&mut none);
        assert_eq!(none, [0.15; 4]);
    }
}
