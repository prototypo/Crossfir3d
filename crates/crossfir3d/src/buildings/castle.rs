//! Castles, towers, portcullises, and moats.

use super::*;

/// The stone city walls on every edge square, a gateway in the middle of the south
/// side, and a tall square tower on each corner. One square alone is one tower.
pub(super) fn castle(w: u32, h: u32) -> Vec<Part> {
    if (w, h) == (1, 1) {
        return tower([0.0; 3], 0.5, 0.5);
    }
    let (fw, fh) = (w as f32, h as f32);
    let mut out = Vec::new();
    for j in 0..h {
        for i in 0..w {
            let mut mask = 0;
            if j == 0 || j == h - 1 {
                mask |= if i > 0 { W } else { 0 } | if i < w - 1 { E } else { 0 };
            }
            if i == 0 || i == w - 1 {
                mask |= if j > 0 { N } else { 0 } | if j < h - 1 { S } else { 0 };
            }
            let pos = [-fw / 2.0 + i as f32 + 0.5, 0.0, -fh / 2.0 + j as f32 + 0.5];
            // Two wide: the south corners get only half a wall from the north, and the
            // gate goes between them.
            if w == 2 && j == h - 1 {
                if mask & N != 0 {
                    let half = [pos[0], 0.0, pos[2] - 0.25];
                    out.push(part(Shape::Model("kaykit/wall.gltf"), half, 90.0, [0.125, 0.3, 0.25], None));
                }
            } else if mask != 0 {
                let style = if j == h - 1 && i == w / 2 && mask == E | W { "gate" } else { "stone" };
                out.extend(wall_parts(style, mask).into_iter().map(|p| Part { pos, ..p }));
            }
            if (i == 0 || i == w - 1) && (j == 0 || j == h - 1) {
                out.extend(tower(pos, 0.3, 0.45));
            }
        }
    }
    if w == 2 {
        out.extend(wall_parts("gate", E | W).into_iter().map(|p| Part { pos: [0.0, 0.0, fh / 2.0 - 0.5], ..p }));
    }
    let gates: Vec<[f32; 3]> =
        out.iter().filter(|p| matches!(p.shape, Shape::Model(f) if f.contains("doorway"))).map(|p| p.pos).collect();
    for [x, _, z] in gates {
        out.extend(portcullis().into_iter().map(|p| Part { pos: [p.pos[0] + x, p.pos[1], p.pos[2] + z], ..p }));
    }
    out
}

/// An iron grid filling a doorway that runs east-west, in the middle of the wall.
fn portcullis() -> Vec<Part> {
    let mut out: Vec<Part> = (0..5).map(|i| cube([0.025, 0.8, 0.025], [-0.2 + i as f32 * 0.1, 0.4, 0.0], IRON)).collect();
    out.extend([0.15, 0.4, 0.65].map(|y| cube([0.5, 0.025, 0.025], [0.0, y, 0.0], IRON)));
    out
}

/// A square stone tower: the KayKit pillar `wide` times across and `high` times up,
/// with slit windows that climb round it, one face higher each time, like a stair
/// winding up inside.
pub(super) fn tower(pos: [f32; 3], wide: f32, high: f32) -> Vec<Part> {
    // The pillar's shaft faces are 0.55 from its middle, and it is 4 high.
    let (face, top) = (0.55 * wide + 0.01, 4.0 * high);
    let mut out = vec![part(Shape::Model("kaykit/pillar.gltf"), pos, 0.0, [wide, high, wide], None)];
    for i in 0..4 {
        let at = [face / 2.0, top * (0.2 + i as f32 * 0.17), 0.0];
        let slit = cube([face, 0.12 * wide / 0.3, 0.05 * wide / 0.3], at, INSIDE);
        out.extend(turned(vec![slit], 90.0 * i as f32).into_iter().map(|p| Part { pos: [p.pos[0] + pos[0], p.pos[1], p.pos[2] + pos[2]], ..p }));
    }
    out
}

/// Water filling the square, with a low brick bank on each side that has no moat next
/// to it, and a brick post on each inside corner so bends close up. No two faces lie
/// in one place (they would flicker): banks sink a little, reach a hair past the
/// water's edge, and the east and west ones stop at the north and south ones.
pub(super) fn moat(mask: u8) -> Vec<Part> {
    const THICK: f32 = 0.15;
    const HIGH: f32 = 0.2;
    const SINK: f32 = 0.02;
    const OUT: f32 = 0.005;
    let open = |side| mask & side != 0;
    let (edge, y) = (0.5 - THICK / 2.0 + OUT, HIGH / 2.0 - SINK);
    let mut out = vec![part(Shape::Water([1.0, 0.04, 1.0]), [0.0, 0.02, 0.0], 0.0, [1.0; 3], None)];
    for (side, dz) in [(N, -1.0), (S, 1.0)] {
        if !open(side) {
            out.push(part(Shape::Bricks([1.0, HIGH, THICK]), [0.0, y, dz * edge], 0.0, [1.0; 3], None));
        }
    }
    let [n, s] = [N, S].map(|side| if open(side) { 0.0 } else { THICK - OUT });
    for (side, dx) in [(E, 1.0), (W, -1.0)] {
        if !open(side) {
            out.push(part(Shape::Bricks([THICK, HIGH, 1.0 - n - s]), [dx * edge, y, (n - s) / 2.0], 0.0, [1.0; 3], None));
        }
    }
    let post = 0.5 - THICK / 2.0;
    for (a, b, x, z) in [(N, E, 1.0, -1.0), (E, S, 1.0, 1.0), (S, W, -1.0, 1.0), (W, N, -1.0, -1.0)] {
        if open(a) && open(b) {
            out.push(part(Shape::Bricks([THICK, HIGH, THICK]), [x * post, y, z * post], 0.0, [1.0; 3], None));
        }
    }
    out
}

/// KayKit stone brick walls, 4 units long and high: scaled to one square, a bit taller.
/// Unturned, `wall` runs east-west, `wall_corner` joins west and south, and
/// `wall_Tsplit` joins west, east and south.
pub(super) const KAYKIT_WALL: [f32; 3] = [0.25, 0.3, 0.25];
