//! Creatures and their limbs.

use super::*;

/// A big bee flying at head height, facing east: a black head, a brown furry thorax, a
/// yellow body with black stripes and a sting, and two pale wings.
pub(super) fn bee() -> Vec<Part> {
    let y = 0.9;
    let ball = |r: f32, pos: [f32; 3], stretch: f32, color| part(Shape::Ball(r), pos, 0.0, [stretch, 1.0, 1.0], Some(color));
    let black = [0.08, 0.07, 0.06];
    let mut out = vec![
        ball(0.07, [0.17, y, 0.0], 1.0, black),
        ball(0.1, [0.05, y, 0.0], 1.0, [0.35, 0.22, 0.1]),
        ball(0.12, [-0.15, y - 0.02, 0.0], 1.5, [0.95, 0.75, 0.15]),
        // The sting: a cone laid on its side, pointing west from the end of the body.
        Part { tilt_deg: -90.0, ..part(Shape::Cone { radius: 0.025, height: 0.08 }, [-0.35, y - 0.02, 0.0], 0.0, [1.0; 3], Some(black)) },
    ];
    for x in [-0.1, -0.2] {
        out.push(Part { tilt_deg: 90.0, ..cylinder(0.118, 0.035, [x, y - 0.02, 0.0], black) });
    }
    // Big dark eyes on the sides of the head, each with a shine.
    for side in [-1.0, 1.0] {
        out.push(ball(0.04, [0.19, y + 0.015, side * 0.045], 1.0, [0.3, 0.14, 0.05]));
        out.push(ball(0.012, [0.215, y + 0.035, side * 0.065], 1.0, [1.0, 1.0, 1.0]));
    }
    for side in [-1.0, 1.0] {
        out.push(Part { yaw_deg: side * -20.0, ..cube([0.16, 0.006, 0.22], [-0.02, y + 0.11, side * 0.13], [0.88, 0.93, 1.0]) });
    }
    out
}

/// A spider web standing across the square, facing south: threads out from the middle
/// to the sides, and rings of thread round the middle.
pub(super) fn spider_web() -> Vec<Part> {
    // Dark threads, to stand out against pale floors and the sky.
    let silk = [0.12, 0.12, 0.14];
    let (y, reach) = (0.55, 0.48);
    let mut out = Vec::new();
    for i in 0..8 {
        let a = i as f32 * 45.0 + 10.0;
        let (sin, cos) = a.to_radians().sin_cos();
        out.push(Part { tilt_deg: -a, ..cube([reach, 0.009, 0.009], [cos * reach / 2.0, y + sin * reach / 2.0, 0.0], silk) });
    }
    for r in [0.1, 0.2, 0.3, 0.4] {
        out.extend(arc([0.0, y], r, [10.0, 370.0], 8, 0.008, silk));
    }
    out
}

/// A rabbit's foot charm: a white furry foot with toes, and a gold cap with a ring.
pub(super) fn rabbit_foot() -> Vec<Part> {
    let fur = [0.95, 0.94, 0.9];
    let mut out = vec![
        part(Shape::Ball(0.05), [0.0, 0.05, 0.0], 0.0, [2.4, 1.0, 1.0], Some(fur)),
        Part { tilt_deg: 90.0, ..cylinder(0.04, 0.05, [-0.12, 0.05, 0.0], GOLD) },
        Part { yaw_deg: 90.0, tilt_deg: 90.0, ..cylinder(0.025, 0.008, [-0.16, 0.05, 0.0], GOLD) },
    ];
    for z in [-0.025, 0.0, 0.025] {
        out.push(part(Shape::Ball(0.022), [0.115, 0.03, z], 0.0, [1.0; 3], Some(fur)));
    }
    out
}

/// A small rag doll sitting on the floor: white body and arms, a round head, legs out
/// in front.
pub(super) fn doll() -> Vec<Part> {
    let cloth = [0.95, 0.93, 0.9];
    vec![
        cube([0.1, 0.12, 0.06], [0.0, 0.08, 0.0], cloth),
        part(Shape::Ball(0.05), [0.0, 0.19, 0.0], 0.0, [1.0; 3], Some(cloth)),
        part(Shape::Ball(0.008), [-0.02, 0.2, 0.045], 0.0, [1.0; 3], Some([0.1, 0.1, 0.1])),
        part(Shape::Ball(0.008), [0.02, 0.2, 0.045], 0.0, [1.0; 3], Some([0.1, 0.1, 0.1])),
        cube([0.03, 0.03, 0.1], [-0.03, 0.02, 0.06], cloth),
        cube([0.03, 0.03, 0.1], [0.03, 0.02, 0.06], cloth),
        Part { tilt_deg: 30.0, ..cube([0.025, 0.08, 0.025], [-0.07, 0.1, 0.0], cloth) },
        Part { tilt_deg: -30.0, ..cube([0.025, 0.08, 0.025], [0.07, 0.1, 0.0], cloth) },
    ]
}

