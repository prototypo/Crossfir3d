//! Recipe name to parts: the dispatch tables and their lookups.

use super::*;

/// The parts of a recipe over a `w` by `h` footprint. Unknown recipe: None (keep the card).
pub fn parts(recipe: &str, w: u32, h: u32) -> Option<Vec<Part>> {
    if let Some(rest) = recipe.strip_prefix("burning/") {
        return burning(rest, w, h);
    }
    masked(recipe)
        .or_else(|| coloured(recipe))
        .or_else(|| PLAIN.iter().find(|(name, _)| *name == recipe).map(|(_, make)| make()))
        .or_else(|| sized(recipe, w, h))
}

/// `burning/house`: the house with flames on its roof.
pub(super) fn burning(rest: &str, w: u32, h: u32) -> Option<Vec<Part>> {
    let mut out = parts(rest, w, h)?;
    if rest == "castle" {
        // Inside the walls, from low down, sticking up over them.
        out.extend(flames(w.max(2) - 1, h.max(2) - 1, 0.2, 0.6));
    } else {
        let top = out.iter().map(|p| p.pos[1]).fold(0.0, f32::max) + 0.3;
        out.extend(flames(w, h, top, 0.0));
    }
    Some(out)
}

/// Recipes that end in a hex digit after an underscore, like `longtable_5`.
fn masked(recipe: &str) -> Option<Vec<Part>> {
    // Long tables and benches join like walls: `longtable_5` joins north and south.
    let (kind, mask) = recipe.split_once('_')?;
    let mask = u8::from_str_radix(mask, 16).ok().filter(|m| *m < 16)?;
    match kind {
        "longtable" => Some(run(mask, 0.85, 0.45, BOARD)),
        "bench" => Some(run(mask, 0.3, 0.25, [0.45, 0.2, 0.12])),
        "moat" => Some(moat(mask)),
        // Spike gates: the digit is how far up the spikes are, 0 down to all the way up.
        "spike" => Some(spike(mask as f32 / 11.0)),
        "spikes" => Some(spikes(mask as f32 / 8.0)),
        _ => aimed(kind, mask),
    }
}

/// Boats and cannons that point one of 8 ways.
pub(super) fn aimed(kind: &str, mask: u8) -> Option<Vec<Part>> {
    match kind {
        // A rowboat facing one of 8 ways: 1 north, 3 east, 5 south, 7 west.
        "rowboat" if (1..=8).contains(&mask) => Some(turned(rowboat(), (mask as f32 - 3.0) * 45.0)),
        // A cannon aimed one of 8 ways (0: not aimed, south).
        "cannon" if mask <= 8 => Some(turned(cannon(), (if mask == 0 { 5.0 } else { mask as f32 } - 3.0) * 45.0)),
        _ => None,
    }
}

/// Clothes, armour and the like in the colour after the colon: `armour:0.5,0.5,0.5`.
fn coloured(recipe: &str) -> Option<Vec<Part>> {
    let (kind, rgb) = recipe.split_once(':')?;
    let [Ok(r), Ok(g), Ok(b)] = rgb.split(',').map(str::parse::<f32>).collect::<Vec<_>>()[..] else {
        return None;
    };
    let c = [r, g, b];
    if let Some((_, make)) = COLOURED.iter().find(|(name, _)| *name == kind) {
        return Some(make(c));
    }
    if let Some(piece) = kind.strip_prefix("chess_") {
        return Some(chess_piece(piece, c));
    }
    ITEM_KINDS.contains(&kind).then(|| item(kind, c))
}

/// The recipes that need the footprint size, then the houses.
fn sized(recipe: &str, w: u32, h: u32) -> Option<Vec<Part>> {
    match recipe {
        "volcano" => Some(volcano(w, h)),
        "castle" => Some(castle(w, h)),
        "shop" => Some(shop(w, h)),
        _ => style(recipe).map(|s| village(&s, w, h)),
    }
}

type Tint = fn([f32; 3]) -> Vec<Part>;

type Plain = fn() -> Vec<Part>;

