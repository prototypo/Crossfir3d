//! Furniture, machines, and other small props.

use super::*;

/// Tongues of fire (`taller` than usual) over a `w` by `h` area at height `y`, and a
/// glow that lights round them.
pub(super) fn flames(w: u32, h: u32, y: f32, taller: f32) -> Vec<Part> {
    let mut out = vec![part(Shape::Lamp([w as f32 * 0.6, 0.1, h as f32 * 0.6]), [0.0, y - 0.1, 0.0], 0.0, [1.0; 3], Some(FLAME))];
    for j in 0..h {
        for i in 0..w {
            let (x, z) = (i as f32 + 0.5 - w as f32 / 2.0, j as f32 + 0.5 - h as f32 / 2.0);
            let tongues = [(-0.25, -0.1, 1.0), (0.2, 0.2, 0.7), (0.1, -0.25, 1.3), (-0.15, 0.25, 0.6), (0.0, 0.0, 1.6), (0.3, -0.05, 0.5)];
            for (k, (dx, dz, high)) in tongues.into_iter().enumerate() {
                let (radius, color) = if k % 2 == 0 { (0.22, [1.0, 0.35, 0.05]) } else { (0.15, [1.0, 0.75, 0.15]) };
                let high = high + taller;
                let pos = [x + dx, y + high / 2.0 - 0.15, z + dz];
                out.push(part(Shape::Fire { radius, height: high }, pos, 0.0, [1.0; 3], Some(color)));
            }
        }
    }
    out
}

/// A grand piano in `color` (a harpsichord is longer and thinner) on three legs, with
/// a keyboard sticking out on the south side and its lid propped up on a stick.
pub(super) fn piano(color: [f32; 3], harpsichord: bool) -> Vec<Part> {
    let (len, wide) = if harpsichord { (1.0, 0.45) } else { (0.85, 0.6) };
    let (y, high) = (0.6, 0.25);
    let top = y + high / 2.0;
    let front = wide / 2.0 - 0.05;
    let mut out = vec![
        cube([len, high, wide], [0.0, y, -0.05], color),
        // The narrow tail end, turned like the curve of a grand piano; a little lower,
        // so its top does not lie in the same plane as the body's.
        Part { yaw_deg: 30.0, ..cube([0.35, high - 0.01, wide * 0.6], [len / 2.0 - 0.05, y - 0.005, -0.05 - wide * 0.15], color) },
        cube([len * 0.7, 0.03, 0.14], [-0.03, y + 0.02, front], [0.95, 0.93, 0.88]),
    ];
    for k in 0..12 {
        if k % 7 != 2 && k % 7 != 6 {
            out.push(cube([0.025, 0.02, 0.06], [-0.03 - len * 0.33 + k as f32 * len * 0.06, y + 0.045, front + 0.02], [0.08, 0.08, 0.08]));
        }
    }
    // The lid, hinged along the south edge of the top and lifted 35 degrees at the north.
    let (lid, lean) = (wide * 0.9, 35f32.to_radians());
    let (south, z) = (front - 0.1, front - 0.1 - lid / 2.0 * lean.cos());
    out.push(Part { yaw_deg: 90.0, tilt_deg: -35.0, ..cube([lid, 0.02, len * 0.9], [0.0, top + lid / 2.0 * lean.sin(), z], color) });
    out.push(cylinder(0.01, lid * lean.sin(), [0.1, top + lid * lean.sin() / 2.0, south - lid * lean.cos() * 0.9], WOOD));
    for (x, z) in [(-len / 2.0 + 0.08, front - 0.05), (-len / 2.0 + 0.08, -wide / 2.0), (len / 2.0 - 0.05, -0.1)] {
        out.push(cube([0.07, y - high / 2.0, 0.07], [x, (y - high / 2.0) / 2.0, z], color));
    }
    out
}

