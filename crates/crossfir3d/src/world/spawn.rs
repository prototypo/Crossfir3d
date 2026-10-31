//! Per-kind spawners: people, creatures, buildings, statues, cards, and ground.

use super::*;

/// A player's class look: the own one's from the login, another's from a class picture.
fn class_of(sq: &Square, map: &ModelMap, own: &OwnLook) -> Option<ClassLook> {
    match (classify_player_face(sq.name), map.lookup(sq.name)) {
        _ if sq.own_tile => own.class,
        (Some(PlayerFace::Class(arch)), _) => class_look(arch),
        (None, Visual::Person { class, .. }) => *class,
        _ => None,
    }
}

/// The look and body of a player or a `person` creature, and a creature's outfit.
fn person_of(sq: &Square, map: &ModelMap, own: &OwnLook) -> Option<(Look, Body, Option<Outfit>)> {
    match (classify_player_face(sq.name), map.lookup(sq.name)) {
        _ if sq.own_tile => Some((own.look, own.body, None)),
        (Some(kind), _) => Some((other_look(kind), Body::Regular, None)),
        (None, &Visual::Person { body, look, dress, broom, armour, wide, head, build, horns, ears, .. }) => {
            // Pointed ears are the skin's colour (its tint over a human's skin).
            let ears = ears.then(|| [0.78 * look.skin[0], 0.56 * look.skin[1], 0.42 * look.skin[2]]);
            Some((look, body, Some(Outfit { scale: look.scale, dress, broom, armour, wide, head, build, horns, ears })))
        }
        _ => None,
    }
}