/// A wicker basket with a pink cushion, and a ginger cat lying on it facing east, its
/// tail curled round its side.
pub(super) fn cat_basket() -> Vec<Part> {
    let fur = [0.92, 0.55, 0.2];
    let ball = |r: f32, pos: [f32; 3], scale: [f32; 3], color| part(Shape::Ball(r), pos, 0.0, scale, Some(color));
    let mut out = vec![
        cylinder(0.25, 0.12, [0.0, 0.06, 0.0], [0.7, 0.55, 0.3]),
        cylinder(0.22, 0.03, [0.0, 0.125, 0.0], [0.95, 0.6, 0.7]),
        ball(0.1, [-0.02, 0.2, 0.0], [1.6, 0.75, 1.0], fur),
        ball(0.07, [0.14, 0.26, 0.0], [1.0; 3], fur),
        ball(0.028, [0.2, 0.245, 0.0], [1.0; 3], [0.98, 0.85, 0.7]),
        ball(0.008, [0.225, 0.255, 0.0], [1.0; 3], [0.9, 0.45, 0.5]),
    ];
    for side in [-1.0, 1.0] {
        out.push(Part { tilt_deg: -side * 15.0, ..point(0.025, 0.05, [0.15, 0.31, side * 0.035], fur) });
        out.push(ball(0.011, [0.2, 0.275, side * 0.026], [1.0; 3], [0.25, 0.6, 0.2]));
        out.push(ball(0.03, [0.16, 0.14, side * 0.05], [1.3, 0.7, 1.0], fur));
    }
    // The tail, round the south side from the back to the front paws.
    for k in 0..16 {
        let a = (170.0 - k as f32 * 7.5).to_radians();
        out.push(ball(0.028, [0.17 * a.cos(), 0.155, 0.17 * a.sin()], [1.0; 3], fur));
    }
    // Darker stripes: thin bands round the body, as big as the body where they are.
    for x in [-0.1, -0.03, 0.04] {
        let across = (1.0 - ((x + 0.02) / 0.16f32).powi(2)).sqrt();
        out.push(ball(0.1 * across + 0.004, [x, 0.2, 0.0], [0.12, 0.75, 1.0], [0.7, 0.35, 0.1]));
    }
    out
}

/// A snake lying in an S on the ground, facing east, `k` times the size of a small one:
/// a body of balls getting thinner to the tail, a head with eyes and a forked tongue;
/// a cobra rears up with its hood spread.
pub(super) fn snake(color: [f32; 3], k: f32, cobra: bool) -> Vec<Part> {
    let ball = |r: f32, pos: [f32; 3], scale: [f32; 3], color| part(Shape::Ball(r * k), pos.map(|c| c * k), 0.0, scale, Some(color));
    let mut out = Vec::new();
    for i in 0..12 {
        let t = i as f32 / 11.0;
        let x = -0.4 + t * 0.6;
        let y = if cobra && t > 0.8 { 0.05 + (t - 0.8) * 1.2 } else { 0.045 };
        out.push(ball(0.045 * (0.5 + 0.5 * t), [x, y, 0.08 * (t * 9.0).sin()], [1.0; 3], color));
    }
    let head = if cobra { [0.25, 0.32, 0.0] } else { [0.24, 0.05, 0.0] };
    out.push(ball(0.05, head, [1.5, 0.8, 1.1], color));
    for side in [-1.0, 1.0] {
        out.push(ball(0.012, [head[0] + 0.03, head[1] + 0.02, side * 0.03], [1.0; 3], [0.9, 0.8, 0.1]));
    }
    out.push(cube([0.08 * k, 0.005 * k, 0.015 * k], [(head[0] + 0.1) * k, head[1] * k, 0.0], [0.8, 0.1, 0.1]));
    if cobra {
        out.push(ball(0.1, [0.22, 0.27, 0.0], [0.3, 1.0, 1.3], color));
    }
    out
}