/// A slot machine: a cabinet in `color` with a dark window of reels, a tray and a pull
/// handle on the east side.
pub(super) fn slot_machine(color: [f32; 3]) -> Vec<Part> {
    vec![
        cube([0.5, 0.9, 0.4], [0.0, 0.45, 0.0], color),
        cube([0.4, 0.2, 0.02], [0.0, 0.65, 0.2], [0.1, 0.1, 0.12]),
        cube([0.1, 0.16, 0.025], [-0.12, 0.65, 0.205], [0.95, 0.95, 0.9]),
        cube([0.1, 0.16, 0.025], [0.0, 0.65, 0.205], [0.95, 0.95, 0.9]),
        cube([0.1, 0.16, 0.025], [0.12, 0.65, 0.205], [0.95, 0.95, 0.9]),
        cube([0.3, 0.06, 0.1], [0.0, 0.25, 0.23], GOLD),
        cylinder(0.02, 0.3, [0.28, 0.75, 0.0], STEEL),
        part(Shape::Ball(0.05), [0.28, 0.92, 0.0], 0.0, [1.0; 3], Some([0.85, 0.1, 0.1])),
    ]
}

/// A curve of `n` boxes `thick` round `[x, y]` facing south, from `from` to `to`
/// degrees (0 east, 90 up).
pub(super) fn arc([cx, cy]: [f32; 2], radius: f32, [from, to]: [f32; 2], n: usize, thick: f32, color: [f32; 3]) -> Vec<Part> {
    let step = (to - from) / n as f32;
    let len = 2.0 * radius * (step.to_radians() / 2.0).sin() * 1.15;
    (0..n)
        .map(|i| {
            let a = from + step * (i as f32 + 0.5);
            let (sin, cos) = a.to_radians().sin_cos();
            Part { tilt_deg: -(a + 90.0), ..cube([len, thick, thick], [cx + radius * cos, cy + radius * sin, 0.0], color) }
        })
        .collect()
}

/// A knife, a fork and a spoon lying side by side, pointing north.
pub(super) fn utensils() -> Vec<Part> {
    let steel = [0.78, 0.8, 0.83];
    let mut out = vec![
        cube([0.03, 0.01, 0.12], [-0.1, 0.006, 0.07], WOOD),
        cube([0.035, 0.006, 0.14], [-0.1, 0.004, -0.06], steel),
        cube([0.025, 0.008, 0.16], [0.0, 0.005, 0.06], steel),
        cube([0.04, 0.008, 0.03], [0.0, 0.005, -0.03], steel),
        cube([0.025, 0.008, 0.16], [0.1, 0.005, 0.06], steel),
        part(Shape::Ball(0.035), [0.1, 0.008, -0.05], 0.0, [1.0, 0.25, 1.3], Some(steel)),
    ];
    for x in [-0.013, 0.0, 0.013] {
        out.push(cube([0.007, 0.006, 0.07], [x, 0.004, -0.08], steel));
    }
    out
}

/// A soft cloud of balls of mist floating over the square.
pub(super) fn cloud(color: [f32; 3]) -> Vec<Part> {
    [(0.0, 0.55, 0.0, 0.32), (-0.22, 0.45, 0.1, 0.24), (0.22, 0.5, -0.08, 0.26), (0.05, 0.75, -0.05, 0.22), (-0.1, 0.4, -0.18, 0.2)]
        .into_iter()
        .map(|(x, y, z, r)| part(Shape::Mist(r), [x, y, z], 0.0, [1.0; 3], Some(color)))
        .collect()
}

/// A spinning wheel facing south: a bench on three legs, two posts on it holding the
/// big wheel at its west end, and a small post holding the spindle at its east end.
pub(super) fn spinning_wheel() -> Vec<Part> {
    let (bench, high) = (0.42, 0.07);
    let top = bench + high / 2.0;
    let mut out = vec![cube([0.7, high, 0.16], [0.0, bench, 0.0], WOOD)];
    for (x, z) in [(-0.3, -0.06), (-0.3, 0.06), (0.3, 0.0)] {
        out.push(cube([0.04, bench, 0.04], [x, bench / 2.0, z], WOOD));
    }
    let axle = top + 0.32;
    for z in [-0.07, 0.07] {
        out.push(cube([0.04, axle - top + 0.03, 0.03], [-0.18, (axle + top) / 2.0, z], WOOD));
    }
    out.extend(wheel(0.28, [-0.18, axle, 0.0]));
    out.push(Part { tilt_deg: 90.0, ..cylinder(0.012, 0.16, [-0.18, axle, 0.0], WOOD) });
    out.push(cube([0.04, 0.16, 0.04], [0.28, top + 0.08, 0.0], WOOD));
    out.push(Part { tilt_deg: 90.0, ..cylinder(0.025, 0.2, [0.28, top + 0.17, 0.0], WOOD) });
    out.push(part(Shape::Ball(0.04), [0.39, top + 0.17, 0.0], 0.0, [1.0; 3], Some([0.9, 0.88, 0.8])));
    out
}

