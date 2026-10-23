//! Village buildings, wells, signs, lamps, clocks, and other fixtures.

use super::*;

/// Walls round the edge (door in the middle of the south side), posts at the corners,
/// a roof on top, and maybe a chimney.
pub(super) fn village(s: &Style, w: u32, h: u32) -> Vec<Part> {
    let mut out = Vec::new();
    for story in 0..s.stories {
        storey(&mut out, s, (w, h), story);
    }
    village_top(&mut out, s, (w, h));
    out
}

/// The piece for square `i` of a side `len` long: the door, a window or plain wall.
fn wall_file(s: &Style, story: u32, side: u8, i: u32, len: u32) -> &'static str {
    if story == 0 && side == S && i == len / 2 {
        s.door
    } else if len == 1 || i % 2 == 1 {
        s.window
    } else {
        s.wall
    }
}

/// One storey: the walls on all four sides and a post at each corner.
fn storey(out: &mut Vec<Part>, s: &Style, (w, h): (u32, u32), story: u32) {
    let (fw, fh) = (w as f32, h as f32);
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
            out.push(model(wall_file(s, story, side, i, len), [x, y, z], yaw, VILLAGE));
        }
    }
    for (i, (cx, cz)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].into_iter().enumerate() {
        out.push(model(s.corner, [cx * fw / 2.0, y, cz * fh / 2.0], i as f32 * 90.0, VILLAGE));
    }
}

/// The solid inside, the wedge under the roof, the roof and maybe a chimney.
fn village_top(out: &mut Vec<Part>, s: &Style, (w, h): (u32, u32)) {
    let (fw, fh) = (w as f32, h as f32);
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
}

/// A one-storey house with a sign board high on the front (south) wall, over the door.
pub(super) fn shop(w: u32, h: u32) -> Vec<Part> {
    let mut out = village(&style("cottage").expect("cottage style"), w, h);
    let fh = h as f32;
    let (wide, tall) = if w >= 2 { ((w as f32 - 1.1).clamp(0.7, 2.0), 0.3) } else { (0.7, 0.22) };
    let (y, z) = (STORY - 0.08 - tall, fh / 2.0 + 0.17);
    out.push(cube([wide, tall, 0.02], [0.0, y + tall / 2.0, z - 0.015], BOARD));
    out.push(part(Shape::Sign([wide, tall]), [0.0, y, z], 0.0, [1.0; 3], None));
    out
}

pub(super) fn well() -> Vec<Part> {
    vec![
        cylinder(0.35, 0.5, [0.0, 0.25, 0.0], STONE),
        cylinder(0.27, 0.52, [0.0, 0.26, 0.0], WATER),
        cube([0.08, 1.0, 0.08], [-0.3, 0.5, 0.0], WOOD),
        cube([0.08, 1.0, 0.08], [0.3, 0.5, 0.0], WOOD),
        cube([0.8, 0.08, 0.5], [0.0, 1.04, 0.0], WOOD),
    ]
}

pub(super) fn fountain() -> Vec<Part> {
    vec![
        cylinder(0.45, 0.25, [0.0, 0.125, 0.0], STONE),
        cylinder(0.4, 0.27, [0.0, 0.135, 0.0], WATER),
        cylinder(0.08, 0.7, [0.0, 0.35, 0.0], STONE),
        cylinder(0.2, 0.08, [0.0, 0.7, 0.0], STONE),
    ]
}

pub(super) fn signpost() -> Vec<Part> {
    vec![cube([0.08, 1.1, 0.08], [0.0, 0.55, 0.0], WOOD), cube([0.6, 0.25, 0.05], [0.0, 0.95, 0.0], BOARD)]
}

/// A post with two arms, one each way.
pub(super) fn crossroads() -> Vec<Part> {
    vec![
        cube([0.08, 1.3, 0.08], [0.0, 0.65, 0.0], WOOD),
        cube([0.7, 0.18, 0.05], [0.0, 1.1, 0.0], BOARD),
        cube([0.05, 0.18, 0.7], [0.0, 0.85, 0.0], BOARD),
    ]
}

