//! Putting built headwear on a head bone: each hat family's parts, and their materials.

use super::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Finish {
    Leather,
    Cloth,
    Brass,
    Steel,
    Bone,
    /// Lit from inside: gems, glass, glowing eyes.
    Glow,
}

/// One piece of a hat: (mesh, colour, finish).
type Part = (Mesh, [f32; 3], Finish);

/// The tools `put_on` spawns with.
pub struct HatKit<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub leather: &'a Leather,
    pub meshes: &'a mut Assets<Mesh>,
    pub materials: &'a mut Assets<StandardMaterial>,
}

/// The colour darkened, for bands and trim.
fn dark([r, g, b]: [f32; 3]) -> [f32; 3] {
    [r * 0.45, g * 0.45, b * 0.45]
}

/// The soft leather pointed hat in colour `c`.
fn pointed_hat(c: [f32; 3]) -> Vec<Part> {
    vec![
        (mesh(&crown(), false, Vec2::new(3.0, 2.0)), c, Finish::Leather),
        (mesh(&brim(1.0, 0.7), true, Vec2::new(5.0, 1.0)), c, Finish::Leather),
        (mesh(&band(), false, Vec2::new(6.0, 0.25)), dark(c), Finish::Leather),
    ]
}

/// The tricorn in colour `c`, with two brass buttons.
fn tricorn_hat(c: [f32; 3]) -> Vec<Part> {
    let [crown, brim] = tricorn();
    // Brass buttons on the two front sides, near the top edge.
    let top = brim.last().expect("a tricorn brim always has rings");
    let buttons = [AROUND / 4 + AROUND / 6, AROUND / 4 - AROUND / 6].map(|j| {
        let at = top[j] - Vec3::Y * 0.015 + ring(j) * 0.004;
        (Sphere::new(0.009).mesh().build().translated_by(at), [0.85, 0.65, 0.25], Finish::Brass)
    });
    [(mesh(&crown, false, Vec2::new(3.0, 1.0)), c, Finish::Leather), (mesh(&brim, false, Vec2::new(5.0, 0.5)), c, Finish::Leather)].into_iter().chain(buttons).collect()
}

/// The warrior's steel helmet: dome, brow band, nose guard and rivets.
fn helm_hat() -> Vec<Part> {
    let [dome, band] = helm();
    let steel = [0.62, 0.64, 0.68];
    let mut out = vec![
        (mesh(&dome, false, Vec2::ONE), steel, Finish::Steel),
        (mesh(&band, false, Vec2::ONE), dark(steel), Finish::Steel),
        // The nose guard, down from the band.
        (plate(Vec3::new(0.024, 0.075, 0.012), Vec3::new(0.0, 0.09, 0.142), Quat::IDENTITY), steel, Finish::Steel),
    ];
    for j in (0..AROUND).step_by(AROUND / 8) {
        let at = Vec3::new(0.0, 0.131, -0.01) + ring(j) * Vec3::new(0.143, 0.0, 0.153);
        out.push((Sphere::new(0.006).mesh().build().translated_by(at), dark(steel), Finish::Steel));
    }
    out
}

/// A gold band with a gem.
fn circlet_hat() -> Vec<Part> {
    let gold = [0.95, 0.75, 0.3];
    vec![
        (mesh(&head_band(0.14, 0.16, 0.124, 0.137), false, Vec2::ONE), gold, Finish::Brass),
        (Sphere::new(0.016).mesh().build().translated_by(Vec3::new(0.0, 0.15, 0.133)), [0.85, 0.92, 1.0], Finish::Glow),
    ]
}

/// The priest's white mitre with a gold band.
fn mitre_hat() -> Vec<Part> {
    vec![
        (mesh(&mitre(), false, Vec2::ONE), [0.95, 0.94, 0.9], Finish::Cloth),
        (mesh(&head_band(0.15, 0.188, 0.134, 0.142), false, Vec2::ONE), [0.95, 0.75, 0.3], Finish::Brass),
    ]
}