/// Recipes that take a colour: the kind before the colon picks the maker.
const COLOURED: &[(&str, Tint)] = &[
    ("armour", armour),
    ("garment", |c| garment(c, false)),
    ("cloak", |c| garment(c, true)),
    ("converter", converter),
    ("piano", |c| piano(c, false)),
    ("harpsichord", |c| piano(c, true)),
    ("slotmachine", slot_machine),
    ("snake", |c| snake(c, 1.0, false)),
    ("cobra", |c| snake(c, 1.2, true)),
    ("serpent", |c| snake(c, 3.0, false)),
    ("worm", |c| worm(c, 1.0)),
    ("big_worm", |c| worm(c, 2.5)),
    ("ant", |c| ant(c, 2.0)),
    ("big_ant", |c| ant(c, 3.0)),
    ("spider", spider),
    ("centipede", centipede),
    ("tentacle", tentacle),
    // A low mattress with a white pillow at its north end.
    ("futon", |c| {
        vec![
            cube([0.55, 0.08, 0.85], [0.0, 0.04, 0.0], c),
            cube([0.4, 0.06, 0.15], [0.0, 0.11, -0.3], [0.95, 0.95, 0.92]),
        ]
    }),
    // A roof piece: a ridge running east-west over the square.
    ("roof", |c| vec![part(Shape::Wedge([1.0, 0.5, 1.0]), [0.0; 3], 90.0, [1.0; 3], Some(c))]),
];

