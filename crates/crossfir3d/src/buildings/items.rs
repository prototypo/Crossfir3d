//! Items held or carried, and their shapes.

use super::*;

/// A breastplate lying on its back, neck to the north, with shoulder pads and a belt.
pub(super) fn armour(color: [f32; 3]) -> Vec<Part> {
    let dark = color.map(|c| c * 0.6);
    let mut out = vec![
        cube([0.34, 0.06, 0.38], [0.0, 0.03, 0.0], color),
        cube([0.36, 0.07, 0.05], [0.0, 0.035, 0.17], dark),
        cube([0.1, 0.07, 0.04], [0.0, 0.035, -0.19], dark),
    ];
    for side in [-1.0, 1.0] {
        out.push(cube([0.12, 0.08, 0.13], [side * 0.2, 0.04, -0.12], color));
    }
    out
}

/// Cloth folded in a pile, with its sleeves folded over; a cloak has a gold clasp.
pub(super) fn garment(color: [f32; 3], clasp: bool) -> Vec<Part> {
    let fold = color.map(|c| c * 0.8);
    let mut out = vec![
        cube([0.4, 0.04, 0.32], [0.0, 0.02, 0.0], color),
        cube([0.4, 0.03, 0.18], [0.0, 0.055, -0.06], fold),
    ];
    if clasp {
        out.push(cylinder(0.035, 0.02, [0.0, 0.08, -0.08], GOLD));
    } else {
        out.extend([-0.12, 0.12].map(|x| cube([0.07, 0.03, 0.26], [x, 0.085, 0.0], color)));
    }
    out
}

/// A small table with a heap of gems in `color` on it.
pub(super) fn converter(color: [f32; 3]) -> Vec<Part> {
    let mut out = vec![cube([0.6, 0.05, 0.4], [0.0, 0.42, 0.0], WOOD)];
    for (x, z) in [(-0.25, -0.15), (0.25, -0.15), (-0.25, 0.15), (0.25, 0.15)] {
        out.push(cube([0.05, 0.4, 0.05], [x, 0.2, z], WOOD));
    }
    for (i, (x, z)) in [(-0.1, 0.0), (0.05, -0.06), (0.1, 0.07), (-0.02, 0.08), (0.0, 0.0)].into_iter().enumerate() {
        let y = if i == 4 { 0.52 } else { 0.475 };
        out.push(Part { yaw_deg: i as f32 * 35.0, ..cube([0.07; 3], [x, y, z], color) });
    }
    out
}

/// The (u, v) spot offsets of a die face showing `n` (anything outside 1..=5 shows six).
fn die_spots(n: u8) -> &'static [(f32, f32)] {
    match n {
        1 => &[(0.0, 0.0)],
        2 => &[(-1.0, -1.0), (1.0, 1.0)],
        3 => &[(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)],
        4 => &[(-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)],
        5 => &[(-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (0.0, 0.0)],
        _ => &[(-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, 0.0), (1.0, 0.0)],
    }
}

/// A die (`pos` its foot, turned `yaw` degrees) with the spots `faces` on its top, south
/// and east faces.
pub(super) fn die(pos: [f32; 3], yaw: f32, faces: [u8; 3]) -> Vec<Part> {
    const S: f32 = 0.1;
    const D: f32 = 0.025;
    let dot = [0.08, 0.08, 0.08];
    let mut out = vec![cube([S; 3], [0.0, S / 2.0, 0.0], [0.95, 0.93, 0.88])];
    for &(u, v) in die_spots(faces[0]) {
        out.push(cube([0.016, 0.006, 0.016], [u * D, S, v * D], dot));
    }
    for &(u, v) in die_spots(faces[1]) {
        out.push(cube([0.016, 0.016, 0.006], [u * D, S / 2.0 + v * D, S / 2.0], dot));
    }
    for &(u, v) in die_spots(faces[2]) {
        out.push(cube([0.006, 0.016, 0.016], [S / 2.0, S / 2.0 + v * D, u * D], dot));
    }
    turned(out, yaw).into_iter().map(|p| Part { pos: [p.pos[0] + pos[0], p.pos[1] + pos[1], p.pos[2] + pos[2]], ..p }).collect()
}

