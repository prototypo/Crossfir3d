//! Buildings, walls, and small built things, put together from model pieces and plain
//! shapes. The free village pack has pieces, not houses; no pack has a well or a sign.
//!
//! Positions are relative to the middle of the ground the thing stands on. 1 unit =
//! 1 square; x = east, z = south. Yaw is in degrees, clockwise seen from above.

pub use crate::walls::{E, N, S, W};
use crate::walls::{piece_for, Piece};

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A file under `assets/models/`.
    Model(&'static str),
    /// A box of this size, centred on `pos`.
    Box([f32; 3]),
    /// An upright cylinder, centred on `pos`.
    Cylinder { radius: f32, height: f32 },
    /// A wedge (triangle running north-south) this wide, tall and long, its base on `pos`.
    Wedge([f32; 3]),
    /// The building's sign text, fitted inside this width and height, its bottom middle on `pos`.
    Sign([f32; 2]),
    /// A box of this size, centred on `pos`, faced with bricks (`BRICK_TILE` per picture).
    Bricks([f32; 3]),
    /// A box of this size, centred on `pos`, its top rippled with waves (`WAVE_TILE` per picture).
    Water([f32; 3]),
    /// A glowing box (a lamp) of this size, centred on `pos`, that also lights round it.
    Lamp([f32; 3]),
    /// A glowing upright cylinder, centred on `pos`, that also lights round it.
    RoundLamp { radius: f32, height: f32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub shape: Shape,
    pub pos: [f32; 3],
    pub yaw_deg: f32,
    /// Lean toward the east, in degrees, about `pos` (a lever's handle).
    pub tilt_deg: f32,
    pub scale: [f32; 3],
    /// Shapes: their colour. Models: a tint (None = as made).
    pub color: Option<[f32; 3]>,
}

/// Village pieces are 2 units wide at full size; at this scale one fills one square.
const VILLAGE: f32 = 0.5;
/// Roofs are squashed: full height looked like a second house on top.
const ROOF_Y: f32 = 0.25;
/// Height of one village storey at that scale.
pub const STORY: f32 = 1.56;
const STONE: [f32; 3] = [0.55, 0.55, 0.52];
const WOOD: [f32; 3] = [0.45, 0.3, 0.18];
const BOARD: [f32; 3] = [0.65, 0.5, 0.3];
const WATER: [f32; 3] = [0.15, 0.3, 0.5];
/// The dark inside of a house, seen through windows and doors.
const INSIDE: [f32; 3] = [0.1, 0.08, 0.06];
/// Village walls are this thick inside the footprint edge.
const WALL_INSET: f32 = 0.05;
/// Under the squashed roof: fills the open ends without poking through the tiles.
const GABLE_Y: f32 = 0.75;
/// How far outside the front wall someone stands at the door: the middle of the
/// square in front of it, where they step out to.
const DOORSTEP: f32 = 0.5;

pub const RECIPES: [&str; 55] = [
    "cottage", "house", "tavern", "barn", "temple", "tower", "castle", "shop", "well", "fountain", "signpost", "crossroads",
    "noticeboard", "lamppost", "clock", "lever", "lever_pulled", "ladder_down", "altar", "altar_green", "altar_black",
    "altar_red", "altar_white", "altar_blue", "brazier", "brazier_unlit", "lantern", "lantern_off",
    "torch", "torch_cyan", "torch_indigo", "torch_vermilion", "tikitorch", "mountain1", "mountain2", "mountain3",
    "mountain4", "mountain5", "grave", "coffin", "lair", "nest", "portal",
    "campfire", "campfire_out", "teleporter", "magic_portal", "pier",
    "rowboat", "longship", "fishboat", "galleon", "big_galleon", "big_galleon_west", "wagon",
];
pub const WALL_STYLES: [&str; 7] = ["stone", "brick", "plaster", "wood", "hedge", "rock", "gate"];

/// Squares a picture covers (32 px each), at least 1 by 1.
pub fn footprint(w_px: u32, h_px: u32) -> (u32, u32) {
    (w_px.div_ceil(32).max(1), h_px.div_ceil(32).max(1))
}

fn part(shape: Shape, pos: [f32; 3], yaw_deg: f32, scale: [f32; 3], color: Option<[f32; 3]>) -> Part {
    Part { shape, pos, yaw_deg, tilt_deg: 0.0, scale, color }
}

fn model(file: &'static str, pos: [f32; 3], yaw_deg: f32, scale: f32) -> Part {
    part(Shape::Model(file), pos, yaw_deg, [scale; 3], None)
}

fn cube(size: [f32; 3], pos: [f32; 3], color: [f32; 3]) -> Part {
    part(Shape::Box(size), pos, 0.0, [1.0; 3], Some(color))
}

fn cylinder(radius: f32, height: f32, pos: [f32; 3], color: [f32; 3]) -> Part {
    part(Shape::Cylinder { radius, height }, pos, 0.0, [1.0; 3], Some(color))
}

struct Style {
    wall: &'static str,
    window: &'static str,
    door: &'static str,
    corner: &'static str,
    roof: &'static str,
    stories: u32,
    chimney: bool,
}

fn style(recipe: &str) -> Option<Style> {
    let plaster = Style {
        wall: "buildings/Wall_Plaster_Straight.gltf",
        window: "buildings/Wall_Plaster_Window_Wide_Round.gltf",
        door: "buildings/Wall_Plaster_Door_Round.gltf",
        corner: "buildings/Corner_Exterior_Wood.gltf",
        roof: "buildings/Roof_RoundTiles_4x4.gltf",
        stories: 1,
        chimney: false,
    };
    let brick = Style {
        wall: "buildings/Wall_UnevenBrick_Straight.gltf",
        window: "buildings/Wall_UnevenBrick_Window_Wide_Round.gltf",
        door: "buildings/Wall_UnevenBrick_Door_Round.gltf",
        corner: "buildings/Corner_Exterior_Brick.gltf",
        ..plaster
    };
    Some(match recipe {
        "cottage" => plaster,
        "house" => Style { corner: "buildings/Corner_Exterior_Brick.gltf", chimney: true, ..plaster },
        "tavern" => Style { stories: 2, chimney: true, ..brick },
        "barn" => Style {
            wall: "buildings/Wall_Plaster_WoodGrid.gltf",
            window: "buildings/Wall_Plaster_WoodGrid.gltf",
            ..plaster
        },
        "temple" => Style { stories: 2, ..brick },
        "tower" => Style { stories: 3, roof: "buildings/Roof_Tower_RoundTiles.gltf", ..brick },
        _ => return None,
    })
}

/// The parts of a recipe over a `w` by `h` footprint. Unknown recipe: None (keep the card).
pub fn parts(recipe: &str, w: u32, h: u32) -> Option<Vec<Part>> {
    // Long tables and benches join like walls: `longtable_5` joins north and south.
    if let Some((kind, mask)) = recipe.split_once('_')
        && let Ok(mask) = u8::from_str_radix(mask, 16)
        && mask < 16
    {
        match kind {
            "longtable" => return Some(run(mask, 0.85, 0.45, BOARD)),
            "bench" => return Some(run(mask, 0.3, 0.25, [0.45, 0.2, 0.12])),
            "moat" => return Some(moat(mask)),
            // A rowboat facing one of 8 ways: 1 north, 3 east, 5 south, 7 west.
            "rowboat" if (1..=8).contains(&mask) => return Some(turned(rowboat(), (mask as f32 - 3.0) * 45.0)),
            _ => {}
        }
    }
    match recipe {
        "well" => Some(well()),
        "fountain" => Some(fountain()),
        "signpost" => Some(signpost()),
        "crossroads" => Some(crossroads()),
        "noticeboard" => Some(noticeboard()),
        "lamppost" => Some(lamppost()),
        "clock" => Some(clock()),
        "lever" => Some(lever(35.0)),
        "lever_pulled" => Some(lever(-35.0)),
        "ladder_down" => Some(ladder_down()),
        "altar" => Some(altar(None)),
        "altar_green" => Some(altar(Some([0.15, 0.5, 0.2]))),
        "altar_black" => Some(altar(Some([0.08, 0.08, 0.08]))),
        "altar_red" => Some(altar(Some([0.6, 0.08, 0.06]))),
        "altar_white" => Some(altar(Some([0.9, 0.9, 0.88]))),
        "altar_blue" => Some(altar(Some([0.1, 0.25, 0.7]))),
        "brazier" => Some(brazier(true)),
        "brazier_unlit" => Some(brazier(false)),
        "lantern" => Some(lantern(true)),
        "lantern_off" => Some(lantern(false)),
        "torch" => Some(torch(0.6, FLAME)),
        "torch_cyan" => Some(torch(0.6, [0.4, 1.0, 1.0])),
        "torch_indigo" => Some(torch(0.6, [0.5, 0.4, 1.0])),
        "torch_vermilion" => Some(torch(0.6, [1.0, 0.35, 0.2])),
        "tikitorch" => Some(torch(0.9, FLAME)),
        "teleporter" => Some(teleporter()),
        "magic_portal" => Some(magic_portal()),
        "pier" => Some(pier()),
        "rowboat" => Some(rowboat()),
        "longship" => Some(longship()),
        "fishboat" => Some(fishboat()),
        "galleon" => Some(galleon(0.9, 0.35)),
        "big_galleon" => Some(galleon(1.9, 0.7)),
        "big_galleon_west" => Some(turned(galleon(1.9, 0.7), 180.0)),
        "wagon" => Some(wagon()),
        "campfire" => Some(campfire(true)),
        "campfire_out" => Some(campfire(false)),
        "grave" => Some(grave()),
        "coffin" => Some(coffin()),
        "lair" => Some(lair()),
        "nest" => Some(nest()),
        "portal" => Some(portal()),
        "mountain1" => Some(mountain("hexagon/mountain_A.gltf", 0.0, 0.8, 1)),
        "mountain2" => Some(mountain("hexagon/mountain_B.gltf", 60.0, 0.8, 2)),
        "mountain3" => Some(mountain("hexagon/mountain_C.gltf", 120.0, 0.8, 3)),
        "mountain4" => Some(mountain("hexagon/mountain_A.gltf", 180.0, 0.85, 4)),
        "mountain5" => Some(mountain("hexagon/mountain_B.gltf", 240.0, 0.85, 5)),
        "castle" => Some(castle(w, h)),
        "shop" => Some(shop(w, h)),
        _ => style(recipe).map(|s| village(&s, w, h)),
    }
}

/// One side of a footprint: (side, squares along it, where square i sits, yaw).
type Side<'a> = (u8, u32, &'a dyn Fn(f32) -> (f32, f32), f32);