/// The sorcerer's tall hat in colour `c`, with gold stars up the cone.
fn tall_hat(c: [f32; 3]) -> Vec<Part> {
    let cone = tall_cone();
    let gold = [0.95, 0.8, 0.35];
    let mut out = vec![
        (mesh(&cone, false, Vec2::new(3.0, 2.0)), c, Finish::Cloth),
        (mesh(&brim(0.15, 0.0), true, Vec2::new(5.0, 1.0)), c, Finish::Cloth),
        (mesh(&band(), false, Vec2::new(6.0, 0.25)), gold, Finish::Brass),
    ];
    // Stars scattered up the cone, on its outside.
    for (j, row) in [(AROUND / 4, 4), (AROUND / 4 + 7, 8), (AROUND / 4 - 6, 10), (AROUND / 4 + 2, 13), (0, 6), (AROUND / 2, 7), (3 * AROUND / 4, 9)] {
        let (p, up) = (cone[row][j], cone[row + 1][j] - cone[row][j]);
        let out_dir = ring(j);
        out.push((star(p + out_dir * 0.003, out_dir, up, 0.018 - 0.0008 * row as f32), gold, Finish::Brass));
    }
    out
}

/// The soft, wide hat for the road, in colour `c`.
fn slouch_hat(c: [f32; 3]) -> Vec<Part> {
    vec![
        (mesh(&dome(BRIM_Y - 0.01, 0.13, 0.12), false, Vec2::new(3.0, 1.0)), c, Finish::Leather),
        (mesh(&brim(0.6, 0.6), true, Vec2::new(5.0, 1.0)), c, Finish::Leather),
        (mesh(&band(), false, Vec2::new(6.0, 0.25)), dark(c), Finish::Leather),
    ]
}

/// A fur cap with a band and two bone horns.
fn horned_cap() -> Vec<Part> {
    let fur = [0.42, 0.3, 0.2];
    vec![
        (mesh(&dome(0.12, 0.132, 0.135), false, Vec2::new(3.0, 1.0)), fur, Finish::Leather),
        (mesh(&head_band(0.11, 0.14, 0.137, 0.146), false, Vec2::new(6.0, 0.3)), dark(fur), Finish::Leather),
        (mesh(&horn(1.0), false, Vec2::ONE), [0.9, 0.85, 0.7], Finish::Bone),
        (mesh(&horn(-1.0), false, Vec2::ONE), [0.9, 0.85, 0.7], Finish::Bone),
    ]
}

/// Two bone horns on a bare head.
fn bare_horns() -> Vec<Part> {
    // Sunk into a bare head (the cap's horns stand on its leather), their roots inside the skull.
    let on_head = |m: Mesh, side: f32| m.translated_by(Vec3::new(-side * 0.05, -0.07, 0.0));
    [1.0, -1.0].map(|side| (on_head(mesh(&horn(side), false, Vec2::ONE), side), [0.85, 0.8, 0.65], Finish::Bone)).into()
}

/// Two pointed cloth ears in colour `c`.
fn ears(c: [f32; 3]) -> Vec<Part> {
    [1.0f32, -1.0]
        .map(|side| {
            // Out from the side of the head, up and back to a point.
            let out = Vec3::new(side * 0.85, 0.45, -0.3).normalize();
            let ear = Cone::new(0.03, 0.11).mesh().build().scaled_by(Vec3::new(1.0, 1.0, 0.35));
            // Its base 2 cm inside the skull (0.07 across here), so no gap shows round it.
            (ear.rotated_by(Quat::from_rotation_arc(Vec3::Y, out)).translated_by(Vec3::new(side * 0.05, 0.13, -0.01) + out * 0.055), c, Finish::Cloth)
        })
        .into()
}

