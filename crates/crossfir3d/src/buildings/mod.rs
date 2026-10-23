//! Buildings, walls, and small built things, put together from model pieces and plain
//! shapes. The free village pack has pieces, not houses; no pack has a well or a sign.
//!
//! Positions are relative to the middle of the ground the thing stands on. 1 unit =
//! 1 square; x = east, z = south. Yaw is in degrees, clockwise seen from above.

pub use crate::walls::{E, N, S, W};

use crate::walls::{piece_for, Piece};

mod village;
mod castle;
mod walls;
mod terrain;
mod ships;
mod items;
mod creatures;
mod props;
mod dispatch;
#[cfg(test)]
mod tests;

use self::village::*;
use self::castle::*;
pub use self::walls::*;
use self::terrain::*;
use self::ships::*;
pub use self::items::*;
use self::creatures::*;
use self::props::*;
pub use self::dispatch::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A file under `assets/models/`.
    Model(&'static str),
    /// A box of this size, centred on `pos`.
    Box([f32; 3]),
    /// An upright cylinder, centred on `pos`.
    Cylinder { radius: f32, height: f32 },
    /// An upright cone, point up, centred on `pos`.
    Cone { radius: f32, height: f32 },
    /// An upright cone with its top cut off flat at 0.4 of its `radius`, centred on `pos`.
    Frustum { radius: f32, height: f32 },
    /// A ball, centred on `pos`.
    Ball(f32),
    /// A glowing ball, centred on `pos`, that also lights round it.
    Orb(f32),
    /// A ball of mist you can partly see through, centred on `pos`.
    Mist(f32),
    /// A glowing cone, a tongue of fire, centred on `pos` (no light of its own).
    Fire { radius: f32, height: f32 },
    /// A glowing box of this size, centred on `pos` (no light of its own).
    Glow([f32; 3]),
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

pub const RECIPES: [&str; 116] = [
    "cottage", "house", "tavern", "barn", "temple", "tower", "castle", "shop", "well", "fountain", "signpost", "crossroads",
    "noticeboard", "lamppost", "clock", "wallclock", "lever", "lever_pulled", "ladder_down", "altar", "altar_green", "altar_black",
    "altar_red", "altar_white", "altar_blue", "brazier", "brazier_unlit", "lantern", "lantern_off",
    "torch", "torch_cyan", "torch_indigo", "torch_vermilion", "tikitorch", "mountain1", "mountain2", "mountain3",
    "mountain4", "mountain5", "grave", "coffin", "lair", "nest", "portal",
    "campfire", "campfire_out", "teleporter", "magic_portal", "pier",
    "rowboat", "longship", "fishboat", "galleon", "big_galleon", "big_galleon_west", "wagon", "hidden", "pedestal", "alchemy_bath", "trapdoor", "lava_well", "rope_up", "volcano",
    "bomb", "button", "button_small", "forge",
    "chimney", "anchor", "bee", "acid_pool", "card", "card_skull", "magma", "coldmagma", "crack", "lips", "note", "rune", "charm", "bouquet", "fist", "ant_egg", "halo", "speedball", "mat", "letter", "map", "whistle", "utensils", "fog", "steam", "spider_web", "rabbit_foot", "doll", "cat_basket", "spinningwheel", "machine", "beehive", "bolt_silk", "crystalball", "dice", "fireplace", "shipswheel", "skull", "ten_kilo", "wheel_tall", "wheel_short", "tissue", "stove", "mailbox", "quiver", "frypan", "icecube", "smallicecube", "logs",
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

const IRON: [f32; 3] = [0.2, 0.2, 0.22];

/// A cylinder turned to face south, like a coin standing on edge.
fn disc(radius: f32, thick: f32, pos: [f32; 3], color: [f32; 3]) -> Part {
    Part { yaw_deg: 90.0, tilt_deg: 90.0, ..cylinder(radius, thick, pos, color) }
}

const DIRT: [f32; 3] = [0.35, 0.25, 0.15];

const GOLD: [f32; 3] = [0.95, 0.75, 0.25];
