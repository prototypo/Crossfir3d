//! Boats, ships, wagons, and their parts.

use super::*;

const HULL: [f32; 3] = [0.42, 0.25, 0.13];

pub(super) const DECK: [f32; 3] = [0.3, 0.18, 0.1];

const SAIL: [f32; 3] = [0.92, 0.9, 0.82];

/// Parts leaned `tilt_deg` toward the east, round their foot in the middle of the floor.
pub(super) fn tilted(parts: Vec<Part>, tilt_deg: f32) -> Vec<Part> {
    let (sin, cos) = tilt_deg.to_radians().sin_cos();
    parts
        .into_iter()
        .map(|p| {
            let [x, y, z] = p.pos;
            Part { pos: [x * cos + y * sin, y * cos - x * sin, z], tilt_deg: p.tilt_deg + tilt_deg, ..p }
        })
        .collect()
}

/// Parts turned `yaw_deg` clockwise (seen from above) round the middle.
pub(super) fn turned(parts: Vec<Part>, yaw_deg: f32) -> Vec<Part> {
    let (sin, cos) = yaw_deg.to_radians().sin_cos();
    parts
        .into_iter()
        .map(|p| {
            let [x, y, z] = p.pos;
            Part { pos: [x * cos - z * sin, y, x * sin + z * cos], yaw_deg: p.yaw_deg + yaw_deg, ..p }
        })
        .collect()
}

/// A hull `len` long and `beam` wide, its bow pointing east: a box with a box turned
/// on its corner at the bow (and at the stern, when `pointed_stern`) for a point. A
/// darker deck sits low inside a thin wall (the gunwale) round the edge. It sits a
/// little down in the water.
fn hull(len: f32, beam: f32, high: f32, pointed_stern: bool) -> Vec<Part> {
    const G: f32 = 0.025;
    let tip = beam / std::f32::consts::SQRT_2;
    let back = if pointed_stern { beam / 2.0 } else { 0.0 };
    let body = len - beam / 2.0 - back;
    let mid = (back - beam / 2.0) / 2.0;
    let bow = len / 2.0 - beam / 2.0;
    let (top, floor) = (high - 0.05, high - 0.12);
    let (y, wall) = ((floor - 0.05) / 2.0, (top + floor) / 2.0);
    let diamond = |x, size: [f32; 3], y, color| Part { yaw_deg: 45.0, ..cube(size, [x, y, 0.0], color) };
    let mut ends = vec![bow];
    let mut out = vec![cube([body, floor + 0.05, beam], [mid, y, 0.0], HULL), cube([body, 0.01, beam - G], [mid, floor, 0.0], DECK)];
    for side in [-1.0, 1.0] {
        out.push(cube([body, top - floor, G], [mid, wall, side * (beam - G) / 2.0], HULL));
        out.push(Part { yaw_deg: side * -45.0, ..cube([tip, top - floor, G], [bow + beam / 4.0, wall, side * beam / 4.0], HULL) });
    }
    if pointed_stern {
        ends.push(-bow);
        for side in [-1.0, 1.0] {
            out.push(Part { yaw_deg: side * 45.0, ..cube([tip, top - floor, G], [-bow - beam / 4.0, wall, side * beam / 4.0], HULL) });
        }
    } else {
        out.push(cube([G, top - floor, beam], [-len / 2.0 + G / 2.0, wall, 0.0], HULL));
    }
    for x in ends {
        out.push(diamond(x, [tip, floor + 0.05, tip], y, HULL));
        out.push(diamond(x, [tip - G, 0.01, tip - G], floor, DECK));
    }
    out
}

/// A mast at `x` with a square sail across the ship, hung from a yard (a pole across).
fn mast(x: f32, high: f32, sail: [f32; 2], color: [f32; 3]) -> Vec<Part> {
    let top = high - 0.05;
    vec![
        cylinder(0.02, high, [x, high / 2.0, 0.0], WOOD),
        cube([0.02, sail[1], sail[0]], [x + 0.03, top - sail[1] / 2.0, 0.0], color),
        disc(0.015, sail[0] + 0.12, [x + 0.03, top, 0.0], WOOD),
    ]
}

/// A wheel standing on edge facing south: a rim, four spokes across and a hub.
pub(super) fn wheel(radius: f32, pos: [f32; 3]) -> Vec<Part> {
    let [x, y, z] = pos;
    let mut out = vec![disc(0.03, 0.06, pos, WOOD)];
    for i in 0..8 {
        let a = i as f32 * 45.0;
        let (sin, cos) = a.to_radians().sin_cos();
        out.push(Part { tilt_deg: -(a + 90.0), ..cube([radius * 0.83, 0.02, 0.03], [x + radius * cos, y + radius * sin, z], DECK) });
        if i < 4 {
            out.push(Part { tilt_deg: -a, ..cube([radius * 2.0, 0.012, 0.012], pos, WOOD) });
        }
    }
    out
}