/// Where someone stands just outside the front door, relative to the middle of the
/// footprint; None for things without a door (wells, castles, signposts).
pub fn doorstep(recipe: &str, w: u32, h: u32) -> Option<[f32; 3]> {
    if recipe != "shop" && style(recipe).is_none() {
        return None;
    }
    // `village` puts the door on the south side at square w / 2, counted from the east.
    Some([w as f32 / 2.0 - (w / 2) as f32 - 0.5, 0.0, h as f32 / 2.0 + DOORSTEP])
}

/// Walls round the edge (door in the middle of the south side), posts at the corners,
/// a roof on top, and maybe a chimney.
fn village(s: &Style, w: u32, h: u32) -> Vec<Part> {
    let (fw, fh) = (w as f32, h as f32);
    let mut out = Vec::new();
    for story in 0..s.stories {
        let y = story as f32 * STORY;
        // Pieces run east-west unturned.
        let sides: [Side; 4] = [
            (N, w, &|t| (-fw / 2.0 + t, -fh / 2.0), 0.0),
            (E, h, &|t| (fw / 2.0, -fh / 2.0 + t), 90.0),
            (S, w, &|t| (fw / 2.0 - t, fh / 2.0), 180.0),
            (W, h, &|t| (-fw / 2.0, fh / 2.0 - t), 270.0),
        ];
        for (side, len, at, yaw) in sides {
            for i in 0..len {
                let (x, z) = at(i as f32 + 0.5);
                let file = if story == 0 && side == S && i == len / 2 {
                    s.door
                } else if len == 1 || i % 2 == 1 {
                    s.window
                } else {
                    s.wall
                };
                out.push(model(file, [x, y, z], yaw, VILLAGE));
            }
        }
        for (i, (cx, cz)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].into_iter().enumerate() {
            out.push(model(s.corner, [cx * fw / 2.0, y, cz * fh / 2.0], i as f32 * 90.0, VILLAGE));
        }
    }
    let top = s.stories as f32 * STORY;
    // Fill the hollow pieces: a solid inside behind windows and a wedge under the roof.
    let (iw, ih) = (fw - 2.0 * WALL_INSET, fh - 2.0 * WALL_INSET);
    out.push(cube([iw, top, ih], [0.0, top / 2.0, 0.0], INSIDE));
    out.push(part(Shape::Wedge([iw, GABLE_Y, ih]), [0.0, top, 0.0], 0.0, [1.0; 3], Some(INSIDE)));
    // The 4x4 roof covers 2 by 2 squares at village scale; stretch it to the footprint.
    out.push(part(Shape::Model(s.roof), [0.0, top, 0.0], 0.0, [fw / 4.0, ROOF_Y, fh / 4.0], None));
    if s.chimney {
        out.push(model("buildings/Prop_Chimney.gltf", [fw / 4.0, top, 0.0], 0.0, VILLAGE));
    }
    out
}