/// A big worm crawling east: rings of body rising and falling, and a dark round mouth.
pub(super) fn worm(color: [f32; 3], k: f32) -> Vec<Part> {
    let mut out: Vec<Part> = (0..9)
        .map(|i| {
            let t = i as f32 / 8.0;
            let r = 0.07 * k * (0.7 + 0.3 * (t * 3.0).sin().abs());
            part(Shape::Ball(r), [(-0.3 + t * 0.55) * k, r + 0.03 * k * (t * 6.0).sin().abs(), 0.0], 0.0, [1.0; 3], Some(color))
        })
        .collect();
    out.push(Part { yaw_deg: 90.0, tilt_deg: 90.0, ..cylinder(0.035 * k, 0.01, [0.33 * k, 0.07 * k, 0.0], [0.15, 0.05, 0.05]) });
    out
}

/// A box `thick` from point `a` to point `b`, like a leg or a feeler.
pub(super) fn limb(a: [f32; 3], b: [f32; 3], thick: f32, color: [f32; 3]) -> Part {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let flat = d[0].hypot(d[2]);
    let len = flat.hypot(d[1]);
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
    Part { yaw_deg: d[2].atan2(d[0]).to_degrees(), tilt_deg: -d[1].atan2(flat).to_degrees(), ..cube([len, thick, thick], mid, color) }
}

/// A giant ant facing east, `k` times the size of a small one: abdomen, thorax and head,
/// six legs bent at the knee down to the floor, and two feelers.
pub(super) fn ant(color: [f32; 3], k: f32) -> Vec<Part> {
    let y = 0.08 * k;
    let at = |p: [f32; 3]| p.map(|c| c * k);
    let ball = |r: f32, x: f32, scale: [f32; 3]| part(Shape::Ball(r * k), [x * k, y, 0.0], 0.0, scale, Some(color));
    let mut out = vec![ball(0.07, -0.12, [1.4, 1.0, 1.0]), ball(0.035, 0.0, [1.4, 1.0, 1.0]), ball(0.045, 0.08, [1.0; 3])];
    for side in [-1.0, 1.0] {
        for x in [-0.03, 0.0, 0.03] {
            let (hip, knee, foot) = ([x, 0.08, side * 0.02], [x * 2.5, 0.12, side * 0.09], [x * 4.0, 0.0, side * 0.15]);
            out.push(limb(at(hip), at(knee), 0.012 * k, color));
            out.push(limb(at(knee), at(foot), 0.01 * k, color));
        }
        out.push(limb(at([0.11, 0.1, side * 0.015]), at([0.18, 0.16, side * 0.06]), 0.008 * k, color));
    }
    out
}

/// A giant spider facing east: a fat abdomen, a head with red eyes, and eight legs bent
/// up at the knee and down to the floor.
pub(super) fn spider(color: [f32; 3]) -> Vec<Part> {
    let mut out = vec![
        part(Shape::Ball(0.16), [-0.15, 0.25, 0.0], 0.0, [1.2, 1.0, 1.0], Some(color)),
        part(Shape::Ball(0.09), [0.06, 0.2, 0.0], 0.0, [1.0; 3], Some(color)),
    ];
    for side in [-1.0, 1.0] {
        out.push(part(Shape::Ball(0.018), [0.14, 0.23, side * 0.03], 0.0, [1.0; 3], Some([0.9, 0.1, 0.1])));
        // Legs fan out from the head, two forward, two back.
        for a in [50.0f32, 20.0, -15.0, -45.0] {
            let (sin, cos) = a.to_radians().sin_cos();
            // A point `r` out from the head along this leg, at height `y`.
            let at = |r: f32, y: f32| [0.06 + sin * r, y, side * cos * r];
            out.push(limb(at(0.06, 0.2), at(0.22, 0.36), 0.022, color));
            out.push(limb(at(0.22, 0.36), at(0.42, 0.0), 0.018, color));
        }
    }
    out
}

/// A giant centipede crawling east: a row of segments, each with a pair of legs, and a
/// head with feelers.
pub(super) fn centipede(color: [f32; 3]) -> Vec<Part> {
    let mut out = Vec::new();
    for i in 0..10 {
        let x = -0.45 + i as f32 * 0.09;
        let z = 0.05 * (i as f32 * 0.8).sin();
        out.push(part(Shape::Ball(0.055), [x, 0.07, z], 0.0, [1.0, 0.8, 1.0], Some(color)));
        for side in [-1.0, 1.0] {
            out.push(limb([x, 0.06, z + side * 0.03], [x - 0.02, 0.0, z + side * 0.12], 0.012, color));
        }
    }
    for side in [-1.0, 1.0] {
        out.push(limb([0.45, 0.09, side * 0.02], [0.58, 0.16, side * 0.07], 0.01, color));
    }
    out
}

