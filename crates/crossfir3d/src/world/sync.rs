//! Visual sync: works out what should be on screen and adds, moves, or removes the entities.

use super::*;

/// What should be on screen, keyed by (world x, world y, layer): face, darkness, join.
type Wanted = HashMap<(i32, i32, u8), (u16, u8, u8)>;

/// Lists the picture to show on each square from the view.
fn wanted(view: &View, faces: &Faces, textures: &Textures) -> Wanted {
    let mut out = Wanted::new();
    for ((vx, vy), cell) in view.0.cells() {
        let (wx, wy) = view.0.world(vx, vy);
        for (layer, raw) in cell.layers.iter().enumerate() {
            let face = faces.0.resolve(*raw);
            if face != 0 && textures.images.contains_key(&face) {
                out.insert((wx, wy, layer as u8), (face, cell.darkness, 0));
            }
        }
    }
    out
}

/// Creatures and flying things glide between tiles; everything else snaps.
pub(super) fn glides(layer: u8) -> bool {
    layer >= 6
}

/// Where a gliding thing with a 3D model (a flying arrow) sits on square `tile`.
pub(super) fn model_spot(visual: &Visual, tile: Vec3) -> Option<Vec3> {
    match visual {
        Visual::Model { dy, .. } => Some(tile + Vec3::new(0.5, *dy, 0.5)),
        _ => None,
    }
}

/// What `sync_visuals` draws with: the command queue, the loaded models, and the asset stores.
#[derive(SystemParam)]
pub(super) struct Draw<'w, 's> {
    pub(super) commands: Commands<'w, 's>,
    pub(super) server: Res<'w, AssetServer>,
    pub(super) models: ResMut<'w, Models>,
    pub(super) materials: ResMut<'w, Assets<StandardMaterial>>,
    pub(super) images: ResMut<'w, Assets<Image>>,
    pub(super) textures: Res<'w, Textures>,
}

/// What `sync_visuals` reads: the server's view, the face names, and the own character.
#[derive(SystemParam)]
pub(super) struct Inputs<'w> {
    view: Res<'w, View>,
    faces: Res<'w, Faces>,
    view_size: Res<'w, ViewSize>,
    own: Res<'w, OwnLook>,
    map_gen: Res<'w, MapGen>,
}

type Existing<'w, 's> = Query<'w, 's, (Entity, &'static mut TileVisual, Option<&'static mut Glide>, Has<Character>)>;

/// What the whole view says about each square: walls, ground heights, the player's square.
struct Frame {
    walls: HashSet<(i32, i32)>,
    heights: HashMap<(i32, i32), f32>,
    player: (i32, i32),
}

/// One square's thing to draw, with everything the spawning helpers need to know about it.
pub(super) struct Square<'a> {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) layer: u8,
    face: u16,
    darkness: u8,
    pub(super) join: u8,
    pub(super) name: &'a str,
    pub(super) h: f32,
    pub(super) shade: f32,
    /// The square's corner at ground height, moved to the wall's open side for hung things.
    pub(super) tile: Vec3,
    pub(super) turn: Quat,
    image: Handle<Image>,
    pub(super) size: Vec2,
    pub(super) mean: Color,
    pub(super) wall: bool,
    pub(super) own_tile: bool,
    /// Big pictures are sent at their bottom-right square; models stand in the middle.
    pub(super) fw: u32,
    pub(super) fh: u32,
    pub(super) middle: Vec3,
}

impl<'a> Square<'a> {
    /// Gathers one square's tile, picture and placement from the server data and the frame.
    fn new(key:(i32, i32, u8), val: (u16, u8, u8), name: &'a str, shown: (Handle<Image>, Vec2, Color), frame: &Frame) -> Self {
        let ((x, y, layer), (face, darkness, join), (image, size, mean)) = (key, val, shown);
        let h = frame.heights.get(&(x, y)).copied().unwrap_or(0.0);
        let tile = Vec3::new(x as f32, h, y as f32);
        // A brazier or torch on a wall square stands against the wall's open side.
        // It turns to face the room (a clock's dial; torches look the same either way).
        let hung = open_side(&frame.walls, x, y).filter(|_| !is_wall(name) && hangs_on_walls(name));
        let tile = match hung {
            Some((dx, dy)) => tile + Vec3::new(dx as f32, 0.0, dy as f32) * 0.75,
            None => tile,
        };
        let (fw, fh) = footprint(size.x as u32, size.y as u32);
        Square {
            x,
            y,
            layer,
            face,
            darkness,
            join,
            name,
            h,
            shade: darkness as f32 / 255.0,
            tile,
            turn: Quat::from_rotation_y(hung.map_or(0.0, facing)),
            image,
            size,
            mean,
            wall: is_wall(name),
            // Players (a race or class picture) are 3D characters; the own one in the middle.
            own_tile: (x, y) == frame.player && classify_player_face(name).is_some(),
            fw,
            fh,
            middle: tile + Vec3::new(1.0 - fw as f32 / 2.0, 0.0, 1.0 - fh as f32 / 2.0),
        }
    }

