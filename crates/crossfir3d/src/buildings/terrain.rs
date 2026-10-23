//! Mountains, volcanoes, magma, and lairs.

use super::*;

/// Mountains are as tall as a two-storey house: the hexagon mountain stretched up this much.
const MOUNTAIN_STRETCH: f32 = 2.4;

/// Mountain squares are raised this much (`terrain::ground_height`); the mountain starts
/// at the ground round it.
const MOUNTAIN_RAISE: f32 = 0.6;

const ROCKS: [&str; 4] =
    ["hexagon/rock_single_B.gltf", "hexagon/rock_single_C.gltf", "hexagon/rock_single_D.gltf", "hexagon/rock_single_E.gltf"];

/// A stretched hexagon mountain (`scale` wide, turned `yaw`), with rocks set into its
/// sides so the stretched faces look like rock again. `seed` picks where they go.
pub(super) fn mountain(file: &'static str, yaw: f32, scale: f32, seed: u32) -> Vec<Part> {
    let mut r = seed.wrapping_mul(2_654_435_761);
    let mut rand = || {
        r = r.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (r >> 16 & 0x7fff) as f32 / 32_768.0
    };
    let base = -MOUNTAIN_RAISE;
    let mut out = vec![part(Shape::Model(file), [0.0, base, 0.0], yaw, [scale, scale * MOUNTAIN_STRETCH, scale], None)];
    // Rocks round the sides, more of them low down (squared). Low ones stick out further
    // and are bigger; higher up they sit closer in and get smaller, as the mountain narrows.
    for i in 0..16 {
        let angle = (i as f32 * 22.5 + rand() * 20.0).to_radians();
        let up = 0.05 + rand().powi(2) * 0.9;
        let reach = 0.68 - up * 0.4;
        let pos = [angle.sin() * reach, base + up * 2.6, angle.cos() * reach];
        let size = (1.6 + rand() * 1.4) * (1.3 - up * 0.5);
        out.push(part(Shape::Model(ROCKS[i % ROCKS.len()]), pos, rand() * 360.0, [size, size * (0.8 + rand()), size], None));
    }
    out
}

/// Dark rock filling the floor square, its cracks glowing with lava as bright as `heat`.
pub(super) fn magma(heat: f32) -> Vec<Part> {
    let lava = [1.0 * heat, 0.4 * heat, 0.08 * heat];
    let mut out = vec![cube([1.0, 0.04, 1.0], [0.0, 0.02, 0.0], [0.12, 0.09, 0.08])];
    for (x, z, yaw, len) in [(-0.2, -0.2, 30.0, 0.5), (0.2, 0.1, -20.0, 0.6), (0.0, 0.3, 80.0, 0.4), (-0.3, 0.25, -50.0, 0.3)] {
        out.push(Part { yaw_deg: yaw, ..part(Shape::Glow([len, 0.006, 0.04]), [x, 0.042, z], 0.0, [1.0; 3], Some(lava)) });
    }
    out
}

/// A heap of rocks round a dark cave mouth that faces south.
pub(super) fn lair() -> Vec<Part> {
    vec![
        part(Shape::Model("hexagon/rock_single_E.gltf"), [-0.22, 0.0, -0.05], 30.0, [1.8, 3.0, 1.8], None),
        part(Shape::Model("hexagon/rock_single_C.gltf"), [0.24, 0.0, -0.05], 200.0, [2.0, 3.0, 2.0], None),
        part(Shape::Model("hexagon/rock_single_D.gltf"), [0.0, 0.0, -0.25], 90.0, [2.4, 4.5, 2.4], None),
        part(Shape::Model("hexagon/rock_single_B.gltf"), [0.0, 0.42, 0.0], 150.0, [2.2, 1.6, 2.0], None),
        cube([0.28, 0.36, 0.3], [0.0, 0.18, 0.05], INSIDE),
    ]
}

/// A mound of earth in two steps with a dark hole on top (ants, mice).
pub(super) fn nest() -> Vec<Part> {
    vec![
        cylinder(0.36, 0.14, [0.0, 0.07, 0.0], DIRT),
        cylinder(0.24, 0.26, [0.0, 0.13, 0.0], DIRT),
        cylinder(0.07, 0.02, [0.0, 0.26, 0.0], INSIDE),
    ]
}

const LAVA: [f32; 3] = [1.0, 0.42, 0.08];