/// A wheel `width` wide on an axle running east-west: a wide rim, spokes and a hub.
fn wide_wheel(radius: f32, width: f32, pos: [f32; 3], color: [f32; 3]) -> Vec<Part> {
    const N: usize = 16;
    let len = 2.0 * radius * (std::f32::consts::PI / N as f32).sin() * 1.1;
    let mut wheel: Vec<Part> = (0..N)
        .map(|i| {
            let a = i as f32 * 360.0 / N as f32;
            let (sin, cos) = a.to_radians().sin_cos();
            Part { tilt_deg: -(a + 90.0), ..cube([len, 0.06, width], [radius * cos, radius * sin, 0.0], color) }
        })
        .collect();
    for i in 0..4 {
        wheel.push(Part { tilt_deg: -(i as f32 * 45.0), ..cube([radius * 2.0, 0.05, 0.05], [0.0; 3], color) });
    }
    wheel.push(disc(radius * 0.18, width * 1.2, [0.0; 3], [0.15, 0.15, 0.15]));
    turned(wheel, 90.0).into_iter().map(|p| Part { pos: [p.pos[0] + pos[0], p.pos[1] + pos[1], p.pos[2] + pos[2]], ..p }).collect()
}

/// A pipe of `radius` through `points`, each step straight along one axis, with a round
/// joint at every point.
pub(super) fn pipe(points: &[[f32; 3]], radius: f32, color: [f32; 3]) -> Vec<Part> {
    let mut out: Vec<Part> = points.iter().map(|&p| part(Shape::Ball(radius * 1.3), p, 0.0, [1.0; 3], Some(color))).collect();
    for w in points.windows(2) {
        let ([ax, ay, az], [bx, by, bz]) = (w[0], w[1]);
        let mid = [(ax + bx) / 2.0, (ay + by) / 2.0, (az + bz) / 2.0];
        let (dx, dy, dz) = ((bx - ax).abs(), (by - ay).abs(), (bz - az).abs());
        out.push(if dy >= dx && dy >= dz {
            cylinder(radius, dy, mid, color)
        } else if dx >= dz {
            Part { tilt_deg: 90.0, ..cylinder(radius, dx, mid, color) }
        } else {
            disc(radius, dz, mid, color)
        });
    }
    out
}