/// How tall a chess piece is, relative to a king.
fn chess_height(piece: &str) -> f32 {
    match piece {
        "pawn" => 0.6,
        "rook" => 0.75,
        "knight" | "bishop" => 0.85,
        _ => 1.0,
    }
}

/// A chess piece about a person high: `piece` is pawn, rook, knight, bishop, queen or king.
pub(super) fn chess_piece(piece: &str, color: [f32; 3]) -> Vec<Part> {
    let high = chess_height(piece);
    let mut out = vec![
        cylinder(0.24, 0.08, [0.0, 0.04, 0.0], color),
        part(Shape::Frustum { radius: 0.17, height: high * 0.6 }, [0.0, 0.08 + high * 0.3, 0.0], 0.0, [1.0; 3], Some(color)),
        cylinder(0.12, 0.04, [0.0, 0.08 + high * 0.6, 0.0], color),
    ];
    out.extend(chess_top(piece, 0.1 + high * 0.6, color));
    out
}

/// The crown of a chess piece (the part above the body) at height `top`; a king for any unknown name.
fn chess_top(piece: &str, top: f32, color: [f32; 3]) -> Vec<Part> {
    let mut out = Vec::new();
    match piece {
        "pawn" => out.push(part(Shape::Ball(0.1), [0.0, top + 0.08, 0.0], 0.0, [1.0; 3], Some(color))),
        "rook" => {
            out.push(cylinder(0.14, 0.14, [0.0, top + 0.07, 0.0], color));
            for i in 0..4 {
                let (sin, cos) = (i as f32 * 90.0f32).to_radians().sin_cos();
                out.push(cube([0.07, 0.07, 0.07], [cos * 0.1, top + 0.17, sin * 0.1], color));
            }
        }
        "knight" => {
            out.push(Part { tilt_deg: -20.0, ..cube([0.12, 0.26, 0.1], [0.0, top + 0.12, 0.0], color) });
            out.push(cube([0.2, 0.08, 0.09], [0.1, top + 0.22, 0.0], color));
            out.push(cube([0.03, 0.06, 0.03], [-0.02, top + 0.3, 0.0], color));
        }
        "bishop" => {
            out.push(part(Shape::Ball(0.1), [0.0, top + 0.12, 0.0], 0.0, [1.0, 1.5, 1.0], Some(color)));
            out.push(part(Shape::Ball(0.03), [0.0, top + 0.29, 0.0], 0.0, [1.0; 3], Some(color)));
        }
        "queen" => {
            out.push(part(Shape::Ball(0.11), [0.0, top + 0.1, 0.0], 0.0, [1.0; 3], Some(color)));
            for i in 0..6 {
                let (sin, cos) = (i as f32 * 60.0f32).to_radians().sin_cos();
                out.push(point(0.025, 0.08, [cos * 0.08, top + 0.17, sin * 0.08], color));
            }
        }
        _ => {
            out.push(part(Shape::Ball(0.11), [0.0, top + 0.1, 0.0], 0.0, [1.0; 3], Some(color)));
            out.push(cube([0.03, 0.14, 0.03], [0.0, top + 0.27, 0.0], color));
            out.push(cube([0.1, 0.03, 0.03], [0.0, top + 0.29, 0.0], color));
        }
    }
    out
}

/// A tentacle rising out of the ground and curling over: balls getting thinner to the tip.
pub(super) fn tentacle(color: [f32; 3]) -> Vec<Part> {
    (0..14)
        .map(|i| {
            let t = i as f32 / 13.0;
            let a = t * 2.4;
            part(Shape::Ball(0.11 * (1.0 - 0.75 * t)), [0.25 * (1.0 - a.cos()), 0.6 * a.sin() + 0.08, 0.0], 0.0, [1.0; 3], Some(color))
        })
        .collect()
}

/// A golden halo floating at head height, with two white wings.
pub(super) fn halo() -> Vec<Part> {
    let mut out = arc([0.0, 1.0], 0.12, [0.0, 360.0], 10, 0.03, GOLD);
    for side in [-1.0, 1.0] {
        out.push(part(Shape::Ball(0.14), [side * 0.25, 0.9, 0.0], 0.0, [1.2, 0.6, 0.2], Some([0.97, 0.97, 0.95])));
    }
    out
}