    /// The square's tile record, used to tell whether it changed.
    pub(super) fn tv(&self) -> TileVisual {
        TileVisual { x: self.x, y: self.y, layer: self.layer, face: self.face, darkness: self.darkness, join: self.join }
    }

    /// The middle of the square, where 3D characters stand.
    pub(super) fn centre(&self) -> Vec3 {
        self.tile + Vec3::new(0.5, 0.0, 0.5)
    }
}

/// The name of a picture, or an empty name if it is not known.
fn name_of(faces: &Faces, face: u16) -> &str {
    faces.0.name(face).unwrap_or("")
}

/// The map square the player stands on, at the middle of the view.
fn player_square(input: &Inputs) -> (i32, i32) {
    input.view.0.world((input.view_size.0 / 2) as i8, (input.view_size.1 / 2) as i8)
}

/// The squares with a wall picture or a wall model.
fn wall_set(want: &Wanted, faces: &Faces, models: &Models) -> HashSet<(i32, i32)> {
    let is_wall_face = |f: &u16| is_wall(name_of(faces, *f)) || matches!(models.map.lookup(name_of(faces, *f)), Visual::Wall { .. });
    want.iter().filter(|(_, (f, _, _))| is_wall_face(f)).map(|((x, y, _), _)| (*x, *y)).collect()
}

/// The squares a building picture covers: from its bottom-right square up and left.
fn covered((x, y): (i32, i32), (w, h): (u32, u32)) -> Vec<(i32, i32)> {
    (0..h as i32).flat_map(|dy| (0..w as i32).map(move |dx| (x - dx, y - dy))).collect()
}

/// Gives wall squares their joins, and marks the squares buildings block as solid.
fn mark_solid(want: &mut Wanted, joins: &HashMap<(i32, i32), u8>, input: &Inputs, draw: &Draw, solid: &mut HashSet<(i32, i32)>) {
    let player = player_square(input);
    for ((x, y, layer), (face, _, join)) in want.iter_mut() {
        if let Some(m) = joins.get(&(*x, *y)) {
            *join = *m;
        }
        if let Visual::Building { recipe, .. } = draw.models.map.lookup(name_of(&input.faces, *face))
            && !lava_floor(recipe, *layer)
        {
            let size = draw.textures.images.get(face).map_or((1, 1), |(_, s, _)| footprint(s.x as u32, s.y as u32));
            let squares = covered((*x, *y), size);
            // The building the player stands in is hidden (`hide_entered`), so it never blocks.
            if !squares.contains(&player) {
                solid.extend(squares);
            }
        }
    }
}

/// How high each wanted square's ground is (its layer 0 picture's height).
fn ground_heights(want: &Wanted, faces: &Faces) -> HashMap<(i32, i32), f32> {
    let floor_height = |x: i32, y: i32| want.get(&(x, y, 0)).and_then(|(f, _, _)| faces.0.name(*f)).map_or(0.0, ground_height);
    want.keys().map(|&(x, y, _)| ((x, y), floor_height(x, y))).collect()
}

/// What to show now and the facts about the view; also sets which squares are solid.
fn plan(input: &Inputs, draw: &Draw, solid: &mut Solid) -> (Wanted, Frame) {
    // ponytail: full diff per change; incremental dirty cells if profiling says so.
    let mut want = wanted(&input.view, &input.faces, &draw.textures);
    let walls = wall_set(&want, &input.faces, &draw.models);
    let joins = wall_masks(&walls);
    solid.0 = walls.clone();
    mark_solid(&mut want, &joins, input, draw, &mut solid.0);
    let heights = ground_heights(&want, &input.faces);
    (want, Frame { walls, heights, player: player_square(input) })
}

/// Map memory: still things outside the server's view stay as last seen, up to
/// MEMORY_SQUARES away.
fn remembered(input: &Inputs, player: (i32, i32), new_map: bool, x: i32, y: i32) -> bool {
    let (x0, y0) = input.view.0.world(0, 0);
    let in_view = (x0..x0 + input.view_size.0 as i32).contains(&x) && (y0..y0 + input.view_size.1 as i32).contains(&y);
    !new_map && !in_view && (x - player.0).abs().max((y - player.1).abs()) <= MEMORY_SQUARES
}

/// Whether an existing entity already shows what is wanted: same face, darkness and join.
fn shows(tv: &TileVisual, (face, dark, join): (u16, u8, u8)) -> bool {
    face == tv.face && dark == tv.darkness && join == tv.join
}

/// Keeps what is unchanged (dropping it from `want`) and despawns what is gone.
/// Gliding things that left their tile are returned as orphans.
fn sweep(existing: &Existing<'_, '_>, want: &mut Wanted, commands: &mut Commands, memory: impl Fn(i32, i32) -> bool) -> Vec<Entity> {
    let mut orphans = Vec::new();
    for (entity, tv, _, _) in existing {
        match want.get(&(tv.x, tv.y, tv.layer)) {
            Some(&entry) if shows(tv, entry) => {
                want.remove(&(tv.x, tv.y, tv.layer));
            }
            _ if glides(tv.layer) => orphans.push(entity),
            None if memory(tv.x, tv.y) => {}
            _ => commands.entity(entity).despawn(),
        }
    }
    orphans
}