/// A skull facing south: a long round head, dark eye holes and nose, cheekbones, and a
/// jaw with a row of teeth.
pub(super) fn skull() -> Vec<Part> {
    let bone = [0.9, 0.88, 0.8];
    let dark = [0.06, 0.05, 0.05];
    let ball = |r: f32, pos: [f32; 3], scale: [f32; 3], color| part(Shape::Ball(r), pos, 0.0, scale, Some(color));
    let mut out = vec![
        ball(0.16, [0.0, 0.24, -0.03], [1.0, 0.95, 1.15], bone),
        cube([0.2, 0.12, 0.13], [0.0, 0.13, 0.08], bone),
        cube([0.15, 0.05, 0.1], [0.0, 0.045, 0.1], bone),
        Part { tilt_deg: 180.0, ..part(Shape::Cone { radius: 0.025, height: 0.05 }, [0.0, 0.135, 0.146], 0.0, [1.0; 3], Some(dark)) },
        cube([0.15, 0.03, 0.02], [0.0, 0.08, 0.14], dark),
    ];
    for side in [-1.0, 1.0] {
        out.push(ball(0.048, [side * 0.06, 0.2, 0.13], [1.0, 1.1, 0.35], dark));
        out.push(ball(0.035, [side * 0.09, 0.14, 0.12], [1.0; 3], bone));
    }
    for i in 0..6 {
        out.push(cube([0.018, 0.03, 0.012], [-0.055 + i as f32 * 0.022, 0.08, 0.152], bone));
    }
    out
}

/// A kettlebell: a blue ball on a flat foot, with a black handle arching over it.
pub(super) fn kettlebell() -> Vec<Part> {
    let blue = [0.15, 0.25, 0.65];
    let mut out = vec![
        cylinder(0.09, 0.02, [0.0, 0.01, 0.0], blue),
        part(Shape::Ball(0.15), [0.0, 0.16, 0.0], 0.0, [1.0; 3], Some(blue)),
        cylinder(0.035, 0.08, [-0.1, 0.3, 0.0], blue),
        cylinder(0.035, 0.08, [0.1, 0.3, 0.0], blue),
    ];
    out.extend(arc([0.0, 0.34], 0.1, [0.0, 180.0], 7, 0.035, [0.05, 0.05, 0.05]));
    out
}

/// A wheel of fortune, painted red, on a post `high`, with a pointer.
pub(super) fn fortune_wheel(high: f32) -> Vec<Part> {
    let mut out = vec![cube([0.08, high, 0.08], [0.0, high / 2.0, -0.05], WOOD), disc(0.32, 0.03, [0.0, high + 0.25, 0.0], [0.85, 0.2, 0.15])];
    out.extend(wheel(0.3, [0.0, high + 0.25, 0.03]));
    // A pointer like the big hand of a clock, pointing to one o'clock.
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let at = |d: f32| [sin * d, high + 0.25 + cos * d, 0.075];
    out.push(Part { tilt_deg: 30.0, ..cube([0.03, 0.26, 0.012], at(0.13), GOLD) });
    out.push(Part { tilt_deg: 30.0, ..part(Shape::Cone { radius: 0.035, height: 0.07 }, at(0.29), 0.0, [1.0; 3], Some(GOLD)) });
    out.push(disc(0.04, 0.03, [0.0, high + 0.25, 0.075], GOLD));
    out
}

/// A leather quiver leaning a little, full of arrows with their feathers showing, and
/// a shoulder strap down one side.
pub(super) fn quiver() -> Vec<Part> {
    let leather = [0.45, 0.25, 0.12];
    let mut out = vec![
        cylinder(0.06, 0.5, [0.0, 0.25, 0.0], leather),
        cylinder(0.066, 0.03, [0.0, 0.49, 0.0], [0.3, 0.16, 0.08]),
        cube([0.015, 0.4, 0.03], [-0.11, 0.27, 0.0], [0.3, 0.16, 0.08]),
        cube([0.06, 0.015, 0.03], [-0.085, 0.46, 0.0], [0.3, 0.16, 0.08]),
        cube([0.06, 0.015, 0.03], [-0.085, 0.08, 0.0], [0.3, 0.16, 0.08]),
    ];
    for (k, (x, z)) in [(-0.025, -0.02), (0.02, -0.025), (0.0, 0.0), (-0.02, 0.025), (0.025, 0.02)].into_iter().enumerate() {
        let feather = if k % 2 == 0 { [0.92, 0.9, 0.85] } else { [0.8, 0.2, 0.15] };
        out.push(cylinder(0.006, 0.25, [x, 0.58, z], BOARD));
        out.push(cube([0.004, 0.07, 0.035], [x, 0.66, z], feather));
        out.push(cube([0.035, 0.07, 0.004], [x, 0.66, z], feather));
    }
    tilted(out, 15.0)
}