/// A volcano filling the footprint: a brown cone, cut flat at the top, on a round of
/// grass, with ridges of rock down its sides. Glowing lava fills its crater, spurts up,
/// runs down between the ridges on the south side into a pool, and drips down the other
/// sides; a cloud of smoke hangs over it.
pub(super) fn volcano(w: u32, h: u32) -> Vec<Part> {
    let r = w.min(h) as f32 / 2.0;
    let (bottom, top) = (0.85 * r, 0.34 * r);
    let lean = ((bottom - top) / r).atan().to_degrees();
    // A box `len` long lying on the slope, `a` degrees round, its middle at height `y`,
    // `out` from the surface.
    let on_slope = |shape: fn([f32; 3]) -> Shape, a: f32, y: f32, size: [f32; 3], out: f32, color| {
        let d = bottom - (bottom - top) * y / r + out;
        let (sin, cos) = a.to_radians().sin_cos();
        Part { yaw_deg: a, tilt_deg: -lean, ..part(shape(size), [d * cos, y, d * sin], 0.0, [1.0; 3], Some(color)) }
    };
    let mut out = vec![
        cylinder(r, 0.06, [0.0, 0.03, 0.0], [0.45, 0.6, 0.25]),
        part(Shape::Frustum { radius: bottom, height: r }, [0.0, r / 2.0, 0.0], 0.0, [1.0; 3], Some([0.45, 0.27, 0.15])),
        cylinder(top * 1.05, 0.08 * r, [0.0, r, 0.0], [0.4, 0.22, 0.12]),
        part(Shape::RoundLamp { radius: top * 0.85, height: 0.04 }, [0.0, r + 0.04 * r, 0.0], 0.0, [1.0; 3], Some([1.0, 0.65, 0.15])),
        part(Shape::Fire { radius: 0.1 * r, height: 0.4 * r }, [0.0, 1.2 * r, 0.0], 0.0, [1.0; 3], Some([1.0, 0.6, 0.1])),
    ];
    // Ridges of rock, every 36 degrees, of different lengths, from the foot upward.
    for k in 0..10 {
        let len = r * [0.95, 0.7, 0.85, 0.6, 0.9, 0.75, 0.65, 0.95, 0.8, 0.7][k];
        let color = if k % 2 == 0 { [0.5, 0.31, 0.18] } else { [0.4, 0.24, 0.13] };
        let y = len / 2.0 * (lean.to_radians().cos());
        out.push(on_slope(Shape::Box, k as f32 * 36.0, y, [0.08 * r, len, 0.16 * r], 0.0, color));
    }
    // The stream: short pieces winding down between the ridges at 72 and 108 degrees,
    // wider as it goes down, then a pool on the grass.
    const PIECES: usize = 14;
    let piece = r / PIECES as f32 / lean.to_radians().cos() * 1.25;
    for i in 0..PIECES {
        let y = r * (1.0 - (i as f32 + 0.5) / PIECES as f32);
        let a = 90.0 + 6.0 * (i as f32 * 0.7).sin();
        out.push(on_slope(Shape::Glow, a, y, [0.03, piece, (0.1 + 0.007 * i as f32) * r], 0.015, LAVA));
    }
    out.push(part(Shape::RoundLamp { radius: 0.2 * r, height: 0.03 }, [0.0, 0.075, bottom + 0.1 * r], 0.0, [1.0; 3], Some(LAVA)));
    // Drips over the rim between the ridges, each ending in a blob.
    for (a, len) in [(18.0, 0.3), (162.0, 0.4), (198.0, 0.25), (234.0, 0.35), (306.0, 0.3), (342.0, 0.2)] {
        let len = len * r;
        let y = r - len / 2.0 * lean.to_radians().cos();
        out.push(on_slope(Shape::Glow, a, y, [0.03, len, 0.07 * r], 0.015, LAVA));
        let end = r - len * lean.to_radians().cos();
        let d = bottom - (bottom - top) * end / r + 0.02;
        let (sin, cos) = a.to_radians().sin_cos();
        out.push(part(Shape::Ball(0.05 * r), [d * cos, end, d * sin], 0.0, [1.0; 3], Some(LAVA)));
    }
    volcano_plume(&mut out, r);
    out
}

/// Pushes the lava jets above the crater and the cloud of smoke over them.
fn volcano_plume(out: &mut Vec<Part>, r: f32) {
    for (x, z, tilt) in [(-0.3, 0.1, 30.0), (0.32, -0.05, -30.0), (0.05, 0.3, 0.0)] {
        out.push(Part { tilt_deg: tilt, ..part(Shape::Fire { radius: 0.04 * r, height: 0.14 * r }, [x * r, 1.3 * r, z * r], 0.0, [1.0; 3], Some(LAVA)) });
    }
    for (x, y, z, size) in [(0.0, 1.73, 0.0, 0.3), (-0.22, 1.63, 0.1, 0.22), (0.24, 1.68, -0.05, 0.24), (0.05, 1.98, 0.0, 0.25), (-0.15, 1.93, -0.1, 0.2)] {
        out.push(part(Shape::Ball(size * r), [x * r, y * r, z * r], 0.0, [1.0; 3], Some([0.85, 0.8, 0.72])));
    }
}