/// A steam engine standing in its iron frame on a 3 by 4 footprint, facing south: a
/// brass boiler lying on the frame with a steam dome and a gauge, a red gear on each
/// side on one axle, two brass chimneys at the back, a glowing green tube at the front,
/// and copper pipes joining them.
pub(super) fn machine() -> Vec<Part> {
    let iron = [0.22, 0.22, 0.24];
    let red = [0.6, 0.1, 0.08];
    let brass = [0.8, 0.62, 0.25];
    let copper = [0.72, 0.4, 0.2];
    let mut out = vec![
        cube([1.6, 0.1, 3.0], [0.0, 0.3, 0.0], iron),
        cube([0.7, 0.12, 0.12], [0.0, 0.41, -0.6], iron),
        cube([0.7, 0.12, 0.12], [0.0, 0.41, 0.6], iron),
        disc(0.45, 1.9, [0.0, 0.82, 0.1], brass),
        disc(0.4, 0.04, [0.0, 0.82, 1.06], [0.65, 0.48, 0.18]),
        disc(0.4, 0.04, [0.0, 0.82, -0.86], [0.65, 0.48, 0.18]),
        part(Shape::Ball(0.15), [0.0, 1.25, -0.2], 0.0, [1.0; 3], Some(brass)),
        cylinder(0.025, 0.12, [0.0, 1.43, -0.2], brass),
        // The pressure gauge on the front of the boiler.
        disc(0.08, 0.03, [0.0, 1.02, 1.08], [0.95, 0.93, 0.88]),
        Part { tilt_deg: 40.0, ..cube([0.01, 0.06, 0.01], [0.02, 1.04, 1.1], [0.1, 0.1, 0.1]) },
        // The axle through the frame, with a gear on each end.
        Part { tilt_deg: 90.0, ..cylinder(0.04, 2.0, [0.0, 0.85, 0.3], iron) },
        // The glowing tube in its brass caps at the front.
        part(Shape::RoundLamp { radius: 0.08, height: 0.6 }, [0.5, 0.75, 1.25], 0.0, [1.0; 3], Some([0.3, 1.0, 0.5])),
        cylinder(0.11, 0.06, [0.5, 0.42, 1.25], brass),
        cylinder(0.11, 0.06, [0.5, 1.08, 1.25], brass),
    ];
    for x in [-0.7, 0.7] {
        for z in [-1.4, 1.4] {
            out.push(cube([0.1, 0.25, 0.1], [x, 0.125, z], iron));
            out.push(cube([0.08, 1.6, 0.08], [x, 1.15, z], iron));
        }
        out.push(cube([0.06, 0.06, 2.9], [x, 1.95, 0.0], iron));
        out.extend(wide_wheel(0.42, 0.15, [x * 1.22, 0.85, 0.3], red));
    }
    for z in [-1.4, 1.4] {
        out.push(cube([1.5, 0.06, 0.06], [0.0, 1.95, z], iron));
    }
    for x in [-0.25, 0.25] {
        out.push(cylinder(0.1, 1.0, [x, 1.65, -0.9], brass));
        out.push(cylinder(0.14, 0.08, [x, 2.19, -0.9], brass));
    }
    // Copper pipes: dome to both chimneys, boiler to the top of the tube, and a pipe
    // round the west side from the boiler to the frame top.
    out.extend(pipe(&[[0.0, 1.38, -0.2], [0.0, 1.38, -0.6], [-0.25, 1.38, -0.6], [-0.25, 1.38, -0.8]], 0.035, copper));
    out.extend(pipe(&[[0.0, 1.38, -0.6], [0.25, 1.38, -0.6], [0.25, 1.38, -0.8]], 0.035, copper));
    out.extend(pipe(&[[0.0, 1.25, 0.7], [0.0, 1.5, 0.7], [0.5, 1.5, 0.7], [0.5, 1.5, 1.25], [0.5, 1.11, 1.25]], 0.035, copper));
    out.extend(pipe(&[[-0.45, 0.7, -0.3], [-0.6, 0.7, -0.3], [-0.6, 1.7, -0.3], [-0.6, 1.7, 0.9], [-0.45, 1.7, 0.9]], 0.03, copper));
    out
}

/// The converter table with an iron tray of glowing lava on it.
pub(super) fn alchemy_bath() -> Vec<Part> {
    let mut out = converter(IRON);
    out.truncate(5);
    out.push(cube([0.46, 0.05, 0.3], [0.0, 0.47, 0.0], IRON));
    out.push(part(Shape::Lamp([0.4, 0.02, 0.24]), [0.0, 0.49, 0.0], 0.0, [1.0; 3], Some([1.0, 0.4, 0.08])));
    out
}

/// A cone standing on `pos`.
pub(super) fn point(radius: f32, height: f32, pos: [f32; 3], color: [f32; 3]) -> Part {
    part(Shape::Cone { radius, height }, [pos[0], pos[1] + height / 2.0, pos[2]], 0.0, [1.0; 3], Some(color))
}

pub(super) const STEEL: [f32; 3] = [0.7, 0.72, 0.75];

/// One big spike, `up` of the way out of a dark hole in the ground (0 is down).
pub(super) fn spike(up: f32) -> Vec<Part> {
    let mut out = vec![cylinder(0.25, 0.01, [0.0, 0.005, 0.0], INSIDE)];
    if up > 0.0 {
        out.push(point(0.2, 0.9, [0.0, 0.9 * (up - 1.0), 0.0], STEEL));
    }
    out
}

/// A bed of small spikes, `up` of the way out of their slots in the ground.
pub(super) fn spikes(up: f32) -> Vec<Part> {
    let mut out = Vec::new();
    for j in 0..4 {
        for i in 0..4 {
            let (x, z) = (-0.33 + i as f32 * 0.22, -0.33 + j as f32 * 0.22);
            out.push(cube([0.12, 0.01, 0.04], [x, 0.005, z], INSIDE));
            if up > 0.0 {
                out.push(point(0.04, 0.35, [x, 0.35 * (up - 1.0), z], STEEL));
            }
        }
    }
    out
}