/// A playing card lying on the floor, with a black skull on it when `skull` is given.
pub(super) fn card(skull: Option<[f32; 3]>) -> Vec<Part> {
    let mut out = vec![Part { yaw_deg: 15.0, ..cube([0.12, 0.006, 0.17], [0.0, 0.003, 0.0], [0.97, 0.96, 0.93]) }];
    match skull {
        Some(color) => out.push(part(Shape::Ball(0.03), [0.0, 0.008, 0.0], 0.0, [1.0, 0.2, 1.1], Some(color))),
        None => out.push(Part { yaw_deg: 60.0, ..cube([0.035, 0.008, 0.035], [0.0, 0.006, 0.0], [0.8, 0.1, 0.1]) }),
    }
    out
}

/// A white note lying on the floor with lines of writing on it.
/// Small things lying on the ground, in the colour after the colon (`ring:0.8,0.1,0.1`):
/// our own shapes for the item families (the blade, the gem, the liquid takes the colour).
pub const ITEM_KINDS: [&str; 19] = [
    "sword", "dagger", "axe", "hammer", "mace", "club", "spear", "staff", "bow", "arrow", "ring", "amulet", "potion",
    "shield", "helmet", "coin", "gem", "ingot", "fruit",
];

const GRIP: [f32; 3] = [0.35, 0.2, 0.12];

/// A ring of `n` small balls resting on the ground round the middle (chains, bands).
fn hoop(radius: f32, n: usize, ball: f32, z: f32, color: [f32; 3]) -> Vec<Part> {
    (0..n)
        .map(|i| {
            let (sin, cos) = (i as f32 / n as f32 * std::f32::consts::TAU).sin_cos();
            part(Shape::Ball(ball), [radius * cos, ball, z + radius * sin], 0.0, [1.0; 3], Some(color))
        })
        .collect()
}

/// A sword `k` times full length (a dagger is a short one), lying east-west.
fn sword(c: [f32; 3], k: f32) -> Vec<Part> {
    vec![
        cube([0.42 * k, 0.012, 0.045 * k.sqrt()], [0.07 * k, 0.008, 0.0], c),
        cube([0.06 * k, 0.01, 0.025 * k.sqrt()], [0.31 * k, 0.008, 0.0], c),
        cube([0.03, 0.025, 0.15 * k.sqrt()], [-0.15 * k, 0.013, 0.0], GOLD),
        cube([0.1 * k, 0.028, 0.028], [-0.22 * k, 0.014, 0.0], GRIP),
        part(Shape::Ball(0.022), [-0.285 * k, 0.022, 0.0], 0.0, [1.0; 3], Some(GOLD)),
    ]
}

/// A wooden handle from its grip end on the ground at `grip_x` up through the middle of
/// the head at `head`, a little past it (the thing leans on its head, not flat).
pub(super) fn shaft(grip_x: f32, head: [f32; 3]) -> Part {
    let (dx, dy) = (head[0] - grip_x, head[1] - 0.013);
    let k = 1.0 + 0.03 / dx;
    limb([grip_x, 0.013, 0.0], [grip_x + dx * k, 0.013 + dy * k, 0.0], 0.026, WOOD)
}

fn ball(r: f32, pos: [f32; 3], scale: [f32; 3], color: [f32; 3]) -> Part {
    part(Shape::Ball(r), pos, 0.0, scale, Some(color))
}

/// A long staff, its gem glowing in a cage of three prongs at the top.
fn staff(c: [f32; 3]) -> Vec<Part> {
    let mut out = vec![shaft(-0.42, [0.37, 0.045, 0.0])];
    for z in [-0.03, 0.0, 0.03] {
        out.push(limb([0.3, 0.015, z * 0.6], [0.4, 0.05, z], 0.012, GOLD));
    }
    out.push(Part { yaw_deg: 45.0, tilt_deg: 45.0, ..part(Shape::Glow([0.045; 3]), [0.37, 0.045, 0.0], 0.0, [1.0; 3], Some(c)) });
    out
}