/// A stone post on the dungeon plinth, with a lamp in an iron cage open on all four sides.
pub(super) fn lamppost() -> Vec<Part> {
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
pub(super) fn clock() -> Vec<Part> {
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

/// A clock hung on a north wall (its back 0.75 north of its middle, where the wall
/// is): a round wooden case, a dial with two hands, and a short pendulum box below.
pub(super) fn wall_clock() -> Vec<Part> {
    const DIAL: [f32; 3] = [0.9, 0.85, 0.7];
    const BRASS: [f32; 3] = [0.85, 0.65, 0.25];
    let z = -0.62;
    vec![
        disc(0.16, 0.06, [0.0, 1.3, z], WOOD),
        disc(0.13, 0.02, [0.0, 1.3, z + 0.035], DIAL),
        cube([0.015, 0.09, 0.01], [0.0, 1.34, z + 0.05], INSIDE),
        cube([0.06, 0.015, 0.01], [0.03, 1.3, z + 0.05], INSIDE),
        cube([0.14, 0.26, 0.05], [0.0, 1.03, z], WOOD),
        cube([0.1, 0.2, 0.01], [0.0, 1.03, z + 0.03], INSIDE),
        cube([0.01, 0.14, 0.01], [0.0, 1.06, z + 0.04], BRASS),
        disc(0.03, 0.01, [0.0, 0.99, z + 0.045], BRASS),
    ]
}

/// An iron knob with a handle leaning `tilt_deg` toward the east (west when negative).
pub(super) fn lever(tilt_deg: f32) -> Vec<Part> {
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
pub(super) fn ladder_down() -> Vec<Part> {
    vec![
        cube([0.6, 0.02, 0.6], [0.0, 0.01, 0.0], WOOD),
        cube([0.5, 0.025, 0.5], [0.0, 0.0125, 0.0], INSIDE),
        model("platformer/Ladder_medium.gltf", [0.0, -0.95, -0.2], 0.0, 0.55),
    ]
}

/// A stone block under a wider slab, with a god's cloth over the top and down the front.
pub(super) fn altar(cloth: Option<[f32; 3]>) -> Vec<Part> {
    let mut out = vec![cube([0.7, 0.5, 0.45], [0.0, 0.25, 0.0], STONE), cube([0.8, 0.06, 0.55], [0.0, 0.53, 0.0], STONE)];
    if let Some(c) = cloth {
        out.push(cube([0.4, 0.01, 0.56], [0.0, 0.565, 0.0], c));
        out.push(cube([0.4, 0.3, 0.01], [0.0, 0.41, 0.28], c));
    }
    out
}

/// An iron bowl on three legs; lit, its fire lights round it, unlit it holds ash.
pub(super) fn brazier(lit: bool) -> Vec<Part> {
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

/// A gold ring on the floor round a cyan glow, with a gold cross over it.
pub(super) fn teleporter() -> Vec<Part> {
    vec![
        cylinder(0.42, 0.03, [0.0, 0.015, 0.0], GOLD),
        part(Shape::RoundLamp { radius: 0.34, height: 0.02 }, [0.0, 0.025, 0.0], 0.0, [1.0; 3], Some([0.3, 0.9, 1.0])),
        cube([0.06, 0.012, 0.6], [0.0, 0.04, 0.0], GOLD),
        cube([0.6, 0.012, 0.06], [0.0, 0.04, 0.0], GOLD),
    ]
}

/// A dark blue ring standing on its edge round a blue glow, facing south.
pub(super) fn magic_portal() -> Vec<Part> {
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
pub(super) fn pier() -> Vec<Part> {
    let mut out: Vec<Part> = (0..6).map(|i| cube([1.0, 0.04, 0.15], [0.0, 0.1, -0.42 + i as f32 * 0.168], WOOD)).collect();
    for x in [-0.35, 0.35] {
        out.push(cube([0.08, 0.06, 1.0], [x, 0.05, 0.0], WOOD));
        for z in [-0.4, 0.4] {
            out.push(cube([0.1, 0.4, 0.1], [x, -0.1, z], WOOD));
        }
    }
    out
}

/// A stone fireplace against a north wall: a hearth, two sides, a mantel and a fire.
pub(super) fn fireplace() -> Vec<Part> {
    let mut out = vec![
        cube([0.9, 0.06, 0.4], [0.0, 0.03, 0.0], STONE),
        cube([0.9, 1.1, 0.12], [0.0, 0.55, -0.2], STONE),
        cube([0.18, 0.7, 0.35], [-0.36, 0.35, 0.0], STONE),
        cube([0.18, 0.7, 0.35], [0.36, 0.35, 0.0], STONE),
        cube([0.95, 0.1, 0.42], [0.0, 0.75, 0.0], WOOD),
        part(Shape::Lamp([0.3, 0.04, 0.15]), [0.0, 0.08, -0.02], 0.0, [1.0; 3], Some(FLAME)),
    ];
    for (x, high) in [(-0.08, 0.3), (0.06, 0.4), (0.14, 0.22)] {
        out.push(part(Shape::Fire { radius: 0.07, height: high }, [x, 0.08 + high / 2.0, 0.0], 0.0, [1.0; 3], Some([1.0, 0.45, 0.08])));
    }
    out
}

/// A smith's forge: a stone hearth with a bed of glowing coals and flames, a back wall,
/// and a hood and chimney on it.
pub(super) fn forge() -> Vec<Part> {
    let mut out = vec![
        cube([0.8, 0.5, 0.6], [0.0, 0.25, 0.0], STONE),
        part(Shape::Lamp([0.6, 0.04, 0.4]), [0.0, 0.5, 0.0], 0.0, [1.0; 3], Some(FLAME)),
        cube([0.8, 0.12, 0.05], [0.0, 0.56, 0.28], STONE),
        cube([0.8, 0.7, 0.12], [0.0, 0.85, -0.24], STONE),
        cube([0.5, 0.35, 0.3], [0.0, 1.1, -0.1], STONE),
        cube([0.25, 0.5, 0.2], [0.0, 1.5, -0.15], STONE),
    ];
    for (x, z, high) in [(-0.15, 0.0, 0.25), (0.1, -0.05, 0.35), (0.2, 0.08, 0.2)] {
        out.push(part(Shape::Fire { radius: 0.07, height: high }, [x, 0.5 + high / 2.0, z], 0.0, [1.0; 3], Some([1.0, 0.45, 0.08])));
    }
    out
}

/// An iron cooking stove: a box on short legs with a glowing fire door and a chimney pipe.
pub(super) fn stove() -> Vec<Part> {
    let dark = [0.18, 0.18, 0.2];
    let mut out = vec![
        cube([0.6, 0.5, 0.5], [0.0, 0.35, 0.0], dark),
        part(Shape::Glow([0.2, 0.12, 0.02]), [0.0, 0.3, 0.255], 0.0, [1.0; 3], Some(FLAME)),
        cylinder(0.12, 0.03, [-0.15, 0.615, 0.0], [0.3, 0.3, 0.32]),
        cylinder(0.1, 0.03, [0.15, 0.615, 0.0], [0.3, 0.3, 0.32]),
        cylinder(0.05, 0.9, [0.2, 1.05, -0.18], dark),
    ];
    for (x, z) in [(-0.25, -0.2), (0.25, -0.2), (-0.25, 0.2), (0.25, 0.2)] {
        out.push(cube([0.05, 0.1, 0.05], [x, 0.05, z], dark));
    }
    out
}

/// A round wooden hatch in the floor: planks in an iron rim, two hinges on the north
/// side and a ring to pull on the south side.
pub(super) fn trapdoor() -> Vec<Part> {
    let mut out = vec![cylinder(0.4, 0.02, [0.0, 0.01, 0.0], IRON), cylinder(0.36, 0.03, [0.0, 0.015, 0.0], WOOD)];
    out.extend([-0.18, 0.0, 0.18].map(|x| cube([0.012, 0.031, 0.68], [x, 0.016, 0.0], DECK)));
    out.extend([-0.15, 0.15].map(|x| cube([0.08, 0.035, 0.16], [x, 0.018, -0.3], IRON)));
    out.push(Part { yaw_deg: 90.0, ..disc(0.05, 0.015, [0.0, 0.04, 0.27], IRON) });
    out
}

/// A dark red ring on the ground with a glow in it that lights round it.
pub(super) fn portal() -> Vec<Part> {
    vec![
        cylinder(0.4, 0.02, [0.0, 0.01, 0.0], [0.25, 0.04, 0.04]),
        part(Shape::RoundLamp { radius: 0.3, height: 0.02 }, [0.0, 0.025, 0.0], 0.0, [1.0; 3], Some([1.0, 0.2, 0.1])),
    ]
}

/// The KayKit torch (`scale` of it; it stands 0.4 below its middle) with a flame in its
/// cup that lights round it.
pub(super) fn torch(scale: f32, flame: [f32; 3]) -> Vec<Part> {
    vec![
        model("kaykit/torch.gltf", [0.0, 0.4 * scale, 0.0], 0.0, scale),
        part(Shape::Lamp([0.08, 0.12, 0.08]), [0.0, 1.0 * scale, 0.0], 0.0, [scale / 0.6; 3], Some(flame)),
    ]
}

/// A lamp in a little iron frame that stands on the ground; unlit, its glass is dark.
pub(super) fn lantern(lit: bool) -> Vec<Part> {
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
pub(super) fn noticeboard() -> Vec<Part> {
    vec![
        cube([0.08, 1.4, 0.08], [-0.42, 0.7, 0.0], WOOD),
        cube([0.08, 1.4, 0.08], [0.42, 0.7, 0.0], WOOD),
        cube([0.8, 0.55, 0.05], [0.0, 0.95, 0.0], BOARD),
        cube([1.0, 0.05, 0.25], [0.0, 1.42, 0.0], WOOD),
    ]
}
