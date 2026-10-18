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
    /// A glowing box (a lamp) of this size, centred on `pos`, that also lights round it.
    Lamp([f32; 3]),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub shape: Shape,
    pub pos: [f32; 3],
    pub yaw_deg: f32,
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
/// Kenney's castle colours are sandy; this pulls them toward grey stone.
const STONE_TINT: [f32; 3] = [0.62, 0.74, 0.9];
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

pub const RECIPES: [&str; 14] =
    ["cottage", "house", "tavern", "barn", "temple", "tower", "castle", "shop", "well", "fountain", "signpost", "crossroads", "noticeboard", "lamppost"];
pub const WALL_STYLES: [&str; 6] = ["stone", "brick", "plaster", "wood", "hedge", "rock"];

/// Squares a picture covers (32 px each), at least 1 by 1.
pub fn footprint(w_px: u32, h_px: u32) -> (u32, u32) {
    (w_px.div_ceil(32).max(1), h_px.div_ceil(32).max(1))
}

fn part(shape: Shape, pos: [f32; 3], yaw_deg: f32, scale: [f32; 3], color: Option<[f32; 3]>) -> Part {
    Part { shape, pos, yaw_deg, scale, color }
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
    match recipe {
        "well" => Some(well()),
        "fountain" => Some(fountain()),
        "signpost" => Some(signpost()),
        "crossroads" => Some(crossroads()),
        "noticeboard" => Some(noticeboard()),
        "lamppost" => Some(lamppost()),
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

/// Stone blocks on every edge square: towers at the corners, a gateway in the middle
/// of the south side.
fn castle(w: u32, h: u32) -> Vec<Part> {
    let (fw, fh) = (w as f32, h as f32);
    let mut out = Vec::new();
    for j in 0..h {
        for i in 0..w {
            let (edge_x, edge_z) = (i == 0 || i == w - 1, j == 0 || j == h - 1);
            if !edge_x && !edge_z {
                continue;
            }
            let pos = [-fw / 2.0 + i as f32 + 0.5, 0.0, -fh / 2.0 + j as f32 + 0.5];
            let (file, yaw) = if edge_x && edge_z {
                ("castle/tower-square.glb", 0.0)
            } else if j == h - 1 && i == w / 2 {
                ("castle/wall-doorway.glb", 0.0)
            } else if edge_z {
                ("castle/wall.glb", 0.0)
            } else {
                ("castle/wall.glb", 90.0)
            };
            out.push(part(Shape::Model(file), pos, yaw, [1.0, 1.5, 1.0], Some(STONE_TINT)));
        }
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

/// Kenney castle blocks fill a whole square. Unturned, `wall.glb` runs east-west and
/// `wall-corner.glb` joins north and east (checked by screenshot).
fn stone_wall(mask: u8) -> Part {
    let (piece, yaw) = piece_for(mask);
    let file = match piece {
        Piece::Straight | Piece::End => "castle/wall.glb",
        Piece::Corner => "castle/wall-corner.glb",
        Piece::Post | Piece::Tee | Piece::Cross => "castle/tower-square.glb",
    };
    // piece_for's unturned Straight and End run north-south; wall.glb runs east-west.
    let yaw = if matches!(piece, Piece::Straight | Piece::End) { yaw + 90.0 } else { yaw };
    part(Shape::Model(file), [0.0; 3], yaw, [1.0, 1.2, 1.0], Some(STONE_TINT))
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
        for r in ["well", "fountain", "signpost", "crossroads", "noticeboard"] {
            let p = parts(r, 1, 1).unwrap();
            assert!(!p.is_empty() && models(&p).is_empty(), "{r}");
        }
        assert_eq!(parts("spaceship", 1, 1), None);
    }

    #[test]
    fn stone_wall_turns_with_its_neighbours() {
        let we = wall_parts("stone", E | W);
        let ns = wall_parts("stone", N | S);
        assert_eq!(models(&we), ["castle/wall.glb"]);
        assert_eq!((ns[0].yaw_deg - we[0].yaw_deg).rem_euclid(180.0), 90.0);
        assert_eq!(models(&wall_parts("stone", N | E)), ["castle/wall-corner.glb"]);
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