/// A smooth curved bow with its string.
fn bow(c: [f32; 3]) -> Vec<Part> {
    let (cx, r, n) = (-0.2, 0.3, 10);
    let at = |i: usize| {
        let a = (-60.0 + 120.0 * i as f32 / n as f32).to_radians();
        [cx + r * a.cos(), 0.015, r * a.sin()]
    };
    let mut out: Vec<Part> = (0..n).map(|i| limb(at(i), at(i + 1), 0.022, c)).collect();
    out.push(limb(at(0), at(n), 0.004, [0.9, 0.88, 0.8]));
    out
}

/// A gold band with a small stone set on it.
fn ring(c: [f32; 3]) -> Vec<Part> {
    let mut out = hoop(0.045, 12, 0.012, 0.0, GOLD);
    out.push(ball(0.022, [0.05, 0.03, 0.0], [1.0; 3], c));
    out
}

/// A gold chain with a round gold plate and a stone on it.
fn amulet(c: [f32; 3]) -> Vec<Part> {
    let mut out = hoop(0.1, 20, 0.007, -0.03, GOLD);
    out.push(cylinder(0.04, 0.012, [0.0, 0.006, 0.1], GOLD));
    out.push(ball(0.026, [0.0, 0.016, 0.1], [1.0, 0.6, 1.0], c));
    out
}

/// The hand weapons: sword, dagger, axe, hammer, mace, club and spear.
fn weapon(kind: &str, c: [f32; 3]) -> Option<Vec<Part>> {
    Some(match kind {
        "sword" => sword(c, 1.0),
        "dagger" => sword(c, 0.55),
        "axe" => vec![shaft(-0.26, [0.2, 0.045, 0.0]), cube([0.07, 0.03, 0.18], [0.2, 0.045, 0.05], c)],
        "hammer" => vec![shaft(-0.25, [0.22, 0.05, 0.0]), cube([0.08, 0.07, 0.16], [0.22, 0.05, 0.0], c)],
        // An iron head with four flanges, on a short iron-bound handle.
        "mace" => vec![
            shaft(-0.24, [0.15, 0.075, 0.0]),
            ball(0.06, [0.15, 0.075, 0.0], [1.0; 3], c),
            cube([0.13, 0.15, 0.016], [0.15, 0.075, 0.0], c),
            cube([0.13, 0.016, 0.15], [0.15, 0.075, 0.0], c),
        ],
        // One piece of wood, thin at the grip and thick at the end.
        "club" => vec![
            Part { tilt_deg: -90.0, ..part(Shape::Frustum { radius: 0.06, height: 0.42 }, [0.0, 0.045, 0.0], 0.0, [1.0; 3], Some(c)) },
            ball(0.06, [0.21, 0.05, 0.0], [0.5, 1.0, 1.0], c),
        ],
        "spear" => vec![cube([0.78, 0.02, 0.02], [-0.06, 0.011, 0.0], WOOD), cube([0.12, 0.012, 0.045], [0.38, 0.011, 0.0], c)],
        _ => return None,
    })
}

/// The things that reach: staff, bow and arrow.
fn ranged(kind: &str, c: [f32; 3]) -> Option<Vec<Part>> {
    Some(match kind {
        "staff" => staff(c),
        "bow" => bow(c),
        "arrow" => vec![
            cube([0.5, 0.012, 0.012], [0.0, 0.007, 0.0], WOOD),
            cube([0.05, 0.01, 0.03], [0.27, 0.007, 0.0], c),
            cube([0.08, 0.004, 0.045], [-0.21, 0.007, 0.0], [0.92, 0.92, 0.88]),
        ],
        _ => return None,
    })
}