/// A ring of stones round crossed logs; lit, a fire burns on them and lights round it.
pub(super) fn campfire(lit: bool) -> Vec<Part> {
    let mut out: Vec<Part> = (0..8)
        .map(|i| {
            let a = (i as f32 * 45.0).to_radians();
            cube([0.09, 0.06, 0.09], [a.sin() * 0.24, 0.03, a.cos() * 0.24], STONE)
        })
        .collect();
    for yaw in [0.0, 60.0, 120.0] {
        out.push(part(Shape::Box([0.05, 0.05, 0.36]), [0.0, 0.05, 0.0], yaw, [1.0; 3], Some(WOOD)));
    }
    if lit {
        out.push(part(Shape::RoundLamp { radius: 0.1, height: 0.06 }, [0.0, 0.08, 0.0], 0.0, [1.0; 3], Some(FLAME)));
        for (x, z, high, color) in [(0.0, 0.0, 0.45, [1.0, 0.45, 0.08]), (-0.07, 0.05, 0.3, [1.0, 0.75, 0.15]), (0.08, -0.04, 0.32, [1.0, 0.35, 0.05]), (0.03, 0.08, 0.22, [1.0, 0.75, 0.15])] {
            out.push(part(Shape::Fire { radius: 0.08, height: high }, [x, 0.08 + high / 2.0, z], 0.0, [1.0; 3], Some(color)));
        }
    }
    out
}

/// Cut logs in a pile: two side by side and one on top.
pub(super) fn logs() -> Vec<Part> {
    let log = |x: f32, y: f32| Part { tilt_deg: 90.0, ..cylinder(0.07, 0.6, [x, y, 0.0], WOOD) };
    vec![log(-0.075, 0.07), log(0.075, 0.07), log(0.0, 0.19)]
}

// Monster generators: where monsters come from.

/// A headstone at the north end of a low mound of earth.
pub(super) fn grave() -> Vec<Part> {
    vec![
        cube([0.36, 0.08, 0.6], [0.0, 0.04, 0.08], DIRT),
        cube([0.3, 0.36, 0.08], [0.0, 0.18, -0.28], STONE),
        disc(0.15, 0.08, [0.0, 0.36, -0.28], STONE),
    ]
}

/// A dark wooden coffin with a pale cross on its lid.
pub(super) fn coffin() -> Vec<Part> {
    const DARK: [f32; 3] = [0.25, 0.15, 0.1];
    const CROSS: [f32; 3] = [0.8, 0.75, 0.6];
    vec![
        cube([0.32, 0.14, 0.7], [0.0, 0.07, 0.0], DARK),
        cube([0.34, 0.03, 0.72], [0.0, 0.155, 0.0], WOOD),
        cube([0.04, 0.01, 0.3], [0.0, 0.175, -0.05], CROSS),
        cube([0.16, 0.01, 0.04], [0.0, 0.175, -0.12], CROSS),
    ]
}

pub(super) const FLAME: [f32; 3] = [1.0, 0.6, 0.2];

/// A rope hanging down from above, with a knot at the bottom and three dark lines winding
/// round it, so it looks twisted from three strands.
pub(super) fn rope_up() -> Vec<Part> {
    const R: f32 = 0.023;
    const PITCH: f32 = 0.25;
    const STEPS: usize = 8;
    let (bottom, top) = (0.2, 2.4);
    let mut out = vec![cylinder(0.025, top - bottom, [0.0, (top + bottom) / 2.0, 0.0], BOARD), cylinder(0.05, 0.08, [0.0, 0.22, 0.0], BOARD)];
    let around = std::f32::consts::TAU * R / STEPS as f32;
    let rise = PITCH / STEPS as f32;
    let (len, slant) = (around.hypot(rise), rise.atan2(around).to_degrees());
    for strand in [0.0, 120.0, 240.0] {
        for i in 0..((top - bottom - 0.1) / rise) as usize {
            let a = strand + i as f32 * 360.0 / STEPS as f32;
            let (sin, cos) = a.to_radians().sin_cos();
            let line = cube([len, 0.006, 0.006], [R * cos, bottom + 0.1 + i as f32 * rise, R * sin], [0.08, 0.06, 0.05]);
            out.push(Part { yaw_deg: a + 90.0, tilt_deg: -slant, ..line });
        }
    }
    out
}