/// Goggles pushed up on the forehead: a strap and two brass rims with glass.
fn goggles() -> Vec<Part> {
    let mut out = vec![(mesh(&head_band(0.145, 0.165, 0.124, 0.134), false, Vec2::new(6.0, 0.2)), [0.25, 0.16, 0.1], Finish::Leather)];
    for x in [-0.036, 0.036] {
        let front = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let at = Vec3::new(x, 0.165, 0.134);
        out.push((Cylinder::new(0.026, 0.022).mesh().build().rotated_by(front).translated_by(at), [0.8, 0.6, 0.25], Finish::Brass));
        out.push((Cylinder::new(0.02, 0.003).mesh().build().rotated_by(front).translated_by(at + Vec3::Z * 0.0115), [0.3, 0.6, 0.55], Finish::Glow));
    }
    out
}

/// The parts of headwear that has no colour of its own, if built here.
fn fixed_parts(wear: Headwear) -> Option<Vec<Part>> {
    match wear {
        Headwear::Mask => Some(vec![(mesh(&mask(), false, Vec2::new(4.0, 0.3)), [0.05, 0.05, 0.06], Finish::Cloth)]),
        Headwear::Helm => Some(helm_hat()),
        Headwear::Circlet => Some(circlet_hat()),
        Headwear::Mitre => Some(mitre_hat()),
        Headwear::HornedCap => Some(horned_cap()),
        Headwear::Horns => Some(bare_horns()),
        Headwear::Goggles => Some(goggles()),
        _ => None,
    }
}

/// The parts of any headwear built here; none for the rest.
fn parts(wear: Headwear) -> Option<Vec<Part>> {
    match wear {
        Headwear::PointedHat(c) => Some(pointed_hat(c)),
        Headwear::Tricorn(c) => Some(tricorn_hat(c)),
        Headwear::TallHat(c) => Some(tall_hat(c)),
        Headwear::SlouchHat(c) => Some(slouch_hat(c)),
        Headwear::Ears(c) => Some(ears(c)),
        Headwear::GlowEyes(c) => Some([-0.032, 0.032].map(|x| (Sphere::new(0.013).mesh().build().translated_by(Vec3::new(x, 0.122, 0.097)), c, Finish::Glow)).into()),
        _ => fixed_parts(wear),
    }
}

/// How rough a finish is.
fn roughness(finish: Finish) -> f32 {
    match finish {
        Finish::Leather => 0.7,
        Finish::Cloth => 0.95,
        Finish::Brass | Finish::Steel => 0.35,
        Finish::Bone => 0.5,
        Finish::Glow => 0.2,
    }
}

/// The material of a part: its colour, with the leather pictures for leather.
fn material(finish: Finish, [r, g, b]: [f32; 3], leather: &Leather) -> StandardMaterial {
    let leathered = finish == Finish::Leather;
    StandardMaterial {
        base_color: Color::srgb(r, g, b),
        base_color_texture: leathered.then(|| leather.colour.clone()),
        normal_map_texture: leathered.then(|| leather.bumps.clone()),
        perceptual_roughness: roughness(finish),
        metallic: if matches!(finish, Finish::Brass | Finish::Steel) { 0.85 } else { 0.0 },
        emissive: if finish == Finish::Glow { LinearRgba::from(Color::srgb(r, g, b)) * 6.0 } else { LinearRgba::BLACK },
        reflectance: 0.35,
        // A turned-up brim shows its inside.
        double_sided: true,
        cull_mode: None,
        ..default()
    }
}

/// Puts a built hat or mask on a head bone, `k` times the size for a human; `marker` goes on
/// each part. Other headwear is not built here: false.
pub fn put_on(kit: HatKit, head: Entity, wear: Headwear, k: f32, marker: impl Bundle + Clone) -> bool {
    let Some(parts) = parts(wear) else { return false };
    for (m, colour, finish) in parts {
        let material = kit.materials.add(material(finish, colour, kit.leather));
        let part = (Mesh3d(kit.meshes.add(m)), MeshMaterial3d(material), Transform::from_scale(Vec3::splat(k)), marker.clone());
        kit.commands.entity(head).queue_silenced(move |mut e: EntityWorldMut| {
            e.with_child(part);
        });
    }
    true
}