/// Recipes that need nothing but their name.
const PLAIN: &[(&str, Plain)] = &[
    // Something there that the player must not see, like a hidden trap.
    ("hidden", Vec::new),
    ("alchemy_bath", alchemy_bath),
    ("bomb", || vec![
        part(Shape::Ball(0.13), [0.0, 0.13, 0.0], 0.0, [1.0; 3], Some([0.08, 0.08, 0.1])),
        cylinder(0.04, 0.05, [0.0, 0.27, 0.0], IRON),
        cylinder(0.008, 0.1, [0.0, 0.34, 0.0], BOARD),
        Part { tilt_deg: 40.0, ..cylinder(0.008, 0.05, [0.012, 0.4, 0.0], BOARD) },
    ]),
    // A metal plate set in the floor, pressed by stepping on it.
    ("button", || vec![cylinder(0.25, 0.03, [0.0, 0.015, 0.0], IRON), cylinder(0.2, 0.05, [0.0, 0.025, 0.0], STEEL)]),
    ("button_small", || vec![cylinder(0.12, 0.03, [0.0, 0.015, 0.0], IRON), cylinder(0.09, 0.05, [0.0, 0.025, 0.0], STEEL)]),
    ("forge", forge),
    ("chimney", || vec![
        part(Shape::Bricks([0.45, 1.2, 0.45]), [0.0, 0.6, 0.0], 0.0, [1.0; 3], None),
        cube([0.55, 0.08, 0.55], [0.0, 1.24, 0.0], STONE),
    ]),
    ("anchor", anchor),
    ("bee", bee),
    // A green puddle of acid with bubbles on it.
    ("acid_pool", || vec![
        cylinder(0.38, 0.02, [0.0, 0.01, 0.0], [0.25, 0.7, 0.15]),
        part(Shape::Ball(0.05), [0.1, 0.03, -0.08], 0.0, [1.0, 0.6, 1.0], Some([0.6, 0.95, 0.5])),
        part(Shape::Ball(0.035), [-0.12, 0.03, 0.1], 0.0, [1.0, 0.6, 1.0], Some([0.6, 0.95, 0.5])),
        part(Shape::Ball(0.025), [0.0, 0.025, 0.15], 0.0, [1.0, 0.6, 1.0], Some([0.6, 0.95, 0.5])),
    ]),
    ("card", || card(None)),
    ("card_skull", || card(Some([0.08, 0.08, 0.08]))),
    ("magma", || magma(1.0)),
    ("coldmagma", || magma(0.35)),
    // Jagged dark gaps across the floor.
    ("crack", ||
        [(-0.3, 0.0, 20.0, 0.3), (-0.05, 0.05, -35.0, 0.3), (0.2, -0.05, 25.0, 0.3), (0.05, 0.2, 70.0, 0.2)]
            .into_iter()
            .map(|(x, z, yaw, len)| Part { yaw_deg: yaw, ..cube([len, 0.012, 0.035], [x, 0.006, z], [0.08, 0.07, 0.06]) })
            .collect(),
    ),
    // A magic mouth: red lips floating at head height.
    ("lips", || vec![
        part(Shape::Ball(0.07), [0.0, 1.03, 0.0], 0.0, [1.6, 0.45, 0.6], Some([0.8, 0.15, 0.2])),
        part(Shape::Ball(0.065), [0.0, 0.97, 0.0], 0.0, [1.5, 0.5, 0.6], Some([0.75, 0.12, 0.18])),
        part(Shape::Ball(0.05), [0.0, 1.0, 0.01], 0.0, [1.5, 0.15, 0.5], Some([0.15, 0.03, 0.05])),
    ]),
    ("note", note),
    // A rune drawn on the floor in faintly glowing red lines.
    ("rune", ||
        [(0.0, 0.0, 0.0, 0.4), (0.0, -0.12, 90.0, 0.25), (0.08, 0.1, 40.0, 0.18), (-0.08, 0.1, -40.0, 0.18)]
            .into_iter()
            .map(|(x, z, yaw, len)| Part { yaw_deg: yaw, ..part(Shape::Glow([0.025, 0.006, len]), [x, 0.004, z], 0.0, [1.0; 3], Some([0.7, 0.12, 0.1])) })
            .collect(),
    ),
    // A small silver plaque with a red jewel in it.
    ("charm", || vec![
        Part { yaw_deg: 25.0, ..cube([0.16, 0.015, 0.12], [0.0, 0.008, 0.0], [0.82, 0.84, 0.88]) },
        cylinder(0.03, 0.02, [0.0, 0.018, 0.0], [0.85, 0.08, 0.1]),
    ]),
    ("bouquet", bouquet),
    ("fist", fist),
    ("ant_egg", || vec![part(Shape::Ball(0.1), [0.0, 0.1, 0.0], 0.0, [1.0, 1.4, 1.0], Some([0.95, 0.93, 0.85]))]),
    ("halo", halo),
    ("speedball", || vec![part(Shape::Orb(0.15), [0.0, 0.35, 0.0], 0.0, [1.0; 3], Some([1.0, 0.25, 0.15]))]),
    ("doll", doll),
    ("mat", || vec![
        cube([0.62, 0.02, 0.62], [0.0, 0.01, 0.0], [0.55, 0.38, 0.15]),
        cube([0.54, 0.024, 0.54], [0.0, 0.012, 0.0], [0.8, 0.6, 0.28]),
    ]),
    // An envelope with its flap and a red wax seal.
    ("letter", || vec![
        cube([0.22, 0.008, 0.14], [0.0, 0.004, 0.0], [0.95, 0.9, 0.7]),
        Part { yaw_deg: 45.0, ..cube([0.11, 0.004, 0.11], [0.0, 0.009, -0.03], [0.88, 0.82, 0.6]) },
        cylinder(0.018, 0.008, [0.0, 0.013, 0.0], [0.75, 0.1, 0.1]),
    ]),
    // A parchment map with land and sea on it, rolled up a little at each end.
    ("map", || vec![
        cube([0.4, 0.006, 0.28], [0.0, 0.003, 0.0], [0.9, 0.82, 0.6]),
        cube([0.14, 0.004, 0.12], [-0.08, 0.007, 0.03], [0.45, 0.6, 0.3]),
        cube([0.1, 0.004, 0.08], [0.1, 0.007, -0.05], [0.45, 0.6, 0.3]),
        cube([0.12, 0.004, 0.1], [0.06, 0.006, 0.06], [0.35, 0.5, 0.75]),
        Part { tilt_deg: 90.0, ..cylinder(0.022, 0.4, [0.0, 0.022, -0.14], [0.85, 0.77, 0.55]) },
        Part { tilt_deg: 90.0, ..cylinder(0.022, 0.4, [0.0, 0.022, 0.14], [0.85, 0.77, 0.55]) },
    ]),
    ("whistle", || vec![
        Part { tilt_deg: 90.0, ..cylinder(0.022, 0.2, [0.0, 0.022, 0.0], [0.55, 0.33, 0.15]) },
        Part { tilt_deg: 90.0, ..cylinder(0.015, 0.05, [-0.12, 0.022, 0.0], [0.4, 0.22, 0.1]) },
        Part { tilt_deg: 90.0, ..cylinder(0.025, 0.02, [0.06, 0.022, 0.0], GOLD) },
        cube([0.03, 0.01, 0.02], [-0.04, 0.042, 0.0], [0.15, 0.08, 0.04]),
    ]),
    ("utensils", utensils),
    // Soft clouds you can partly see through.
    ("fog", || cloud([0.85, 0.87, 0.9])),
    ("steam", || cloud([0.97, 0.97, 0.97])),
    ("spider_web", spider_web),
    ("rabbit_foot", rabbit_foot),
    ("cat_basket", cat_basket),
    ("spinningwheel", spinning_wheel),
    ("machine", machine),
    // A straw beehive (a skep) on a stand, with its little door on the south side.
    ("beehive", || vec![
        cube([0.5, 0.3, 0.5], [0.0, 0.15, 0.0], WOOD),
        part(Shape::Frustum { radius: 0.3, height: 0.4 }, [0.0, 0.5, 0.0], 0.0, [1.0; 3], Some([0.85, 0.7, 0.35])),
        part(Shape::Ball(0.13), [0.0, 0.7, 0.0], 0.0, [1.0; 3], Some([0.85, 0.7, 0.35])),
        cube([0.08, 0.05, 0.04], [0.0, 0.33, 0.27], INSIDE),
    ]),
    // A roll of silk cloth lying east-west.
    ("bolt_silk", || vec![Part { tilt_deg: 90.0, ..cylinder(0.08, 0.5, [0.0, 0.08, 0.0], [0.4, 0.85, 0.95]) }]),
    ("crystalball", || vec![
        cylinder(0.12, 0.05, [0.0, 0.025, 0.0], GOLD),
        part(Shape::Orb(0.13), [0.0, 0.17, 0.0], 0.0, [1.0; 3], Some([0.6, 0.8, 1.0])),
    ]),
    ("dice", || [die([-0.06, 0.0, 0.0], 20.0, [5, 3, 1]), die([0.08, 0.0, 0.05], 65.0, [2, 6, 4])].concat()),
    ("fireplace", fireplace),
    ("shipswheel", ships_wheel),
    ("skull", skull),
    ("ten_kilo", kettlebell),
    // A wheel of fortune on a post.
    ("wheel_tall", || fortune_wheel(1.2)),
    ("wheel_short", || fortune_wheel(0.6)),
    ("tissue", || vec![
        cube([0.25, 0.12, 0.15], [0.0, 0.06, 0.0], [0.15, 0.25, 0.8]),
        cube([0.19, 0.09, 0.004], [0.0, 0.165, 0.0], [0.97, 0.97, 0.97]),
        Part { yaw_deg: 12.0, ..cube([0.16, 0.07, 0.004], [0.0, 0.155, 0.01], [0.93, 0.93, 0.95]) },
    ]),
    ("stove", stove),
    ("mailbox", || vec![
        cube([0.05, 0.55, 0.05], [0.0, 0.275, 0.0], WOOD),
        cube([0.2, 0.18, 0.4], [0.0, 0.64, 0.0], [0.25, 0.25, 0.3]),
        cube([0.03, 0.12, 0.03], [0.115, 0.7, -0.1], [0.8, 0.15, 0.1]),
    ]),
    ("quiver", quiver),
    ("frypan", || vec![
        cylinder(0.2, 0.06, [0.0, 0.03, 0.0], [0.12, 0.12, 0.13]),
        cube([0.3, 0.03, 0.05], [0.33, 0.04, 0.0], WOOD),
    ]),
    ("icecube", || vec![cube([0.35; 3], [0.0, 0.175, 0.0], [0.75, 0.9, 1.0])]),
    ("smallicecube", || vec![cube([0.18; 3], [0.0, 0.09, 0.0], [0.75, 0.9, 1.0])]),
    // The well with its water swapped for lava.
    ("lava_well", || {
        let mut out = well();
        out[1] = part(Shape::RoundLamp { radius: 0.27, height: 0.52 }, [0.0, 0.26, 0.0], 0.0, [1.0; 3], Some([1.0, 0.35, 0.05]));
        out
    }),
    // A rope hanging down from above, with a knot at the bottom.
    ("rope_up", rope_up),
    ("trapdoor", trapdoor),
    ("pedestal", || vec![
        cube([0.4, 0.08, 0.4], [0.0, 0.04, 0.0], STONE),
        cube([0.26, 0.5, 0.26], [0.0, 0.33, 0.0], STONE),
        cube([0.36, 0.06, 0.36], [0.0, 0.61, 0.0], STONE),
    ]),
    ("well", well),
    ("fountain", fountain),
    ("signpost", signpost),
    ("crossroads", crossroads),
    ("noticeboard", noticeboard),
    ("lamppost", lamppost),
    ("clock", clock),
    ("wallclock", wall_clock),
    ("lever", || lever(35.0)),
    ("lever_pulled", || lever(-35.0)),
    ("ladder_down", ladder_down),
    ("altar", || altar(None)),
    ("altar_green", || altar(Some([0.15, 0.5, 0.2]))),
    ("altar_black", || altar(Some([0.08, 0.08, 0.08]))),
    ("altar_red", || altar(Some([0.6, 0.08, 0.06]))),
    ("altar_white", || altar(Some([0.9, 0.9, 0.88]))),
    ("altar_blue", || altar(Some([0.1, 0.25, 0.7]))),
    ("brazier", || brazier(true)),
    ("brazier_unlit", || brazier(false)),
    ("lantern", || lantern(true)),
    ("lantern_off", || lantern(false)),
    ("torch", || torch(0.6, FLAME)),
    ("torch_cyan", || torch(0.6, [0.4, 1.0, 1.0])),
    ("torch_indigo", || torch(0.6, [0.5, 0.4, 1.0])),
    ("torch_vermilion", || torch(0.6, [1.0, 0.35, 0.2])),
    ("tikitorch", || torch(0.9, FLAME)),
    ("teleporter", teleporter),
    ("magic_portal", magic_portal),
    ("pier", pier),
    ("rowboat", rowboat),
    ("longship", longship),
    ("fishboat", fishboat),
    ("galleon", || galleon(0.9, 0.35)),
    ("big_galleon", || galleon(1.9, 0.7)),
    ("big_galleon_west", || turned(galleon(1.9, 0.7), 180.0)),
    ("wagon", wagon),
    ("campfire", || campfire(true)),
    ("logs", logs),
    ("campfire_out", || campfire(false)),
    ("grave", grave),
    ("coffin", coffin),
    ("lair", lair),
    ("nest", nest),
    ("portal", portal),
    ("mountain1", || mountain("hexagon/mountain_A.gltf", 0.0, 0.8, 1)),
    ("mountain2", || mountain("hexagon/mountain_B.gltf", 60.0, 0.8, 2)),
    ("mountain3", || mountain("hexagon/mountain_C.gltf", 120.0, 0.8, 3)),
    ("mountain4", || mountain("hexagon/mountain_A.gltf", 180.0, 0.85, 4)),
    ("mountain5", || mountain("hexagon/mountain_B.gltf", 240.0, 0.85, 5)),
];

/// One side of a footprint: (side, squares along it, where square i sits, yaw).
pub(super) type Side<'a> = (u8, u32, &'a dyn Fn(f32) -> (f32, f32), f32);

/// Where someone stands just outside the front door, relative to the middle of the
/// footprint; None for things without a door (wells, castles, signposts).
pub fn doorstep(recipe: &str, w: u32, h: u32) -> Option<[f32; 3]> {
    if recipe != "shop" && style(recipe).is_none() {
        return None;
    }
    // `village` puts the door on the south side at square w / 2, counted from the east.
    Some([w as f32 / 2.0 - (w / 2) as f32 - 0.5, 0.0, h as f32 / 2.0 + DOORSTEP])
}