/// Oars sticking out both sides at these points along the hull, slanting down.
fn oars(xs: &[f32], beam: f32, y: f32) -> Vec<Part> {
    let mut out = Vec::new();
    for &x in xs {
        for side in [-1.0, 1.0] {
            let p = cube([0.03, 0.03, 0.35], [x, y, side * (beam / 2.0 + 0.12)], WOOD);
            out.push(Part { tilt_deg: 0.0, yaw_deg: side * 10.0, ..p });
        }
    }
    out
}

pub(super) fn rowboat() -> Vec<Part> {
    let mut out = hull(1.1, 0.4, 0.15, true);
    out.extend([-0.2, 0.15].map(|x| cube([0.08, 0.02, 0.36], [x, 0.09, 0.0], WOOD)));
    out.extend(oars(&[-0.2, 0.15], 0.4, 0.07));
    out
}

/// A long, low ship pointed at both ends, with a curling prow, a row of round red and
/// white shields down each side, oars, and a striped sail.
pub(super) fn longship() -> Vec<Part> {
    let mut out = hull(1.8, 0.45, 0.2, true);
    out.push(Part { tilt_deg: -20.0, ..cube([0.06, 0.35, 0.06], [0.85, 0.28, 0.0], HULL) });
    for (i, x) in [-0.5, -0.25, 0.0, 0.25, 0.5].into_iter().enumerate() {
        let color = if i % 2 == 0 { [0.75, 0.15, 0.1] } else { [0.9, 0.88, 0.8] };
        for side in [-1.0, 1.0] {
            out.push(disc(0.07, 0.02, [x, 0.14, side * 0.235], color));
        }
    }
    out.extend(oars(&[-0.375, -0.125, 0.125, 0.375], 0.45, 0.08));
    out.extend(mast(0.0, 1.0, [0.7, 0.5], [0.8, 0.2, 0.15]));
    out.push(cube([0.025, 0.1, 0.71], [0.03, 0.7, 0.0], SAIL));
    out
}

pub(super) fn fishboat() -> Vec<Part> {
    let mut out = hull(1.5, 0.45, 0.18, false);
    out.extend(mast(0.1, 0.9, [0.4, 0.45], SAIL));
    // A net heaped at the stern.
    out.push(cube([0.25, 0.06, 0.3], [-0.5, 0.09, 0.0], [0.55, 0.5, 0.4]));
    out
}

/// A tall ship: a square stern with a raised cabin on it, two masts with two sails
/// each, and a pole out over the bow. `len` sets the size; the rest follows it.
pub(super) fn galleon(len: f32, beam: f32) -> Vec<Part> {
    let k = len / 1.9;
    let mut out = hull(len, beam, 0.35 * k, false);
    // The raised stern deck (the poop deck), sunk inside its own gunwale.
    let (cx, w, g) = (-len / 2.0 + 0.2 * k, 0.4 * k, 0.025);
    out.extend([
        cube([w, 0.25 * k, beam], [cx, 0.325 * k, 0.0], HULL),
        cube([w - g, 0.01, beam - g], [cx, 0.45 * k, 0.0], DECK),
        cube([g, 0.075 * k, beam], [cx - (w - g) / 2.0, 0.4875 * k, 0.0], HULL),
        cube([g, 0.075 * k, beam], [cx + (w - g) / 2.0, 0.4875 * k, 0.0], HULL),
    ]);
    for side in [-1.0, 1.0] {
        out.push(cube([w, 0.075 * k, g], [cx, 0.4875 * k, side * (beam - g) / 2.0], HULL));
    }
    for x in [-0.15 * k, 0.4 * k] {
        out.extend(mast(x, 1.5 * k, [beam * 1.2, 0.45 * k], SAIL));
        out.push(cube([0.02, 0.35 * k, beam], [x + 0.03, 0.75 * k, 0.0], SAIL));
        out.push(disc(0.012, beam + 0.1, [x + 0.03, 0.925 * k, 0.0], WOOD));
    }
    // The bowsprit: from inside the bow, out over it, slanting 15 degrees up.
    let (sin, cos) = 15f32.to_radians().sin_cos();
    let reach = 0.3 * k;
    out.push(Part { tilt_deg: -15.0, ..cube([2.0 * reach, 0.03, 0.03], [len / 2.0 - 0.1 * k + reach * cos, 0.3 * k - 0.05 + reach * sin, 0.0], WOOD) });
    out
}

/// An open box cart on four wheels with its pole pointing east.
pub(super) fn wagon() -> Vec<Part> {
    let mut out = vec![
        cube([0.6, 0.04, 0.4], [-0.05, 0.23, 0.0], DECK),
        cube([0.03, 0.18, 0.4], [-0.335, 0.3, 0.0], HULL),
        cube([0.03, 0.18, 0.4], [0.235, 0.3, 0.0], HULL),
        cube([0.54, 0.18, 0.03], [-0.05, 0.3, -0.185], HULL),
        cube([0.54, 0.18, 0.03], [-0.05, 0.3, 0.185], HULL),
        cube([0.35, 0.03, 0.03], [0.42, 0.16, 0.0], WOOD),
    ];
    for x in [-0.25, 0.15] {
        for z in [-0.22, 0.22] {
            out.extend(wheel(0.1, [x, 0.11, z]));
        }
    }
    out
}