/// A one-storey house with a sign board high on the front (south) wall, over the door.
fn shop(w: u32, h: u32) -> Vec<Part> {
    let mut out = village(&style("cottage").expect("cottage style"), w, h);
    let fh = h as f32;
    let (wide, tall) = if w >= 2 { ((w as f32 - 1.1).clamp(0.7, 2.0), 0.3) } else { (0.7, 0.22) };
    let (y, z) = (STORY - 0.08 - tall, fh / 2.0 + 0.17);
    out.push(cube([wide, tall, 0.02], [0.0, y + tall / 2.0, z - 0.015], BOARD));
    out.push(part(Shape::Sign([wide, tall]), [0.0, y, z], 0.0, [1.0; 3], None));
    out
}

/// The stone city walls on every edge square, a gateway in the middle of the south
/// side, and a tall square tower on each corner. One square alone is one tower.
fn castle(w: u32, h: u32) -> Vec<Part> {
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

const IRON: [f32; 3] = [0.2, 0.2, 0.22];

/// An iron grid filling a doorway that runs east-west, in the middle of the wall.
fn portcullis() -> Vec<Part> {
    let mut out: Vec<Part> = (0..5).map(|i| cube([0.025, 0.8, 0.025], [-0.2 + i as f32 * 0.1, 0.4, 0.0], IRON)).collect();
    out.extend([0.15, 0.4, 0.65].map(|y| cube([0.5, 0.025, 0.025], [0.0, y, 0.0], IRON)));
    out
}

/// A square stone tower: the KayKit pillar `wide` times across and `high` times up,
/// with slit windows that climb round it, one face higher each time, like a stair
/// winding up inside.
fn tower(pos: [f32; 3], wide: f32, high: f32) -> Vec<Part> {
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

fn well() -> Vec<Part> {
    vec![
        cylinder(0.35, 0.5, [0.0, 0.25, 0.0], STONE),
        cylinder(0.27, 0.52, [0.0, 0.26, 0.0], WATER),
        cube([0.08, 1.0, 0.08], [-0.3, 0.5, 0.0], WOOD),
        cube([0.08, 1.0, 0.08], [0.3, 0.5, 0.0], WOOD),
        cube([0.8, 0.08, 0.5], [0.0, 1.04, 0.0], WOOD),
    ]
}

fn fountain() -> Vec<Part> {
    vec![
        cylinder(0.45, 0.25, [0.0, 0.125, 0.0], STONE),
        cylinder(0.4, 0.27, [0.0, 0.135, 0.0], WATER),
        cylinder(0.08, 0.7, [0.0, 0.35, 0.0], STONE),
        cylinder(0.2, 0.08, [0.0, 0.7, 0.0], STONE),
    ]
}

fn signpost() -> Vec<Part> {
    vec![cube([0.08, 1.1, 0.08], [0.0, 0.55, 0.0], WOOD), cube([0.6, 0.25, 0.05], [0.0, 0.95, 0.0], BOARD)]
}

/// A post with two arms, one each way.
fn crossroads() -> Vec<Part> {
    vec![
        cube([0.08, 1.3, 0.08], [0.0, 0.65, 0.0], WOOD),
        cube([0.7, 0.18, 0.05], [0.0, 1.1, 0.0], BOARD),
        cube([0.05, 0.18, 0.7], [0.0, 0.85, 0.0], BOARD),
    ]
}

/// A stone post on the dungeon plinth, with a lamp in an iron cage open on all four sides.
fn lamppost() -> Vec<Part> {
    const IRON: [f32; 3] = [0.15, 0.15, 0.17];
    let mut out = vec![
        model("dungeon/Pedestal.gltf", [0.0; 3], 0.0, 0.22),
        cube([0.1, 0.9, 0.1], [0.0, 0.93, 0.0], STONE),
        cube([0.26, 0.04, 0.26], [0.0, 1.4, 0.0], IRON),
        part(Shape::Lamp([0.14; 3]), [0.0, 1.52, 0.0], 0.0, [1.0; 3], Some([1.0, 0.85, 0.35])),
        cube([0.3, 0.04, 0.3], [0.0, 1.66, 0.0], IRON),
        cube([0.12, 0.06, 0.12], [0.0, 1.71, 0.0], IRON),
    ];
    for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        out.push(cube([0.03, 0.22, 0.03], [x * 0.115, 1.53, z * 0.115], IRON));
    }
    out
}

/// A tall wooden case on a plinth: a hood with a round dial under a little gable, and
/// a gold pendulum behind a glass door in the trunk.
fn clock() -> Vec<Part> {
    const DIAL: [f32; 3] = [0.9, 0.85, 0.7];
    const BRASS: [f32; 3] = [0.85, 0.65, 0.25];
    const TRIM: [f32; 3] = [0.6, 0.42, 0.25];
    vec![
        cube([0.42, 0.12, 0.32], [0.0, 0.06, 0.0], WOOD),
        cube([0.32, 0.7, 0.24], [0.0, 0.47, 0.0], WOOD),
        cube([0.22, 0.52, 0.02], [0.0, 0.47, 0.12], TRIM),
        cube([0.17, 0.46, 0.02], [0.0, 0.47, 0.125], INSIDE),
        cube([0.015, 0.3, 0.01], [0.0, 0.55, 0.14], BRASS),
        disc(0.045, 0.015, [0.0, 0.38, 0.14], BRASS),
        cube([0.38, 0.04, 0.28], [0.0, 0.84, 0.0], TRIM),
        cube([0.36, 0.32, 0.26], [0.0, 1.02, 0.0], WOOD),
        disc(0.13, 0.02, [0.0, 1.02, 0.13], BRASS),
        disc(0.11, 0.02, [0.0, 1.02, 0.14], DIAL),
        cube([0.015, 0.09, 0.01], [0.0, 1.06, 0.155], INSIDE),
        cube([0.06, 0.015, 0.01], [0.03, 1.02, 0.155], INSIDE),
        cube([0.42, 0.04, 0.32], [0.0, 1.2, 0.0], TRIM),
        part(Shape::Wedge([0.42, 0.14, 0.3]), [0.0, 1.22, 0.0], 0.0, [1.0; 3], Some(WOOD)),
        cube([0.04, 0.05, 0.04], [0.0, 1.38, 0.0], BRASS),
    ]
}

/// A cylinder turned to face south, like a coin standing on edge.
fn disc(radius: f32, thick: f32, pos: [f32; 3], color: [f32; 3]) -> Part {
    Part { yaw_deg: 90.0, tilt_deg: 90.0, ..cylinder(radius, thick, pos, color) }
}

/// One square of a long table or bench: a top `width` wide at `height`, reaching out to
/// the joined sides in `mask`. Ends, corners and lone squares stand on four legs.
fn run(mask: u8, width: f32, height: f32, color: [f32; 3]) -> Vec<Part> {
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

/// Water filling the square, with a low brick bank on each side that has no moat next
/// to it, and a brick post on each inside corner so bends close up. No two faces lie
/// in one place (they would flicker): banks sink a little, reach a hair past the
/// water's edge, and the east and west ones stop at the north and south ones.
fn moat(mask: u8) -> Vec<Part> {
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

/// An iron knob with a handle leaning `tilt_deg` toward the east (west when negative).
fn lever(tilt_deg: f32) -> Vec<Part> {
    const IRON: [f32; 3] = [0.35, 0.35, 0.38];
    const LENGTH: f32 = 0.5;
    let t = tilt_deg.to_radians();
    let (x, y) = (t.sin() * LENGTH / 2.0, 0.15 + t.cos() * LENGTH / 2.0);
    let grip = (t.sin() * LENGTH, 0.15 + t.cos() * LENGTH);
    vec![
        cylinder(0.18, 0.2, [0.0, 0.1, 0.0], IRON),
        Part { tilt_deg, ..cube([0.05, LENGTH, 0.05], [x, y, 0.0], WOOD) },
        cube([0.1, 0.1, 0.1], [grip.0, grip.1, 0.0], [0.6, 0.1, 0.1]),
    ]
}

/// A dark hole in a wooden frame, with the top of the up-ladder (the same model) poking out.
fn ladder_down() -> Vec<Part> {
    vec![
        cube([0.6, 0.02, 0.6], [0.0, 0.01, 0.0], WOOD),
        cube([0.5, 0.025, 0.5], [0.0, 0.0125, 0.0], INSIDE),
        model("platformer/Ladder_medium.gltf", [0.0, -0.95, -0.2], 0.0, 0.55),
    ]
}

/// A stone block under a wider slab, with a god's cloth over the top and down the front.
fn altar(cloth: Option<[f32; 3]>) -> Vec<Part> {
    let mut out = vec![cube([0.7, 0.5, 0.45], [0.0, 0.25, 0.0], STONE), cube([0.8, 0.06, 0.55], [0.0, 0.53, 0.0], STONE)];
    if let Some(c) = cloth {
        out.push(cube([0.4, 0.01, 0.56], [0.0, 0.565, 0.0], c));
        out.push(cube([0.4, 0.3, 0.01], [0.0, 0.41, 0.28], c));
    }
    out
}

/// An iron bowl on three legs; lit, its fire lights round it, unlit it holds ash.
fn brazier(lit: bool) -> Vec<Part> {
    const IRON: [f32; 3] = [0.2, 0.2, 0.22];
    let fill = if lit {
        part(Shape::Lamp([0.25, 0.12, 0.25]), [0.0, 0.7, 0.0], 0.0, [1.0; 3], Some([1.0, 0.55, 0.15]))
    } else {
        cube([0.25, 0.02, 0.25], [0.0, 0.67, 0.0], [0.3, 0.3, 0.3])
    };
    let mut out = vec![cylinder(0.22, 0.12, [0.0, 0.6, 0.0], IRON), fill];
    for yaw in [0.0_f32, 120.0, 240.0] {
        let (s, c) = yaw.to_radians().sin_cos();
        out.push(cube([0.04, 0.6, 0.04], [s * 0.15, 0.3, c * 0.15], IRON));
    }
    out
}

/// Mountains are as tall as a two-storey house: the hexagon mountain stretched up this much.
const MOUNTAIN_STRETCH: f32 = 2.4;
/// Mountain squares are raised this much (`terrain::ground_height`); the mountain starts
/// at the ground round it.
const MOUNTAIN_RAISE: f32 = 0.6;
const ROCKS: [&str; 4] =
    ["hexagon/rock_single_B.gltf", "hexagon/rock_single_C.gltf", "hexagon/rock_single_D.gltf", "hexagon/rock_single_E.gltf"];

/// A stretched hexagon mountain (`scale` wide, turned `yaw`), with rocks set into its
/// sides so the stretched faces look like rock again. `seed` picks where they go.
fn mountain(file: &'static str, yaw: f32, scale: f32, seed: u32) -> Vec<Part> {
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

const DIRT: [f32; 3] = [0.35, 0.25, 0.15];

const GOLD: [f32; 3] = [0.95, 0.75, 0.25];

/// A gold ring on the floor round a cyan glow, with a gold cross over it.
fn teleporter() -> Vec<Part> {
    vec![
        cylinder(0.42, 0.03, [0.0, 0.015, 0.0], GOLD),
        part(Shape::RoundLamp { radius: 0.34, height: 0.02 }, [0.0, 0.025, 0.0], 0.0, [1.0; 3], Some([0.3, 0.9, 1.0])),
        cube([0.06, 0.012, 0.6], [0.0, 0.04, 0.0], GOLD),
        cube([0.6, 0.012, 0.06], [0.0, 0.04, 0.0], GOLD),
    ]
}

/// A dark blue ring standing on its edge round a blue glow, facing south.
fn magic_portal() -> Vec<Part> {
    let standing = |shape, color| Part { yaw_deg: 90.0, tilt_deg: 90.0, ..part(shape, [0.0, 0.42, 0.0], 0.0, [1.0; 3], Some(color)) };
    let mut out = vec![
        standing(Shape::Cylinder { radius: 0.4, height: 0.05 }, [0.15, 0.1, 0.3]),
        standing(Shape::RoundLamp { radius: 0.35, height: 0.06 }, [0.45, 0.25, 1.0]),
    ];
    // Three pale arms that curl in to the middle, like a whirlpool.
    for arm in 0..3 {
        for j in 0..13 {
            let t = j as f32 / 12.0;
            let (r, a) = (0.04 + 0.27 * t, arm as f32 * 120.0 + t * 220.0);
            let (sin, cos) = a.to_radians().sin_cos();
            out.push(Part {
                tilt_deg: -(a + 90.0),
                ..cube([0.04 + 0.09 * t, 0.02 + 0.025 * t, 0.09], [r * cos, 0.42 + r * sin, 0.0], [0.75, 0.9, 1.0])
            });
        }
    }
    out
}

/// Planks across the square on two beams, on four posts down into the water.
fn pier() -> Vec<Part> {
    let mut out: Vec<Part> = (0..6).map(|i| cube([1.0, 0.04, 0.15], [0.0, 0.1, -0.42 + i as f32 * 0.168], WOOD)).collect();
    for x in [-0.35, 0.35] {
        out.push(cube([0.08, 0.06, 1.0], [x, 0.05, 0.0], WOOD));
        for z in [-0.4, 0.4] {
            out.push(cube([0.1, 0.4, 0.1], [x, -0.1, z], WOOD));
        }
    }
    out
}

const HULL: [f32; 3] = [0.42, 0.25, 0.13];
const DECK: [f32; 3] = [0.3, 0.18, 0.1];
const SAIL: [f32; 3] = [0.92, 0.9, 0.82];

/// Parts turned `yaw_deg` clockwise (seen from above) round the middle.
fn turned(parts: Vec<Part>, yaw_deg: f32) -> Vec<Part> {
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
fn wheel(radius: f32, pos: [f32; 3]) -> Vec<Part> {
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

fn rowboat() -> Vec<Part> {
    let mut out = hull(1.1, 0.4, 0.15, true);
    out.extend([-0.2, 0.15].map(|x| cube([0.08, 0.02, 0.36], [x, 0.09, 0.0], WOOD)));
    out.extend(oars(&[-0.2, 0.15], 0.4, 0.07));
    out
}

/// A long, low ship pointed at both ends, with a curling prow, a row of round red and
/// white shields down each side, oars, and a striped sail.
fn longship() -> Vec<Part> {
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

fn fishboat() -> Vec<Part> {
    let mut out = hull(1.5, 0.45, 0.18, false);
    out.extend(mast(0.1, 0.9, [0.4, 0.45], SAIL));
    // A net heaped at the stern.
    out.push(cube([0.25, 0.06, 0.3], [-0.5, 0.09, 0.0], [0.55, 0.5, 0.4]));
    out
}

/// A tall ship: a square stern with a raised cabin on it, two masts with two sails
/// each, and a pole out over the bow. `len` sets the size; the rest follows it.
fn galleon(len: f32, beam: f32) -> Vec<Part> {
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
fn wagon() -> Vec<Part> {
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

/// A ring of stones round crossed logs; lit, a fire burns on them and lights round it.
fn campfire(lit: bool) -> Vec<Part> {
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
        out.push(part(Shape::RoundLamp { radius: 0.1, height: 0.14 }, [0.0, 0.14, 0.0], 0.0, [1.0; 3], Some(FLAME)));
    }
    out
}

// Monster generators: where monsters come from.

/// A headstone at the north end of a low mound of earth.
fn grave() -> Vec<Part> {
    vec![
        cube([0.36, 0.08, 0.6], [0.0, 0.04, 0.08], DIRT),
        cube([0.3, 0.36, 0.08], [0.0, 0.18, -0.28], STONE),
        disc(0.15, 0.08, [0.0, 0.36, -0.28], STONE),
    ]
}

/// A dark wooden coffin with a pale cross on its lid.
fn coffin() -> Vec<Part> {
    const DARK: [f32; 3] = [0.25, 0.15, 0.1];
    const CROSS: [f32; 3] = [0.8, 0.75, 0.6];
    vec![
        cube([0.32, 0.14, 0.7], [0.0, 0.07, 0.0], DARK),
        cube([0.34, 0.03, 0.72], [0.0, 0.155, 0.0], WOOD),
        cube([0.04, 0.01, 0.3], [0.0, 0.175, -0.05], CROSS),
        cube([0.16, 0.01, 0.04], [0.0, 0.175, -0.12], CROSS),
    ]
}

/// A heap of rocks round a dark cave mouth that faces south.
fn lair() -> Vec<Part> {
    vec![
        part(Shape::Model("hexagon/rock_single_E.gltf"), [-0.22, 0.0, -0.05], 30.0, [1.8, 3.0, 1.8], None),
        part(Shape::Model("hexagon/rock_single_C.gltf"), [0.24, 0.0, -0.05], 200.0, [2.0, 3.0, 2.0], None),
        part(Shape::Model("hexagon/rock_single_D.gltf"), [0.0, 0.0, -0.25], 90.0, [2.4, 4.5, 2.4], None),
        part(Shape::Model("hexagon/rock_single_B.gltf"), [0.0, 0.42, 0.0], 150.0, [2.2, 1.6, 2.0], None),
        cube([0.28, 0.36, 0.3], [0.0, 0.18, 0.05], INSIDE),
    ]
}

/// A mound of earth in two steps with a dark hole on top (ants, mice).
fn nest() -> Vec<Part> {
    vec![
        cylinder(0.36, 0.14, [0.0, 0.07, 0.0], DIRT),
        cylinder(0.24, 0.26, [0.0, 0.13, 0.0], DIRT),
        cylinder(0.07, 0.02, [0.0, 0.26, 0.0], INSIDE),
    ]
}

/// A dark red ring on the ground with a glow in it that lights round it.
fn portal() -> Vec<Part> {
    vec![
        cylinder(0.4, 0.02, [0.0, 0.01, 0.0], [0.25, 0.04, 0.04]),
        part(Shape::RoundLamp { radius: 0.3, height: 0.02 }, [0.0, 0.025, 0.0], 0.0, [1.0; 3], Some([1.0, 0.2, 0.1])),
    ]
}

const FLAME: [f32; 3] = [1.0, 0.6, 0.2];

/// The KayKit torch (`scale` of it; it stands 0.4 below its middle) with a flame in its
/// cup that lights round it.
fn torch(scale: f32, flame: [f32; 3]) -> Vec<Part> {
    vec![
        model("kaykit/torch.gltf", [0.0, 0.4 * scale, 0.0], 0.0, scale),
        part(Shape::Lamp([0.08, 0.12, 0.08]), [0.0, 1.0 * scale, 0.0], 0.0, [scale / 0.6; 3], Some(flame)),
    ]
}

/// A lamp in a little iron frame that stands on the ground; unlit, its glass is dark.
fn lantern(lit: bool) -> Vec<Part> {
    const IRON: [f32; 3] = [0.15, 0.15, 0.17];
    let glass = if lit {
        part(Shape::Lamp([0.12, 0.16, 0.12]), [0.0, 0.14, 0.0], 0.0, [1.0; 3], Some([1.0, 0.85, 0.35]))
    } else {
        cube([0.12, 0.16, 0.12], [0.0, 0.14, 0.0], [0.25, 0.25, 0.2])
    };
    let mut out = vec![cube([0.2, 0.04, 0.2], [0.0, 0.02, 0.0], IRON), glass, cube([0.2, 0.04, 0.2], [0.0, 0.24, 0.0], IRON)];
    out.push(cylinder(0.03, 0.08, [0.0, 0.3, 0.0], IRON));
    for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        out.push(cube([0.02, 0.2, 0.02], [x * 0.08, 0.13, z * 0.08], IRON));
    }
    out
}

/// A wide board between two posts, under a little roof.
fn noticeboard() -> Vec<Part> {
    vec![
        cube([0.08, 1.4, 0.08], [-0.42, 0.7, 0.0], WOOD),
        cube([0.08, 1.4, 0.08], [0.42, 0.7, 0.0], WOOD),
        cube([0.8, 0.55, 0.05], [0.0, 0.95, 0.0], BOARD),
        cube([1.0, 0.05, 0.25], [0.0, 1.42, 0.0], WOOD),
    ]
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

/// KayKit stone brick walls, 4 units long and high: scaled to one square, a bit taller.
/// Unturned, `wall` runs east-west, `wall_corner` joins west and south, and
/// `wall_Tsplit` joins west, east and south.
const KAYKIT_WALL: [f32; 3] = [0.25, 0.3, 0.25];

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

#[cfg(test)]
mod tests {
    use super::*;

    fn models(parts: &[Part]) -> Vec<&'static str> {
        parts.iter().filter_map(|p| if let Shape::Model(f) = p.shape { Some(f) } else { None }).collect()
    }

    #[test]
    fn footprint_from_picture_size() {
        assert_eq!(footprint(32, 32), (1, 1));
        assert_eq!(footprint(64, 64), (2, 2));
        assert_eq!(footprint(64, 32), (2, 1));
        assert_eq!(footprint(33, 0), (2, 1));
    }

    #[test]
    fn cottage_has_one_door_on_the_south_side() {
        let parts = parts("cottage", 1, 1).unwrap();
        let doors: Vec<&Part> =
            parts.iter().filter(|p| matches!(p.shape, Shape::Model(f) if f.contains("Door"))).collect();
        assert_eq!(doors.len(), 1);
        assert!(doors[0].pos[2] > 0.0, "door faces south");
        assert_eq!(doors[0].yaw_deg, 180.0);
    }

    #[test]
    fn house_walls_go_round_the_footprint() {
        let parts = parts("house", 2, 2).unwrap();
        let walls = models(&parts).into_iter().filter(|f| f.contains("/Wall_")).count();
        assert_eq!(walls, 8);
        let roof = parts.iter().find(|p| matches!(p.shape, Shape::Model(f) if f.contains("Roof"))).unwrap();
        assert_eq!(roof.pos[1], STORY);
    }

    #[test]
    fn houses_are_filled() {
        let parts = parts("cottage", 2, 3).unwrap();
        let inside = parts.iter().find(|p| matches!(p.shape, Shape::Box(_))).expect("solid inside");
        assert_eq!(inside.shape, Shape::Box([1.9, STORY, 2.9]));
        assert!(parts.iter().any(|p| matches!(p.shape, Shape::Wedge(_)) && p.pos[1] == STORY), "roof wedge");
    }

    #[test]
    fn shop_is_one_storey_with_a_sign_on_the_front() {
        let sign = |w| parts("shop", w, 2).unwrap().into_iter().find(|p| matches!(p.shape, Shape::Sign(_))).expect("sign");
        let shop = parts("shop", 2, 2).unwrap();
        assert!(shop.iter().all(|p| p.pos[1] <= STORY), "one storey");
        let s = sign(2);
        let Shape::Sign([w, h]) = s.shape else { unreachable!() };
        assert!(s.pos[2] > 1.0, "on the south wall");
        assert!(s.pos[1] + h < STORY && w < 2.0, "on the wall, inside the front");
        let Shape::Sign([wide, _]) = sign(5).shape else { unreachable!() };
        assert!(wide > w, "a wider front gets a wider board");
    }

    #[test]
    fn doorstep_is_in_front_of_the_door() {
        for (recipe, w, h) in [("cottage", 1, 1), ("shop", 2, 2), ("house", 3, 2), ("tower", 1, 1)] {
            let door = parts(recipe, w, h).unwrap().into_iter().find(|p| matches!(p.shape, Shape::Model(f) if f.contains("Door"))).unwrap();
            let step = doorstep(recipe, w, h).unwrap();
            assert_eq!(step[0], door.pos[0], "{recipe} {w}x{h}: in line with the door");
            assert_eq!(step[2], door.pos[2] + 0.5, "{recipe}: the middle of the square outside the door");
        }
        assert_eq!(doorstep("well", 1, 1), None);
        assert_eq!(doorstep("castle", 3, 3), None);
    }

    #[test]
    fn tower_is_taller_than_a_house() {
        let top = |r| parts(r, 1, 1).unwrap().iter().map(|p| p.pos[1]).fold(0.0, f32::max);
        assert!(top("tower") > top("house") + STORY);
    }

    #[test]
    fn small_things_are_shapes() {
        for r in ["well", "fountain", "signpost", "crossroads", "noticeboard", "clock", "lever"] {
            let p = parts(r, 1, 1).unwrap();
            assert!(!p.is_empty() && models(&p).is_empty(), "{r}");
        }
        assert_eq!(parts("spaceship", 1, 1), None);
    }

    #[test]
    fn pulled_lever_leans_the_other_way() {
        let handle = |r| parts(r, 1, 1).unwrap().into_iter().find(|p| p.tilt_deg != 0.0).unwrap();
        let (off, on) = (handle("lever"), handle("lever_pulled"));
        assert!(off.tilt_deg > 0.0 && off.pos[0] > 0.0, "leans east");
        assert!(on.tilt_deg < 0.0 && on.pos[0] < 0.0, "leans west");
    }

    #[test]
    fn long_table_reaches_its_joined_sides() {
        let reach = |r| {
            let p = parts(r, 1, 1).unwrap();
            let edge = |f: fn(&Part) -> f32| p.iter().filter(|p| p.pos[1] > 0.4).map(f).fold(0.0, f32::max);
            (edge(|p| -p.pos[2]), edge(|p| p.pos[0]), p.len())
        };
        let (north, east, n) = reach("longtable_5");
        assert!(north > 0.4 && east < 0.1, "runs north-south");
        assert_eq!(n, 3, "a middle piece has no legs");
        assert_eq!(reach("longtable_0").2, 5, "a lone square has four legs");
        assert_eq!(parts("bench_G", 1, 1), None);
    }

    #[test]
    fn turned_quarter_turn_takes_east_to_south() {
        let p = turned(vec![cube([0.1; 3], [1.0, 0.5, 0.0], WOOD)], 90.0).remove(0);
        assert!((p.pos[0]).abs() < 1e-6 && (p.pos[2] - 1.0).abs() < 1e-6, "{:?}", p.pos);
        assert_eq!((p.pos[1], p.yaw_deg), (0.5, 90.0));
        // turningboat.x51 faces south: the rowboat's bow (east) turns to the south.
        assert_eq!(parts("rowboat_5", 2, 2).unwrap()[0].yaw_deg, 90.0);
    }

    #[test]
    fn patterns_fill_their_pictures() {
        for (w, h, px) in [brick_pixels(), wave_pixels()] {
            assert_eq!(px.len(), (w * h * 4) as usize);
        }
        // The bottom row of each brick row is mortar, so rows stay apart when tiled.
        let (w, _, px) = brick_pixels();
        assert!(px[(7 * w * 4) as usize..(8 * w * 4) as usize].chunks(4).all(|c| c[..3] == [185, 178, 165]));
    }

    #[test]
    fn moat_banks_on_closed_sides_and_posts_in_open_corners() {
        // Water, then banks or posts.
        assert_eq!(moat(0).len(), 1 + 4);
        assert_eq!(moat(N | E | S | W).len(), 1 + 4);
        assert_eq!(moat(N | S).len(), 1 + 2);
        assert_eq!(moat(N | E).len(), 1 + 2 + 1);
    }

    #[test]
    fn castle_has_four_towers_and_a_gate_on_the_south_side() {
        let m = models(&parts("castle", 3, 3).unwrap());
        assert_eq!(m.iter().filter(|f| f.contains("pillar")).count(), 4);
        let gate = parts("castle", 3, 3).unwrap().into_iter().find(|p| matches!(p.shape, Shape::Model(f) if f.contains("doorway")));
        assert_eq!(gate.map(|p| p.pos), Some([0.0, 0.0, 1.0]));
        assert_eq!(models(&parts("castle", 1, 1).unwrap()), ["kaykit/pillar.gltf"]);
        let small = parts("castle", 2, 2).unwrap();
        assert_eq!(models(&small).iter().filter(|f| f.contains("pillar")).count(), 4);
        let gate = small.into_iter().find(|p| matches!(p.shape, Shape::Model(f) if f.contains("doorway")));
        assert_eq!(gate.map(|p| p.pos), Some([0.0, 0.0, 0.5]));
    }

    #[test]
    fn stone_wall_turns_with_its_neighbours() {
        let we = wall_parts("stone", E | W);
        let ns = wall_parts("stone", N | S);
        assert_eq!(models(&we), ["kaykit/wall.gltf"]);
        assert_eq!((ns[0].yaw_deg - we[0].yaw_deg).rem_euclid(180.0), 90.0);
        assert_eq!(models(&wall_parts("stone", N | E)), ["kaykit/wall_corner.gltf"]);
    }

    #[test]
    fn village_wall_corner_has_two_arms_and_a_post() {
        let p = wall_parts("brick", N | E);
        let m = models(&p);
        assert_eq!(m.iter().filter(|f| f.contains("Wall_")).count(), 2);
        assert_eq!(m.iter().filter(|f| f.contains("Corner_")).count(), 1);
        assert_eq!(models(&wall_parts("plaster", E | W)).len(), 1, "a straight run is one piece");
        assert!(wall_parts("glass", N).is_empty());
    }

    #[test]
    fn every_model_file_exists() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
        if !root.is_dir() {
            return;
        }
        let mut all = Vec::new();
        for r in RECIPES {
            for (w, h) in [(1, 1), (2, 2), (3, 2)] {
                all.extend(models(&parts(r, w, h).unwrap()));
            }
        }
        for style in WALL_STYLES {
            for mask in 0..16 {
                all.extend(models(&wall_parts(style, mask)));
            }
        }
        for f in all {
            assert!(root.join(f).is_file(), "missing {f}");
        }
    }
}