/// The orphan that is this square's creature one tile away (or its other frame).
/// A 3D character keeps its body when only its picture changes (each turn and step
/// frame), so it is never rebuilt, nor briefly built as someone else.
fn find_orphan(sq: &Square, orphans: &[Entity], existing: &Existing<'_, '_>, models: &Models, faces: &Faces) -> Option<usize> {
    let key = body_key(&models.map, sq.name);
    orphans.iter().position(|&e| {
        existing.get(e).is_ok_and(|(_, tv, _, character)| {
            let same_body = character && key.is_some() && body_key(&models.map, name_of(faces, tv.face)) == key;
            (tv.face == sq.face || same_body) && tv.layer == sq.layer && (tv.x - sq.x).abs() <= 1 && (tv.y - sq.y).abs() <= 1
        })
    })
}

/// Moves an old entity to this square and points its glide at the new spot.
fn retarget(sq: &Square, e: Entity, existing: &mut Existing<'_, '_>, models: &Models) {
    let Ok((_, mut tv, glide, character)) = existing.get_mut(e) else { return };
    (tv.x, tv.y, tv.darkness, tv.face) = (sq.x, sq.y, sq.darkness, sq.face);
    if let Some(mut g) = glide {
        // 3D characters and built creatures (bees) stand in the middle of the square.
        let built = matches!(models.map.lookup(sq.name), Visual::Building { .. });
        g.target = if character || built {
            sq.centre()
        } else {
            model_spot(models.map.lookup(sq.name), sq.tile).unwrap_or_else(|| card_position(sq.tile, sq.size, sq.layer))
        };
    }
}

/// A creature that stepped one tile: moves the old entity instead of making a new one.
fn reuse_orphan(sq: &Square, orphans: &mut Vec<Entity>, existing: &mut Existing<'_, '_>, models: &Models, faces: &Faces) -> bool {
    if !glides(sq.layer) {
        return false;
    }
    let Some(i) = find_orphan(sq, orphans, existing, models, faces) else { return false };
    let e = orphans.swap_remove(i);
    retarget(sq, e, existing, models);
    true
}

/// The square's picture as a material, dimmed by the darkness.
fn square_material(sq: &Square, draw: &mut Draw) -> Handle<StandardMaterial> {
    draw.materials.add(StandardMaterial {
        base_color: Color::srgb(sq.shade, sq.shade, sq.shade),
        base_color_texture: Some(sq.image.clone()),
        // Wall art has see-through parts; solid blocks must not show them as holes.
        alpha_mode: if sq.wall { AlphaMode::Opaque } else { AlphaMode::Mask(0.5) },
        perceptual_roughness: 0.9,
        double_sided: true,
        cull_mode: None,
        ..default()
    })
}

/// Makes the one new entity (or tree of entities) for a square.
fn place(sq: &Square, draw: &mut Draw, own: &OwnLook) {
    let material = square_material(sq, draw);
    if spawn_person(sq, draw, own) {
        return;
    }
    // A player never shows as the server's pixel picture, even when the 3D body fails.
    if classify_player_face(sq.name).is_some() {
        return;
    }
    if spawn_creature(sq, draw) || spawn_built(sq, draw, &material) || spawn_statue(sq, draw) {
        return;
    }
    spawn_plain(sq, draw, material);
}

/// Brings the drawn squares in line with the view, reusing or despawning old ones.
pub(super) fn sync_visuals(mut draw: Draw, input: Inputs, mut existing: Existing, mut solid: ResMut<Solid>, mut seen_gen: Local<u32>) {
    if !input.view.is_changed() && !draw.textures.is_changed() {
        return;
    }
    // A new map: forget the last one's squares (the square numbers carry on).
    let new_map = *seen_gen != input.map_gen.0;
    *seen_gen = input.map_gen.0;
    let (mut want, frame) = plan(&input, &draw, &mut solid);
    let memory = |x, y| remembered(&input, frame.player, new_map, x, y);
    let mut orphans = sweep(&existing, &mut want, &mut draw.commands, memory);
    for (key, val) in want {
        let Some(shown) = draw.textures.images.get(&val.0).cloned() else { continue };
        let sq = Square::new(key, val, name_of(&input.faces, val.0), shown, &frame);
        if !reuse_orphan(&sq, &mut orphans, &mut existing, &draw.models, &input.faces) {
            place(&sq, &mut draw, &input.own);
        }
    }
    for e in orphans {
        draw.commands.entity(e).despawn();
    }
}
/// Big images are sent at their bottom-right tile, so the card's right edge sits on
/// that tile's right edge. Higher layers sit a little nearer the camera.
pub(super) fn card_position(tile: Vec3, size: Vec2, layer: u8) -> Vec3 {
    let width = size.x / TILE_PX;
    tile + Vec3::new(1.0 - width / 2.0, 0.0, 0.5 + layer as f32 * 0.01)
}


pub(super) fn despawn_visuals(mut commands: Commands, q: Query<Entity, With<TileVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
