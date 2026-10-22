//! Built headwear: the soft leather pointed hat (a wavy, drooping brim and a crown whose
//! tip flops back), the tricorn, and the ninja's face cloth.
//! Sizes are for a human head bone (+Y up, +Z the face).

use std::f32::consts::TAU;

use bevy::prelude::*;
use crossfir3d::held::Headwear;

/// Points of a surface, row by row; each row goes once around (its last point joins its first).
type Grid = Vec<Vec<Vec3>>;

const AROUND: usize = 48;
const BRIM_Y: f32 = 0.2;

/// The crown: rings up a centre line that stands straight, then bends back and down.
fn crown() -> Grid {
    let (rows, length) = (32, 0.44);
    let mut at = Vec3::new(0.0, BRIM_Y - 0.01, -0.02);
    // It flops back and to the left, so it shows from the camera behind the player.
    let flop = Vec3::new(0.6, 0.0, -0.8);
    let across = Vec3::Y.cross(flop);
    let mut out = Vec::new();
    for i in 0..=rows {
        let t = i as f32 / rows as f32;
        // Straight for the first 35%, then bending back until the tip hangs down.
        let bend = 3.0 * ((t - 0.35) / 0.65).clamp(0.0, 1.0).powf(1.6);
        let along = bend.cos() * Vec3::Y + bend.sin() * flop;
        let side = bend.cos() * flop - bend.sin() * Vec3::Y;
        // Dents start above the band; a pinch where it folds.
        let dents = 0.07 * ((t - 0.15) / 0.3).clamp(0.0, 1.0);
        let pinch = 1.0 - 0.1 * (-((t - 0.58) / 0.08).powi(2)).exp();
        let radius = (0.13 * (1.0 - t).powf(0.8)).max(0.003) * pinch;
        out.push(
            (0..AROUND)
                .map(|j| {
                    let a = TAU * j as f32 / AROUND as f32;
                    let r = radius * (1.0 + dents * (3.0 * a + 5.0 * t).sin());
                    at + r * (a.cos() * across + a.sin() * side)
                })
                .collect(),
        );
        at += along * length / rows as f32;
    }
    out
}

/// The brim: a thin, round-edged ring whose outer edge droops in waves.
fn brim() -> Grid {
    let (inner, outer, thick, rows) = (0.1, 0.26, 0.009, 16);
    (0..rows)
        .map(|i| {
            // Around the brim's cross-section: outer edge, top, inner edge, bottom.
            let v = TAU * i as f32 / rows as f32;
            let d = (inner + outer) / 2.0 + (outer - inner) / 2.0 * v.cos();
            let q = ((d - inner) / (outer - inner)).powi(2);
            (0..AROUND)
                .map(|j| {
                    let a = TAU * j as f32 / AROUND as f32;
                    let droop = 0.06 + 0.025 * (3.0 * a + 0.7).sin() + 0.012 * (5.0 * a + 2.0).sin();
                    Vec3::new(d * a.cos(), BRIM_Y + thick * v.sin() - droop * q, d * a.sin())
                })
                .collect()
        })
        .collect()
}

/// The hat band: a short upright ring where the crown meets the brim.
fn band() -> Grid {
    [BRIM_Y - 0.005, BRIM_Y + 0.035]
        .map(|y| (0..AROUND).map(|j| Vec3::new(0.0, y, -0.02) + 0.128 * ring(j)).collect())
        .into()
}

fn ring(j: usize) -> Vec3 {
    let a = TAU * j as f32 / AROUND as f32;
    Vec3::new(a.cos(), 0.0, a.sin())
}

/// One normal per point, pointing out: along the rows times across them.
/// `closed` grids also wrap from the last row to the first.
fn normals(grid: &Grid, closed: bool) -> Grid {
    let rows = grid.len();
    let row = |i: isize| -> &Vec<Vec3> {
        if closed {
            &grid[i.rem_euclid(rows as isize) as usize]
        } else {
            &grid[i.clamp(0, rows as isize - 1) as usize]
        }
    };
    (0..rows as isize)
        .map(|i| {
            (0..AROUND)
                .map(|j| {
                    let across = row(i)[(j + 1) % AROUND] - row(i)[(j + AROUND - 1) % AROUND];
                    let along = row(i + 1)[j] - row(i - 1)[j];
                    along.cross(across).normalize_or(Vec3::Y)
                })
                .collect()
        })
        .collect()
}

