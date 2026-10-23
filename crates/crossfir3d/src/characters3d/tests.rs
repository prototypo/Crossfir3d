use super::*;

#[test]
fn a_hood_matches_peasant_clothes() {
    let devotee = class_look("devotee");
    let human = look_for_race("human");
    let (look, _) = class_dressed(human, Body::Regular, devotee, false);
    let t = tints(&look);
    for i in 0..3 {
        assert!((t.hair[i] / t.clothes[i] - HOOD_TO_PEASANT[i]).abs() < 1e-4, "{:?} {:?}", t.hair, t.clothes);
    }
    let ranger = class_dressed(human, Body::Regular, class_look("ranger"), false).0;
    assert_eq!(tints(&ranger).hair, tints(&ranger).clothes, "the ranger's hood is the ranger outfit's own cloth");
}

#[test]
fn ninjas_wear_the_hood_under_the_mask() {
    let ninja = class_look("ninja");
    let (_, files) = class_dressed(look_for_race("human"), Body::Regular, ninja, false);
    assert_eq!(files[1], "outfits/Male_Ranger_Head_Hood.gltf");
}

#[test]
fn weapons_point_out_of_the_fist() {
    let (at, turn) = grip(Hand::Right, &Carry::Model("weapons/Sword.gltf"));
    let blade = turn * Vec3::Y;
    assert!(blade.z > 0.5 && blade.y > 0.3, "forward and down the fingers: {blade}");
    assert!(at.y > 0.05, "in the fist, past the wrist");
    let staff = grip(Hand::Left, &Carry::Staff(crossfir3d::held::StaffTop::Plain)).1 * Vec3::Y;
    assert!(staff.y < -0.5, "the staff's top is up the arm: {staff}");
    assert!(staff.x < -0.15, "and out, away from the body (the left palm faces +X): {staff}");
}

#[test]
fn the_double_axe_handle_is_in_the_fist() {
    let (fist, _) = grip(Hand::Right, &Carry::Model("weapons/Sword.gltf"));
    let (at, turn) = grip(Hand::Right, &Carry::Model("weapons/Axe_Double.gltf"));
    // The model's handle runs up x = 0.67, not x = 0.
    let handle = at + turn * Vec3::X * 0.67 * WEAPON_SCALE;
    assert!(handle.abs_diff_eq(fist, 0.01), "{handle} {fist}");
}

#[test]
fn staff_tops_sit_on_the_staff_and_glow() {
    use crossfir3d::held::StaffTop::*;
    let c = [1.0, 0.5, 0.2];
    for top in [Cradle(c), Flame(c), Crystal(c), Hoop(c), Horns(c), Sun(c)] {
        let parts = staff_top(top);
        assert!(parts.iter().any(|p| p.2), "{top:?} glows");
        for (m, ..) in &parts {
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) = m.attribute(Mesh::ATTRIBUTE_POSITION) else { panic!() };
            for v in p {
                assert!(v[1] > -0.04 && v[1] < 0.22 && v[0].hypot(v[2]) < 0.11, "{top:?}: {v:?}");
            }
        }
    }
    assert!(staff_top(Plain).is_empty());
}

#[test]
fn shields_face_out_on_both_arms() {
    for (side, out) in [(Hand::Right, Vec3::X), (Hand::Left, Vec3::NEG_X)] {
        let (at, turn) = grip(side, &Carry::Model("weapons/Shield_Round.gltf"));
        assert!(turn.is_normalized() && (turn * Vec3::X).cross(turn * Vec3::Y).abs_diff_eq(turn * Vec3::Z, 1e-5), "a turn, not a mirror");
        assert!((turn * Vec3::Z).abs_diff_eq(out, 1e-5) && at.dot(out) > 0.0);
    }
}

#[test]
fn facing_north_looks_down_minus_z() {
    let f = |d| facing_rotation(d) * Vec3::Z;
    assert!(f(1).abs_diff_eq(Vec3::NEG_Z, 1e-5));
    assert!(f(3).abs_diff_eq(Vec3::X, 1e-5));
    assert!(f(5).abs_diff_eq(Vec3::Z, 1e-5));
}