/// The valuables: ring, amulet, gem, coin and ingot.
fn jewel(kind: &str, c: [f32; 3]) -> Option<Vec<Part>> {
    Some(match kind {
        "ring" => ring(c),
        "amulet" => amulet(c),
        "coin" => [(0.0, 0.005), (0.01, 0.015), (-0.005, 0.025)].map(|(x, y)| cylinder(0.045, 0.01, [x, y, 0.0], c)).to_vec(),
        "gem" => vec![point(0.05, 0.06, [0.0, 0.03, 0.0], c), Part { tilt_deg: 180.0, ..point(0.05, 0.03, [0.0, 0.0, 0.0], c) }],
        "ingot" => vec![cube([0.2, 0.05, 0.08], [0.0, 0.025, 0.0], c), cube([0.17, 0.01, 0.06], [0.0, 0.055, 0.0], c)],
        _ => return None,
    })
}

/// The rest: potion, shield, helmet and fruit.
fn other(kind: &str, c: [f32; 3]) -> Option<Vec<Part>> {
    Some(match kind {
        "potion" => vec![
            ball(0.07, [0.0, 0.075, 0.0], [1.0, 1.1, 1.0], c),
            cylinder(0.022, 0.06, [0.0, 0.17, 0.0], c),
            cylinder(0.018, 0.03, [0.0, 0.215, 0.0], [0.6, 0.45, 0.3]),
        ],
        "shield" => vec![cylinder(0.24, 0.03, [0.0, 0.015, 0.0], c), ball(0.05, [0.0, 0.03, 0.0], [1.0, 0.5, 1.0], STEEL)],
        "helmet" => vec![ball(0.12, [0.0, 0.0, 0.0], [1.0, 0.85, 1.05], c), cylinder(0.13, 0.015, [0.0, 0.008, 0.0], c.map(|v| v * 0.7))],
        "fruit" => vec![ball(0.06, [0.0, 0.06, 0.0], [1.0; 3], c), cube([0.008, 0.03, 0.008], [0.0, 0.13, 0.0], [0.35, 0.25, 0.12])],
        _ => return None,
    })
}

/// Builds the parts for an item of this kind (weapon, ranged, jewel or other).
pub(super) fn item(kind: &str, c: [f32; 3]) -> Vec<Part> {
    let parts = weapon(kind, c)
        .or_else(|| ranged(kind, c))
        .or_else(|| jewel(kind, c))
        .or_else(|| other(kind, c))
        .unwrap_or_default();
    turned(parts, 25.0)
}

pub(super) fn note() -> Vec<Part> {
    let mut out = vec![Part { yaw_deg: -10.0, ..cube([0.18, 0.005, 0.24], [0.0, 0.003, 0.0], [0.97, 0.96, 0.92]) }];
    for i in 0..5 {
        out.push(Part { yaw_deg: -10.0, ..cube([0.13, 0.006, 0.008], [0.0, 0.004, -0.08 + i as f32 * 0.04], [0.2, 0.2, 0.25]) });
    }
    out
}

/// A bunch of flowers lying on the floor: green stems tied with a ribbon, and red,
/// yellow and pink flowers.
pub(super) fn bouquet() -> Vec<Part> {
    let mut out = Vec::new();
    for (i, z) in [-0.04f32, -0.015, 0.01, 0.035].into_iter().enumerate() {
        out.push(Part { yaw_deg: z * 300.0, ..cube([0.25, 0.012, 0.012], [-0.05, 0.015, z], [0.25, 0.55, 0.2]) });
        let color = [[0.85, 0.15, 0.15], [1.0, 0.85, 0.2], [1.0, 0.55, 0.75], [0.85, 0.15, 0.15]][i];
        out.push(part(Shape::Ball(0.035), [0.09, 0.035, z * 2.0], 0.0, [1.0; 3], Some(color)));
    }
    out.push(cube([0.03, 0.035, 0.1], [-0.08, 0.02, 0.0], [0.9, 0.2, 0.5]));
    out
}

/// A big fist floating at chest height, knuckles to the east.
pub(super) fn fist() -> Vec<Part> {
    let skin = [0.88, 0.7, 0.55];
    let mut out = vec![part(Shape::Ball(0.11), [0.0, 0.8, 0.0], 0.0, [1.0, 0.9, 1.1], Some(skin))];
    for z in [-0.06, -0.02, 0.02, 0.06] {
        out.push(part(Shape::Ball(0.035), [0.09, 0.82, z], 0.0, [1.0; 3], Some(skin)));
    }
    out.push(part(Shape::Ball(0.03), [0.05, 0.76, 0.1], 0.0, [1.6, 1.0, 1.0], Some(skin)));
    out
}
