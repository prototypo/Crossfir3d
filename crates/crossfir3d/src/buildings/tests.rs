//! Tests for the building recipes and wall styles.

use super::*;

fn models(parts: &[Part]) -> Vec<&'static str> {
    parts.iter().filter_map(|p| if let Shape::Model(f) = p.shape { Some(f) } else { None }).collect()
}

#[test]
fn every_building_in_the_model_list_is_built() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models.txt")).unwrap();
    let unknown: Vec<&str> = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").split_whitespace().collect::<Vec<_>>())
        .filter(|w| w.get(1) == Some(&"building"))
        .filter_map(|w| w.get(2).copied())
        .filter(|recipe| parts(recipe, 1, 1).is_none() && parts(recipe, 2, 2).is_none())
        .collect();
    assert!(unknown.is_empty(), "drawn as flat pictures: {unknown:?}");
}

#[test]
fn a_lit_campfire_has_tongues_of_fire() {
    let fire = |lit| parts(if lit { "campfire" } else { "campfire_out" }, 1, 1).unwrap().iter().filter(|p| matches!(p.shape, Shape::Fire { .. })).count();
    assert!(fire(true) >= 3 && fire(false) == 0);
}

#[test]
fn a_wall_clock_hangs_flat_on_the_wall_behind_it() {
    let clock = parts("wallclock", 1, 1).unwrap();
    // Hung 0.75 out from the wall square's middle, its back reaches the wall.
    let back = clock.iter().map(|p| p.pos[2]).fold(0.0, f32::min);
    assert!(back < -0.5, "back {back}");
    assert!(clock.iter().all(|p| p.pos[1] > 0.8), "off the floor");
}

#[test]
fn a_cannon_points_the_way_it_faces() {
    // The barrel is the part furthest out from the middle.
    let muzzle = |recipe: &str| {
        let p = parts(recipe, 1, 1).unwrap_or_else(|| panic!("{recipe}"));
        let far = p.iter().max_by(|a, b| (a.pos[0].hypot(a.pos[2])).total_cmp(&b.pos[0].hypot(b.pos[2]))).unwrap();
        (far.pos[0], far.pos[2])
    };
    let (x, z) = muzzle("cannon_3");
    assert!(x > 0.1 && z.abs() < 0.05, "east: {x} {z}");
    let (x, z) = muzzle("cannon_1");
    assert!(z < -0.1 && x.abs() < 0.05, "north: {x} {z}");
    assert!(parts("cannon_0", 1, 1).is_some(), "idle");
}

#[test]
fn logs_lie_in_a_pile() {
    let p = parts("logs", 1, 1).unwrap();
    assert!(p.len() >= 3 && p.iter().all(|l| l.tilt_deg == 90.0));
    assert!(p.iter().any(|l| l.pos[1] > 0.1), "one on top");
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
fn clothes_take_their_colour_from_the_recipe() {
    let p = parts("cloak:0.5,0.25,1", 1, 1).unwrap();
    assert_eq!(p[0].color, Some([0.5, 0.25, 1.0]));
    assert!(parts("armour:red", 1, 1).is_none());
}

#[test]
fn burning_building_is_the_building_with_flames_over_it() {
    let house = parts("house", 2, 2).unwrap();
    let burning = parts("burning/house", 2, 2).unwrap();
    assert!(burning.len() > house.len() && burning.starts_with(&house));
    assert!(parts("burning/nothing", 1, 1).is_none());
}

#[test]
fn tilted_parts_lean_east_round_their_foot() {
    let p = tilted(vec![cube([0.1; 3], [0.0, 1.0, 0.0], WOOD)], 90.0).remove(0);
    assert!((p.pos[0] - 1.0).abs() < 1e-6 && p.pos[1].abs() < 1e-6, "{:?}", p.pos);
    assert_eq!(p.tilt_deg, 90.0);
}

#[test]
fn limb_runs_from_one_point_to_the_other() {
    let p = limb([0.0, 0.0, 0.0], [0.0, 0.0, 2.0], 0.1, WOOD);
    assert_eq!((p.pos, p.yaw_deg, p.shape), ([0.0, 0.0, 1.0], 90.0, Shape::Box([2.0, 0.1, 0.1])));
    let up = limb([0.0, 0.0, 0.0], [1.0, 1.0, 0.0], 0.1, WOOD);
    assert!((up.tilt_deg + 45.0).abs() < 1e-4, "{}", up.tilt_deg);
}

#[test]
fn items_lie_small_on_their_square() {
    for kind in ITEM_KINDS {
        let p = parts(&format!("{kind}:0.5,0.6,0.7"), 1, 1).unwrap_or_else(|| panic!("no recipe {kind}"));
        assert!(!p.is_empty(), "{kind} is empty");
        assert!(p.iter().any(|x| x.color == Some([0.5, 0.6, 0.7])), "{kind} does not use its colour");
        for x in &p {
            assert!(x.pos[1] < 0.35, "{kind} stands too high");
            assert!(x.pos[0].abs() < 0.5 && x.pos[2].abs() < 0.5, "{kind} leaves its square");
        }
    }
}

#[test]
fn spikes_rise_out_of_the_ground() {
    let top = |p: &[Part]| p.iter().map(|p| p.pos[1]).fold(f32::MIN, f32::max);
    assert!(top(&parts("spike_B", 1, 1).unwrap()) > 0.4, "the middle of a cone 0.9 high");
    assert_eq!(parts("spike_0", 1, 1).unwrap().len(), 1, "only the hole");
    assert!(top(&parts("spikes_4", 1, 1).unwrap()) < top(&parts("spikes_8", 1, 1).unwrap()));
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