/// An anchor standing up, facing south: a ring at the top, a stock across with round
/// ends, a curved crown with pointed flukes at its ends, and a rope wound round the
/// shank and tied round the ring.
pub(super) fn anchor() -> Vec<Part> {
    let iron = [0.12, 0.12, 0.13];
    let mut out = vec![
        cube([0.07, 0.62, 0.06], [0.0, 0.45, 0.0], iron),
        cube([0.46, 0.05, 0.05], [0.0, 0.68, 0.0], iron),
        part(Shape::Ball(0.04), [-0.24, 0.68, 0.0], 0.0, [1.0; 3], Some(iron)),
        part(Shape::Ball(0.04), [0.24, 0.68, 0.0], 0.0, [1.0; 3], Some(iron)),
        // The point at the bottom of the crown.
        Part { tilt_deg: 180.0, ..part(Shape::Cone { radius: 0.045, height: 0.08 }, [0.0, 0.1, 0.0], 0.0, [1.0; 3], Some(iron)) },
    ];
    out.extend(arc([0.0, 0.86], 0.08, [0.0, 360.0], 10, 0.035, iron));
    out.extend(arc([0.0, 0.42], 0.3, [195.0, 345.0], 9, 0.07, iron));
    // Flukes: cones at the crown's ends, pointing up and out.
    for side in [-1.0, 1.0] {
        let (sin, cos) = 20f32.to_radians().sin_cos();
        let end = [side * 0.29, 0.34];
        let pos = [end[0] + side * sin * 0.09, end[1] + cos * 0.09, 0.0];
        out.push(Part { tilt_deg: side * 20.0, ..part(Shape::Cone { radius: 0.06, height: 0.18 }, pos, 0.0, [1.0; 3], Some(iron)) });
    }
    // The rope winds up round the shank to the bottom of the ring, is tied round the
    // ring in two turns, and its end hangs down.
    const R: f32 = 0.055;
    const STEPS: usize = 8;
    const PITCH: f32 = 0.16;
    let around = std::f32::consts::TAU * R / STEPS as f32;
    let rise = PITCH / STEPS as f32;
    let (len, slant) = (around.hypot(rise) * 1.1, rise.atan2(around).to_degrees());
    for i in 0..((0.76 - 0.2) / rise) as usize {
        let a = i as f32 * 360.0 / STEPS as f32;
        let (sin, cos) = a.to_radians().sin_cos();
        let piece = cube([len, 0.02, 0.02], [R * cos, 0.2 + i as f32 * rise, R * sin], BOARD);
        out.push(Part { yaw_deg: a + 90.0, tilt_deg: -slant, ..piece });
    }
    for x in [-0.02, 0.02] {
        let turn = turned(arc([0.0, 0.0], 0.035, [0.0, 360.0], 8, 0.012, BOARD), 90.0);
        out.extend(turn.into_iter().map(|p| Part { pos: [p.pos[0] + x, p.pos[1] + 0.78, p.pos[2]], ..p }));
    }
    out.push(Part { tilt_deg: 15.0, ..cylinder(0.012, 0.1, [-0.05, 0.73, 0.03], BOARD) });
    out
}

/// A ship's wheel standing on its post, facing south, with handles round the rim.
pub(super) fn ships_wheel() -> Vec<Part> {
    let mut out = vec![cube([0.12, 0.6, 0.12], [0.0, 0.3, -0.05], WOOD)];
    out.extend(wheel(0.25, [0.0, 0.75, 0.05]));
    for i in 0..8 {
        let a = (i as f32 * 45.0).to_radians();
        out.push(Part { tilt_deg: -(i as f32 * 45.0), ..cube([0.1, 0.03, 0.03], [0.3 * a.cos(), 0.75 + 0.3 * a.sin(), 0.05], WOOD) });
    }
    out
}

/// A cannon on a wooden carriage, aimed east: an iron barrel, thicker at the breech,
/// and two spoked wheels.
pub(super) fn cannon() -> Vec<Part> {
    let lying = |p: Part| Part { tilt_deg: 90.0, ..p };
    let wheel = |z: f32| Part { yaw_deg: 90.0, tilt_deg: 90.0, ..cylinder(0.14, 0.04, [-0.05, 0.14, z], WOOD) };
    vec![
        cube([0.45, 0.12, 0.22], [-0.05, 0.16, 0.0], WOOD),
        wheel(0.14),
        wheel(-0.14),
        Part { yaw_deg: 90.0, tilt_deg: 90.0, ..cylinder(0.03, 0.34, [-0.05, 0.14, 0.0], IRON) },
        lying(cylinder(0.07, 0.3, [0.0, 0.3, 0.0], IRON)),
        lying(cylinder(0.055, 0.3, [0.28, 0.3, 0.0], IRON)),
        lying(cylinder(0.065, 0.05, [0.44, 0.3, 0.0], IRON)),
        part(Shape::Ball(0.07), [-0.16, 0.3, 0.0], 0.0, [1.0; 3], Some(IRON)),
    ]
}
