//! Wall styles, wall runs, and the brick and wave textures.

use super::*;

/// One square of a long table or bench: a top `width` wide at `height`, reaching out to
/// the joined sides in `mask`. Ends, corners and lone squares stand on four legs.
pub(super) fn run(mask: u8, width: f32, height: f32, color: [f32; 3]) -> Vec<Part> {
    const THICK: f32 = 0.06;
    let y = height - THICK / 2.0;
    let half = width / 2.0;
    let arm = 0.5 - half;
    let mut out = vec![cube([width, THICK, width], [0.0, y, 0.0], color)];
    for (side, dx, dz) in [(N, 0.0, -1.0), (E, 1.0, 0.0), (S, 0.0, 1.0), (W, -1.0, 0.0)] {
        if mask & side != 0 {
            let size = if dx != 0.0 { [arm, THICK, width] } else { [width, THICK, arm] };
            out.push(cube(size, [dx * (half + arm / 2.0), y, dz * (half + arm / 2.0)], color));
        }
    }
    if mask != N | S && mask != E | W {
        let c = half - 0.04;
        for (x, z) in [(-c, -c), (c, -c), (c, c), (-c, c)] {
            out.push(cube([0.05, height - THICK, 0.05], [x, (height - THICK) / 2.0, z], WOOD));
        }
    }
    out
}

/// How much of a face one brick picture covers (width, height): 2 rows, 2 bricks long.
pub const BRICK_TILE: [f32; 2] = [0.2, 0.1];

/// How much of the water one wave picture covers.
pub const WAVE_TILE: f32 = 0.5;

/// The brick picture as RGBA: two rows of bricks, the second set over by half a brick,
/// with pale mortar between and each brick a slightly different shade. Tiles both ways.
pub fn brick_pixels() -> (u32, u32, Vec<u8>) {
    let (w, h) = (32, 16);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let row = y / 8;
        for x in 0..w {
            let along = x + row * 8;
            let mortar = y % 8 == 7 || along % 16 == 15;
            let shade = ((along / 16 + row * 3) % 4) as u8 * 9;
            let rgb = if mortar { [185, 178, 165] } else { [128 + shade, 82 + shade / 2, 62] };
            px.extend(rgb.into_iter().chain([255]));
        }
    }
    (w, h, px)
}

/// The wave picture as RGBA: faint wavy bands of blue with thin, soft crests.
/// Every wave fits a whole number of times across, so it tiles both ways.
pub fn wave_pixels() -> (u32, u32, Vec<u8>) {
    let n = 64;
    let tau = std::f32::consts::TAU;
    let mut px = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x as f32 / n as f32, y as f32 / n as f32);
            let c = 0.5 + 0.5 * (tau * (3.0 * v + 0.12 * (tau * 2.0 * u).sin())).sin();
            let rgb = if c > 0.985 {
                [46.0, 86.0, 130.0]
            } else {
                [32.0 + 7.0 * c, 68.0 + 10.0 * c, 120.0 + 12.0 * c]
            };
            px.extend(rgb.map(|v| v as u8).into_iter().chain([255]));
        }
    }
    (n, n, px)
}

/// The parts of one wall square that joins its neighbours on the sides in `mask`.
/// Unknown style: nothing (keep the plain block).
pub fn wall_parts(style: &str, mask: u8) -> Vec<Part> {
    let (straight, post) = match style {
        "stone" => return vec![stone_wall(mask)],
        // A way through a stone wall, turned to run along it.
        "gate" => {
            let yaw = if mask & (N | S) != 0 && mask & (E | W) == 0 { 90.0 } else { 0.0 };
            return vec![part(Shape::Model("kaykit/wall_doorway.gltf"), [0.0; 3], yaw, KAYKIT_WALL, None)];
        }
        "hedge" => return vec![model("nature/Bush_Common.gltf", [0.0; 3], 0.0, 0.55)],
        "rock" => return vec![model("nature/Rock_Medium_1.gltf", [0.0; 3], (mask as f32) * 40.0, 0.35)],
        "brick" => ("buildings/Wall_UnevenBrick_Straight.gltf", "buildings/Corner_Exterior_Brick.gltf"),
        "plaster" => ("buildings/Wall_Plaster_Straight.gltf", "buildings/Corner_Exterior_Wood.gltf"),
        "wood" => ("buildings/Wall_Plaster_WoodGrid.gltf", "buildings/Corner_Exterior_Wood.gltf"),
        _ => return Vec::new(),
    };
    // A straight run (or a lone square) is one full piece; anything else is a half
    // piece toward each neighbour round a post.
    match mask & 0xF {
        0 => return vec![model(straight, [0.0; 3], 0.0, VILLAGE)],
        m if m == E | W => return vec![model(straight, [0.0; 3], 0.0, VILLAGE)],
        m if m == N | S => return vec![model(straight, [0.0; 3], 90.0, VILLAGE)],
        _ => {}
    }
    let mut out = vec![model(post, [0.0; 3], 0.0, VILLAGE)];
    for (side, (dx, dz), yaw) in [(N, (0.0, -0.25), 90.0), (E, (0.25, 0.0), 0.0), (S, (0.0, 0.25), 90.0), (W, (-0.25, 0.0), 0.0)] {
        if mask & side != 0 {
            out.push(part(Shape::Model(straight), [dx, 0.0, dz], yaw, [VILLAGE / 2.0, VILLAGE, VILLAGE], None));
        }
    }
    out
}

fn stone_wall(mask: u8) -> Part {
    let (piece, yaw) = piece_for(mask);
    // piece_for's unturned pieces: Straight and End run north-south, Corner joins
    // north and east, Tee joins north, east and south.
    let (file, turn) = match piece {
        Piece::Post | Piece::Straight | Piece::End => ("kaykit/wall.gltf", 90.0),
        Piece::Corner => ("kaykit/wall_corner.gltf", 180.0),
        Piece::Tee => ("kaykit/wall_Tsplit.gltf", 270.0),
        Piece::Cross => ("kaykit/wall_crossing.gltf", 0.0),
    };
    part(Shape::Model(file), [0.0; 3], yaw + turn, KAYKIT_WALL, None)
}