/// The three body files of a person: the class's outfit, a bare head under horns or
/// pointed ears, and metal armour.
fn person_files(look: Look, body: Body, class: Option<ClassLook>, outfit: Option<Outfit>) -> (Look, [&'static str; 3]) {
    let (look, mut files) = class_dressed(look, body, class, outfit.is_some());
    // Horns and pointed ears go on a bare head.
    if outfit.is_some_and(|o| o.horns || o.ears.is_some()) {
        files[1] = "";
    }
    if outfit.is_some_and(|o| o.armour) {
        files[2] = outfit_file(body, Garb::Metal);
    }
    (look, files)
}

/// The robe or rags a person wears, if any.
fn dress_of(sq: &Square, map: &ModelMap, class: Option<ClassLook>, armour: bool) -> Option<crate::robe::Dress> {
    let rags = match map.lookup(sq.name) {
        Visual::Person { rags, .. } => *rags,
        _ => None,
    };
    crate::robe::Dress::of(class, armour)
        .map(|d| crate::robe::Dress { rags, ..d })
        .or(rags.map(|_| crate::robe::Dress { robe: false, back: None, over: None, rags }))
}

/// The own character is marked as such; another's class look is kept for its gear.
fn mark_owner_or_class(sq: &Square, draw: &mut Draw, e: Entity, class: Option<ClassLook>, scale: f32) {
    if sq.own_tile {
        draw.commands.entity(e).insert(OwnCharacter);
    } else if let Some(class) = class {
        draw.commands.entity(e).insert(ClassGear(class, scale));
    }
}

/// The body of a player whose race is not human-shaped.
fn player_beast(sq: &Square, own: &OwnLook) -> Option<BeastBody> {
    match classify_player_face(sq.name) {
        _ if sq.own_tile => own.beast,
        Some(PlayerFace::Race(race)) => beast_body(race),
        _ => None,
    }
}

/// Spawns a player whose race is not human-shaped; false if none, or its model failed.
fn spawn_player_beast(sq: &Square, draw: &mut Draw, own: &OwnLook) -> bool {
    let Some(b) = player_beast(sq, own).filter(|b| !draw.models.failed.contains(b.file)) else { return false };
    let pos = sq.centre();
    let scene = draw.models.scene(&draw.server, b.file);
    let look = BeastLook { scene, file: b.file, scale: b.scale, tint: b.tint, alpha: b.alpha, parts: Visibility::Hidden };
    let extra = (sq.tv(), Glide { target: pos }, Assembled(vec![b.file.to_string()]));
    let e = spawn_beast(&mut draw.commands, look, Transform::from_translation(pos), extra);
    if sq.own_tile {
        draw.commands.entity(e).insert(OwnCharacter);
    }
    true
}

/// Spawns a gliding player or `person` creature as a 3D character; false if it is none
/// or one of its body files failed to load.
pub(super) fn spawn_person(sq: &Square, draw: &mut Draw, own: &OwnLook) -> bool {
    if !glides(sq.layer) {
        return false;
    }
    if spawn_player_beast(sq, draw, own) {
        return true;
    }
    let Some((look, body, outfit)) = person_of(sq, &draw.models.map, own) else { return false };
    let class = class_of(sq, &draw.models.map, own);
    let (look, files) = person_files(look, body, class, outfit);
    if files.iter().any(|f| draw.models.failed.contains(*f)) {
        return false;
    }
    let pos = sq.centre();
    let load = |f: &str| draw.models.scene(&draw.server, f);
    let at = Transform::from_translation(pos);
    let e = spawn_character(&mut draw.commands, load, files, &look, Placement { at, parts: Visibility::Hidden, extra: (sq.tv(), Glide { target: pos }) });
    draw.commands.entity(e).insert(Assembled(files.iter().filter(|f| !f.is_empty()).map(|f| f.to_string()).collect()));
    if let Some(dress) = dress_of(sq, &draw.models.map, class, outfit.is_some_and(|o| o.armour)) {
        draw.commands.entity(e).insert(dress);
    }
    mark_owner_or_class(sq, draw, e, class, look.scale);
    if let Some(outfit) = outfit {
        draw.commands.entity(e).insert(outfit);
    }
    true
}

/// Spawns a gliding beast, monster or skeleton as a 3D creature in the middle of its
/// square; false if the square holds none (or its model failed to load).
pub(super) fn spawn_creature(sq: &Square, draw: &mut Draw) -> bool {
    if !glides(sq.layer) {
        return false;
    }
    let pos = sq.centre();
    let at = Transform::from_translation(pos);
    let extra = (sq.tv(), Glide { target: pos });
    match draw.models.map.lookup(sq.name) {
        Visual::Beast { file, scale, tint } if !draw.models.failed.contains(file) => {
            let (file, scale, tint) = (file.clone(), *scale, tint.unwrap_or([1.0; 3]));
            let scene = draw.models.scene(&draw.server, &file);
            spawn_beast(&mut draw.commands, BeastLook { scene, file: &file, scale, tint, alpha: 1.0, parts: Visibility::Hidden }, at, (extra, Assembled(vec![file.clone()])));
        }
        Visual::Monster { file, scale, tint } if !draw.models.failed.contains(file) => {
            let (file, scale, tint) = (file.clone(), *scale, tint.unwrap_or([1.0; 3]));
            let scene = draw.models.scene(&draw.server, &file);
            spawn_monster(&mut draw.commands, scene, scale, tint, Placement { at, parts: Visibility::Hidden, extra: (extra, Assembled(vec![file])) });
        }
        &Visual::Skeleton { scale, tint } => {
            let file = character_files(Body::Regular)[0];
            let look = Look { scale, ..look_for_race("human") };
            let load = |f: &str| draw.models.scene(&draw.server, f);
            let bony = crate::skeleton::Bony(tint.unwrap_or([1.0; 3]));
            let extra = (extra, Assembled(vec![file.to_string()]), bony);
            spawn_character(&mut draw.commands, load, [file, "", ""], &look, Placement { at, parts: Visibility::Hidden, extra });
        }
        _ => return false,
    }
    true
}

/// A building's sign: its material and its width / height.
type Sign = (Handle<StandardMaterial>, f32);

/// The parts of a wall or building, and a building's sign.
fn built_parts(sq: &Square, draw: &mut Draw) -> (Option<Vec<Part>>, Option<Sign>) {
    match draw.models.map.lookup(sq.name) {
        Visual::Wall { style } => (Some(wall_parts(style, sq.join)).filter(|p| !p.is_empty()), None),
        Visual::Building { recipe, .. } if lava_floor(recipe, sq.layer) => (None, None),
        Visual::Building { recipe, sign, icon } => {
            let sign = sign.as_deref().map(|t| sign_material(t, icon.as_deref(), sq.shade, &mut draw.images, &mut draw.materials));
            (parts(recipe, sq.fw, sq.fh), sign)
        }
        _ => (None, None),
    }
}

/// The picture (a block for walls, else a card), hidden while a 3D thing loads,
/// shows if it fails: never nothing.
fn stand_in(sq: &Square, draw: &mut Draw, material: &Handle<StandardMaterial>, parent: Entity, at: Vec3) {
    let picture = if sq.wall {
        Transform::from_translation(sq.tile + Vec3::new(0.5, 0.5, 0.5))
    } else {
        Transform::from_translation(card_position(sq.tile, sq.size, sq.layer)).with_scale(Vec3::new(sq.size.x / TILE_PX, sq.size.y / TILE_PX, 1.0))
    };
    let t = Transform { translation: picture.translation - at, ..picture };
    let mesh = if sq.wall { draw.textures.cube.clone() } else { draw.textures.card.clone() };
    let mut e = draw.commands.spawn((StandIn, Mesh3d(mesh), MeshMaterial3d(material.clone()), t, Visibility::Hidden, ChildOf(parent)));
    if !sq.wall {
        e.insert(Card);
    }
}

/// Spawns the root of a wall or building: its span, the files it needs, a glide for
/// creatures and a door for houses.
fn spawn_building_root(sq: &Square, draw: &mut Draw, parts: &[Part], at: Vec3) -> Entity {
    let span = Span(sq.x + 1 - sq.fw as i32, sq.y + 1 - sq.fh as i32, sq.x, sq.y);
    let files: Vec<String> = parts.iter().filter_map(|p| if let Shape::Model(f) = p.shape { Some(f.to_string()) } else { None }).collect();
    let parent = draw.commands.spawn((sq.tv(), span, Assembled(files), Transform::from_translation(at).with_rotation(sq.turn), Visibility::default())).id();
    if glides(sq.layer) {
        draw.commands.entity(parent).insert(Glide { target: at });
    }
    if let Visual::Building { recipe, .. } = draw.models.map.lookup(sq.name)
        && let Some(step) = doorstep(recipe, sq.fw, sq.fh)
    {
        draw.commands.entity(parent).insert(Door(at + Vec3::from(step)));
    }
    parent
}

/// Spawns a wall or building from its parts; false if the square holds none.
pub(super) fn spawn_built(sq: &Square, draw: &mut Draw, material: &Handle<StandardMaterial>) -> bool {
    let (built, sign) = built_parts(sq, draw);
    let Some(parts) = built else { return false };
    let at = if sq.layer == 0 || sq.wall { sq.centre() } else { sq.middle };
    let parent = spawn_building_root(sq, draw, &parts, at);
    for p in &parts {
        let child = spawn_part(draw, p, sq.shade, sign.as_ref());
        draw.commands.entity(parent).add_child(child);
    }
    stand_in(sq, draw, material, parent, at);
    true
}

/// Spawns a statue: a plinth with a stone figure on top; false if the square holds none.
pub(super) fn spawn_statue(sq: &Square, draw: &mut Draw) -> bool {
    let Visual::Statue { tint } = draw.models.map.lookup(sq.name) else { return false };
    let [r, g, b] = tint.unwrap_or(STATUE_STONE);
    let stone = Color::srgb(r * sq.shade, g * sq.shade, b * sq.shade);
    let parent = draw.commands.spawn((sq.tv(), Transform::from_translation(sq.centre()), Visibility::default())).id();
    let plinth = draw.models.scene(&draw.server, PLINTH);
    draw.commands.spawn((WorldAssetRoot(plinth), Transform::from_scale(Vec3::splat(PLINTH_SCALE)), ModelTint(stone), ChildOf(parent)));
    // Body colours are tints relative to a human (`races::tints`): undo that, so
    // every part comes out the same stone.
    let human = look_for_race("human");
    let rel = |h: [f32; 3]| [r * h[0], g * h[1], b * h[2]];
    let look = Look { scale: 0.5, skin: rel(human.skin), hair: [r, g, b], clothes: rel(human.clothes) };
    let load = |f: &str| draw.models.scene(&draw.server, f);
    let on_top = Transform::from_xyz(0.0, PLINTH_TOP, 0.0);
    let figure = spawn_character(&mut draw.commands, load, character_files(Body::Regular), &look, Placement { at: on_top, parts: Visibility::Inherited, extra: StatueFigure });
    draw.commands.entity(figure).insert(ChildOf(parent));
    true
}

/// A square's 3D model (file, placement, tint), if it has one that has not failed.
fn scene_model(sq: &Square, models: &Models) -> Option<(String, Transform, Color)> {
    match models.map.lookup(sq.name) {
        Visual::Model { file, scale, rot_deg, dy, tint } if !models.failed.contains(file) => {
            let t = Transform::from_xyz(0.0, *dy, 0.0).with_rotation(Quat::from_rotation_y(-rot_deg.to_radians())).with_scale(Vec3::splat(*scale));
            let [r, g, b] = tint.unwrap_or([1.0; 3]);
            Some((file.clone(), t, Color::srgb(r * sq.shade, g * sq.shade, b * sq.shade)))
        }
        _ => None,
    }
}

/// A block under a ground square, reaching down below the lowest ground.
/// Raised and sunken squares (hills, sea) need sides, or they float over a gap.
/// Under a 3D model (mountains, hills) the sides are the picture's average
/// colour: the stretched pixels did not blend into the model.
fn add_ground_side(sq: &Square, draw: &mut Draw, ground: Entity, material: Handle<StandardMaterial>, has_model: bool) {
    let depth = sq.h - GROUND_BOTTOM;
    let side = Transform::from_xyz(0.0, -depth / 2.0 - 0.002, 0.0).with_scale(Vec3::new(1.0, depth, 1.0));
    let side_material = if has_model {
        let c = sq.mean.to_srgba();
        draw.materials.add(StandardMaterial {
            base_color: Color::srgb(c.red * sq.shade, c.green * sq.shade, c.blue * sq.shade),
            perceptual_roughness: 0.9,
            ..default()
        })
    } else {
        material
    };
    draw.commands.entity(ground).with_child((Mesh3d(draw.textures.cube.clone()), MeshMaterial3d(side_material), side));
}

/// A ground square (or a flat decoration just above it), with its side block and,
/// for trees and rocks, its model standing on the ground tile.
fn spawn_ground(sq: &Square, draw: &mut Draw, material: Handle<StandardMaterial>, flat: bool) {
    let model = scene_model(sq, &draw.models);
    // Flat decorations lie just above the ground, as big as their picture.
    let (pos, scale) = if flat {
        (sq.middle + Vec3::Y * (0.01 + sq.layer as f32 * 0.001), Vec3::new(sq.fw as f32, 1.0, sq.fh as f32))
    } else {
        (sq.centre(), Vec3::ONE)
    };
    let t = Transform::from_translation(pos).with_scale(scale);
    let ground = draw.commands.spawn((sq.tv(), Mesh3d(draw.textures.ground.clone()), MeshMaterial3d(material.clone()), t)).id();
    if !flat {
        add_ground_side(sq, draw, ground, material, model.is_some());
    }
    if let (Some((file, t, tint)), false) = (model, flat) {
        let handle = draw.models.scene(&draw.server, &file);
        draw.commands.entity(ground).with_child((WorldAssetRoot(handle), t, ModelTint(tint)));
    }
}

/// The developer's compare row: models beside the card. Children turn with the card,
/// so the row always faces the camera.
fn add_compare_row(sq: &Square, draw: &mut Draw, card: Entity, scale: Vec3) {
    let base = sq.name.split('.').next().unwrap_or(sq.name);
    for (i, file) in draw.models.compare.get(base).cloned().unwrap_or_default().iter().enumerate() {
        let at = Vec3::new(COMPARE_GAP * (i + 1) as f32, 0.0, 0.0) / scale;
        let turn = Quat::from_rotation_y(COMPARE_TURN_DEG.to_radians());
        let scene = draw.models.scene(&draw.server, file);
        draw.commands.entity(card).with_child((WorldAssetRoot(scene), Transform::from_translation(at).with_rotation(turn).with_scale(1.0 / scale)));
    }
}

/// Gives a card the 3D model it waits for: glides to the model's spot, or stands in the middle.
fn add_pending_model(sq: &Square, draw: &mut Draw, card: Entity, pos: Vec3, model: Option<(String, Transform, Color)>) {
    if glides(sq.layer) {
        match (model, model_spot(draw.models.map.lookup(sq.name), sq.tile)) {
            (Some((file, t, tint)), Some(spot)) => {
                let handle = draw.models.scene(&draw.server, &file);
                let transform = Transform { translation: spot, ..t };
                draw.commands.entity(card).insert((Glide { target: spot }, PendingModel { file, handle, transform, tint }));
            }
            _ => {
                draw.commands.entity(card).insert(Glide { target: pos });
            }
        }
    } else if let Some((file, t, tint)) = model {
        let handle = draw.models.scene(&draw.server, &file);
        let transform = Transform { translation: sq.middle + t.translation, ..t };
        draw.commands.entity(card).insert(PendingModel { file, handle, transform, tint });
    }
}

/// An upright picture card, with its compare row and 3D model.
fn spawn_card(sq: &Square, draw: &mut Draw, material: Handle<StandardMaterial>) {
    let model = scene_model(sq, &draw.models);
    let pos = card_position(sq.tile, sq.size, sq.layer);
    let t = Transform::from_translation(pos).with_scale(Vec3::new(sq.size.x / TILE_PX, sq.size.y / TILE_PX, 1.0));
    let card = draw.commands.spawn((sq.tv(), Card, Mesh3d(draw.textures.card.clone()), MeshMaterial3d(material), t)).id();
    add_compare_row(sq, draw, card, t.scale);
    add_pending_model(sq, draw, card, pos, model);
}

/// Spawns what is left once no 3D character, creature, building or statue fits: a wall
/// block, a ground square, or a card.
pub(super) fn spawn_plain(sq: &Square, draw: &mut Draw, material: Handle<StandardMaterial>) {
    let flat = matches!(draw.models.map.lookup(sq.name), Visual::Flat);
    if sq.wall {
        let t = Transform::from_translation(sq.tile + Vec3::new(0.5, 0.5, 0.5));
        draw.commands.spawn((sq.tv(), Mesh3d(draw.textures.cube.clone()), MeshMaterial3d(material), t));
    } else if sq.layer == 0 || flat {
        spawn_ground(sq, draw, material, flat);
    } else {
        spawn_card(sq, draw, material);
    }
}

/// Pictures that share a 3D body: a player's pictures of one race or class, or one
/// creature's frames.
pub(super) fn body_key<'a>(map: &ModelMap, name: &'a str) -> Option<&'a str> {
    match (classify_player_face(name), map.lookup(name)) {
        (Some(PlayerFace::Race(arch) | PlayerFace::Class(arch)), _) => Some(arch),
        (None, Visual::Person { .. } | Visual::Monster { .. } | Visual::Beast { .. }) => name.split('.').next(),
        _ => None,
    }
}

/// Another player's look: their race if the picture shows it, else a human in their
/// class colours.
fn other_look(kind: PlayerFace) -> Look {
    match kind {
        PlayerFace::Race(race) => look_for_race(race),
        PlayerFace::Class(class) => Look { clothes: clothes_for_class(class), ..look_for_race("human") },
    }
}