/// A mesh of the grid with `repeat` texture tiles around and along it.
fn mesh(grid: &Grid, closed: bool, repeat: Vec2) -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};
    let normal = normals(grid, closed);
    let rows = grid.len() + closed as usize;
    let (mut points, mut norms, mut uvs, mut indices) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for i in 0..rows {
        let r = i % grid.len();
        for j in 0..=AROUND {
            points.push(grid[r][j % AROUND]);
            norms.push(normal[r][j % AROUND]);
            uvs.push(Vec2::new(j as f32 / AROUND as f32, i as f32 / (rows - 1) as f32) * repeat);
        }
    }
    let w = (AROUND + 1) as u32;
    for i in 0..rows as u32 - 1 {
        for j in 0..AROUND as u32 {
            let (a, b, c, d) = (i * w + j, i * w + j + 1, (i + 1) * w + j, (i + 1) * w + j + 1);
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, norms)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices));
    mesh.generate_tangents().expect("the hat mesh has normals and texture places");
    mesh
}

const LEATHER_PX: u32 = 256;

/// Smooth noise from 0 to 1 that repeats every `cells` cells across the picture.
fn noise(x: f32, y: f32, cells: u32, seed: u32) -> f32 {
    let hash = |i: u32, k: u32| {
        let mut h = (i % cells).wrapping_mul(374_761_393) ^ (k % cells).wrapping_mul(668_265_263) ^ seed.wrapping_mul(2_246_822_519);
        h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let (fx, fy) = (x * cells as f32, y * cells as f32);
    let (i, k) = (fx.floor() as u32, fy.floor() as u32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(fx.fract()), s(fy.fract()));
    let top = hash(i, k) * (1.0 - u) + hash(i + 1, k) * u;
    let bottom = hash(i, k + 1) * (1.0 - u) + hash(i + 1, k + 1) * u;
    top * (1.0 - v) + bottom * v
}

/// Leather's surface height: small pebbles cut by fine creases.
fn grain(x: f32, y: f32) -> f32 {
    let crease = (1.0 - (2.0 * noise(x, y, 24, 3) - 1.0).abs()).powi(6);
    0.6 * noise(x, y, 48, 1) + 0.3 * noise(x, y, 96, 2) - 0.6 * crease
}

/// The leather pictures as RGBA: light mottled colour (the hat's colour tints it)
/// and the matching normal map. Both tile.
pub fn leather_pixels() -> ((u32, u32, Vec<u8>), (u32, u32, Vec<u8>)) {
    let n = LEATHER_PX;
    let step = 1.0 / n as f32;
    let (mut colour, mut bumps) = (Vec::new(), Vec::new());
    for py in 0..n {
        for px in 0..n {
            let (x, y) = (px as f32 * step, py as f32 * step);
            let h = grain(x, y);
            let shade = (0.8 + 0.25 * noise(x, y, 5, 4) + 0.12 * h).clamp(0.0, 1.0);
            colour.extend([shade, shade * 0.97, shade * 0.93].map(|c| (c * 255.0) as u8).into_iter().chain([255]));
            // Wrapping neighbours keep the slope right at the picture's edges.
            let dx = grain((x + step) % 1.0, y) - grain((x + 1.0 - step) % 1.0, y);
            let dy = grain(x, (y + step) % 1.0) - grain(x, (y + 1.0 - step) % 1.0);
            let normal = Vec3::new(-dx * 6.0, -dy * 6.0, 1.0).normalize();
            bumps.extend(normal.to_array().map(|c| ((c * 0.5 + 0.5) * 255.0) as u8).into_iter().chain([255]));
        }
    }
    ((n, n, colour), (n, n, bumps))
}

/// The leather pictures, made once.
#[derive(Resource)]
pub struct Leather {
    colour: Handle<Image>,
    bumps: Handle<Image>,
}

impl FromWorld for Leather {
    fn from_world(world: &mut World) -> Self {
        use bevy::image::ImageSamplerDescriptor;
        use bevy::render::render_resource::TextureFormat;
        let (colour, bumps) = leather_pixels();
        let mut images = world.resource_mut::<Assets<Image>>();
        let colour = images.add(crate::world::tiled(colour, ImageSamplerDescriptor::linear()));
        let mut bumps = crate::world::tiled(bumps, ImageSamplerDescriptor::linear());
        bumps.texture_descriptor.format = TextureFormat::Rgba8Unorm; // a normal map is not a colour
        Leather { colour, bumps: images.add(bumps) }
    }
}

/// The tricorn: a low round crown, and a brim turned up on three sides so that from
/// above it is a triangle with one corner to the front.
fn tricorn() -> [Grid; 2] {
    // Lower than the pointed hat: it sits down over the hair.
    let centre = Vec3::new(0.0, -0.04, -0.01);
    let crown = (0..=16)
        .map(|i| {
            let up = std::f32::consts::FRAC_PI_2 * i as f32 / 16.0;
            let r = (0.125 * up.cos().powf(0.6)).max(0.002);
            (0..AROUND).map(|j| centre + Vec3::new(0.0, BRIM_Y - 0.005 + 0.12 * up.sin(), 0.0) + r * ring(j)).collect()
        })
        .collect();
    let third = TAU / 3.0;
    let brim = (0..=10)
        .map(|i| {
            // Out from under the crown, then curling up.
            let curl = std::f32::consts::FRAC_PI_2 * i as f32 / 10.0;
            (0..AROUND)
                .map(|j| {
                    let a = TAU * j as f32 / AROUND as f32;
                    // How far round from the middle of a side: 0 there, ±60° at a corner.
                    let off = (a - std::f32::consts::FRAC_PI_2).rem_euclid(third) - third / 2.0;
                    // A triangle's edge, its corners rounded off.
                    let sharp = 0.15 / off.cos();
                    let reach = (sharp.powi(-6) + 0.25f32.powi(-6)).powf(-1.0 / 6.0);
                    let rise = 0.04 + 0.075 * (off * 1.5).cos().powi(2);
                    let d = 0.12 + (reach - 0.12) * curl.sin();
                    centre + Vec3::new(0.0, BRIM_Y - 0.01 + rise * (1.0 - curl.cos()), 0.0) + d * ring(j)
                })
                .collect()
        })
        .collect();
    [crown, brim]
}

/// The ninja's face cloth: a band round the head from the nose bridge down under the chin.
fn mask() -> Grid {
    // (height, half width, half depth, centre's depth), top to bottom.
    let rows = [(0.105, 0.09, 0.105, 0.01), (0.07, 0.088, 0.108, 0.012), (0.03, 0.082, 0.104, 0.012), (0.0, 0.078, 0.1, 0.012), (-0.035, 0.07, 0.09, 0.0), (-0.07, 0.062, 0.068, -0.01)];
    rows.iter()
        .map(|&(y, wide, deep, z)| (0..AROUND).map(|j| Vec3::new(0.0, y, z) + ring(j) * Vec3::new(wide, 0.0, deep)).collect())
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Finish {
    Leather,
    Cloth,
    Brass,
}

/// Puts a built hat or mask on a head bone, `k` times the size for a human; `marker` goes on
/// each part. Other headwear is not built here: false.
#[allow(clippy::too_many_arguments)]
pub fn put_on(
    commands: &mut Commands,
    head: Entity,
    wear: Headwear,
    k: f32,
    leather: &Leather,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    marker: impl Bundle + Clone,
) -> bool {
    let dark = |[r, g, b]: [f32; 3]| [r * 0.45, g * 0.45, b * 0.45];
    // (mesh, colour, finish)
    let parts: Vec<(Mesh, [f32; 3], Finish)> = match wear {
        Headwear::PointedHat(c) => vec![
            (mesh(&crown(), false, Vec2::new(3.0, 2.0)), c, Finish::Leather),
            (mesh(&brim(), true, Vec2::new(5.0, 1.0)), c, Finish::Leather),
            (mesh(&band(), false, Vec2::new(6.0, 0.25)), dark(c), Finish::Leather),
        ],
        Headwear::Tricorn(c) => {
            let [crown, brim] = tricorn();
            // Brass buttons on the two front sides, near the top edge.
            let top = brim.last().unwrap();
            let buttons = [AROUND / 4 + AROUND / 6, AROUND / 4 - AROUND / 6].map(|j| {
                let at = top[j] - Vec3::Y * 0.015 + ring(j) * 0.004;
                (Sphere::new(0.009).mesh().build().translated_by(at), [0.85, 0.65, 0.25], Finish::Brass)
            });
            [(mesh(&crown, false, Vec2::new(3.0, 1.0)), c, Finish::Leather), (mesh(&brim, false, Vec2::new(5.0, 0.5)), c, Finish::Leather)].into_iter().chain(buttons).collect()
        }
        Headwear::Mask => vec![(mesh(&mask(), false, Vec2::new(4.0, 0.3)), [0.05, 0.05, 0.06], Finish::Cloth)],
        _ => return false,
    };
    for (m, [r, g, b], finish) in parts {
        let leathered = finish == Finish::Leather;
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            base_color_texture: leathered.then(|| leather.colour.clone()),
            normal_map_texture: leathered.then(|| leather.bumps.clone()),
            perceptual_roughness: match finish {
                Finish::Leather => 0.7,
                Finish::Cloth => 0.95,
                Finish::Brass => 0.35,
            },
            metallic: if finish == Finish::Brass { 0.8 } else { 0.0 },
            reflectance: 0.35,
            // A turned-up brim shows its inside.
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        let part = (Mesh3d(meshes.add(m)), MeshMaterial3d(material), Transform::from_scale(Vec3::splat(k)), marker.clone());
        commands.entity(head).queue_silenced(move |mut e: EntityWorldMut| {
            e.with_child(part);
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn centre(row: &[Vec3]) -> Vec3 {
        row.iter().sum::<Vec3>() / row.len() as f32
    }

    #[test]
    fn the_tip_flops_back_and_down() {
        let c = crown();
        let top = c.iter().map(|r| centre(r).y).fold(f32::MIN, f32::max);
        let tip = centre(c.last().unwrap());
        assert!(tip.y < top - 0.03, "tip {tip} below the highest point {top}");
        assert!(tip.z < -0.12, "tip {tip} hangs behind the head");
        assert!(centre(&c[0]).y < 0.21, "the crown starts on the brim");
    }

    #[test]
    fn the_brim_droops_unevenly() {
        let b = brim();
        let rim: Vec<f32> = (0..AROUND).map(|j| b.iter().map(|r| r[j].y).fold(f32::MAX, f32::min)).collect();
        assert!(rim.iter().all(|y| *y < 0.18), "the outer edge hangs below the inner edge");
        let (lo, hi) = rim.iter().fold((f32::MAX, f32::MIN), |(lo, hi), y| (lo.min(*y), hi.max(*y)));
        assert!(hi - lo > 0.02, "the droop changes around the hat");
    }

    #[test]
    fn the_tricorn_turns_up_three_sides() {
        let [_, brim] = tricorn();
        let top = brim.last().unwrap();
        // Rows go from the crown out and up: the last row is the brim's top edge.
        let corner = top[AROUND / 4]; // the front point
        let side = top[AROUND / 4 + AROUND / 6]; // halfway to the next corner
        assert!(corner.xz().length() > side.xz().length() + 0.05, "the corners reach out");
        assert!(side.y > corner.y + 0.03, "the sides stand higher than the corners");
        assert!(corner.z > 0.15, "a corner points forward");
    }

    #[test]
    fn the_mask_covers_the_lower_face() {
        let m = mask();
        let front = |row: &Vec<Vec3>| row[AROUND / 4];
        assert!(m.iter().any(|r| front(r).y > 0.09), "up to the nose");
        assert!(m.iter().any(|r| front(r).y < 0.0), "down past the chin");
        assert!(m.iter().all(|r| front(r).z > 0.05), "in front of the face");
    }

    #[test]
    fn normals_point_out() {
        let c = crown();
        let n = normals(&c, false);
        for (p, n) in c[2].iter().zip(&n[2]) {
            assert!(n.dot(Vec3::new(p.x, 0.0, p.z + 0.02)) > 0.0, "crown side at {p}");
        }
        let b = brim();
        let n = normals(&b, true);
        // The highest point of the brim's section faces up.
        let top = (0..b.len()).max_by(|i, k| b[*i][0].y.total_cmp(&b[*k][0].y)).unwrap();
        assert!(n[top][0].y > 0.5);
    }
}
