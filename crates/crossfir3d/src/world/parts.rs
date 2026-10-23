//! Building parts: the sign material and the spawner for one part of a building.

use super::*;

/// Pixel text (and icon) for a building's sign: its material and its width / height.
pub(super) fn sign_material(
    text: &str,
    icon: Option<&str>,
    shade: f32,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<StandardMaterial>, f32) {
    let (w, h, px) = crossfir3d::signs::sign_pixels(text, icon);
    let size = Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    let mut image = Image::new(size, TextureDimension::D2, px, TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    image.sampler = ImageSampler::nearest();
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(shade, shade, shade),
        base_color_texture: Some(images.add(image)),
        alpha_mode: AlphaMode::Mask(0.5),
        perceptual_roughness: 0.9,
        ..default()
    });
    (material, w as f32 / h as f32)
}

/// The lava pictures are lava floors on the floor layer, and alchemy baths above it.
pub(super) fn lava_floor(recipe: &str, layer: u8) -> bool {
    layer == 0 && recipe == "alchemy_bath"
}

/// Spawns a loaded model, tinted, hidden until assembled; kaykit walls also get a `StoneWall`.
fn spawn_model(draw: &mut Draw, file: &'static str, t: Transform, color: Color) -> Entity {
    let handle = draw.models.scene(&draw.server, file);
    let mut e = draw.commands.spawn((WorldAssetRoot(handle), t, ModelTint(color), Visibility::Hidden));
    if file.starts_with("kaykit/wall") {
        e.insert(StoneWall(t.scale.y));
    }
    e.id()
}

/// Spawns a box of bricks or water, its texture repeated to the box's size.
fn spawn_slab(draw: &mut Draw, p: &Part, [x, y, z]: [f32; 3], t: Transform, color: Color) -> Entity {
    // One scale for every face of the box: bricks by length and height, waves
    // across the top, so the pattern keeps its size on long and short boxes.
    let (image, repeats) = match p.shape {
        Shape::Bricks(_) => (&draw.textures.bricks, Vec2::new(x.max(z) / BRICK_TILE[0], y / BRICK_TILE[1])),
        _ => (&draw.textures.waves, Vec2::new(x, z) / WAVE_TILE),
    };
    let material = draw.materials.add(StandardMaterial {
        base_color: color,
        base_color_texture: Some(image.clone()),
        uv_transform: bevy::math::Affine2::from_scale(repeats),
        perceptual_roughness: 0.9,
        ..default()
    });
    let t = Transform { scale: t.scale * Vec3::new(x, y, z), ..t };
    let mut part = draw.commands.spawn((Mesh3d(draw.textures.cube.clone()), MeshMaterial3d(material), t, Visibility::Hidden));
    if let Shape::Water(_) = p.shape {
        part.insert(Waves(repeats));
    }
    part.id()
}

/// A lit, emissive material of one colour.
fn glow_material(color: Color) -> StandardMaterial {
    StandardMaterial { base_color: color, emissive: LinearRgba::from(color) * LAMP_GLOW, unlit: true, ..default() }
}

/// Spawns a lamp (box, round or orb) that glows and carries a point light.
fn spawn_lamp(draw: &mut Draw, p: &Part, t: Transform, color: Color) -> Entity {
    let (mesh, size) = match p.shape {
        Shape::Orb(radius) => (draw.textures.ball.clone(), Vec3::splat(radius)),
        Shape::RoundLamp { radius, height } => (draw.textures.cylinder.clone(), Vec3::new(radius, height, radius)),
        Shape::Lamp(size) => (draw.textures.cube.clone(), Vec3::from(size)),
        _ => unreachable!(),
    };
    let glow = draw.materials.add(glow_material(color));
    let t = Transform { scale: t.scale * size, ..t };
    let light = PointLight { color, intensity: LAMP_LUMENS, range: LAMP_RANGE, shadow_maps_enabled: false, ..default() };
    // The light is a child of the scaled shape: undo the scale so its range is in squares.
    let unscaled = Transform::from_scale(Vec3::ONE / t.scale);
    draw.commands.spawn((Mesh3d(mesh), MeshMaterial3d(glow), t, Visibility::Hidden, NotShadowCaster)).with_child((light, unscaled)).id()
}

/// Spawns a translucent ball of mist.
fn spawn_mist(draw: &mut Draw, radius: f32, t: Transform, color: Color) -> Entity {
    let mist = draw.materials.add(StandardMaterial {
        base_color: color.with_alpha(0.45),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        ..default()
    });
    let t = Transform { scale: t.scale * radius, ..t };
    draw.commands.spawn((Mesh3d(draw.textures.ball.clone()), MeshMaterial3d(mist), t, Visibility::Hidden, NotShadowCaster)).id()
}

/// Spawns a fire (cone) or a glow (box): emissive, unlit, no light.
fn spawn_glow(draw: &mut Draw, p: &Part, t: Transform, color: Color) -> Entity {
    let glow = draw.materials.add(glow_material(color));
    let (mesh, size) = match p.shape {
        Shape::Fire { radius, height } => (draw.textures.cone.clone(), Vec3::new(radius, height, radius)),
        Shape::Glow(size) => (draw.textures.cube.clone(), Vec3::from(size)),
        _ => unreachable!(),
    };
    let t = Transform { scale: t.scale * size, ..t };
    draw.commands.spawn((Mesh3d(mesh), MeshMaterial3d(glow), t, Visibility::Hidden, NotShadowCaster)).id()
}

/// Spawns a sign board showing the building's sign text; an empty marker if it has none.
fn spawn_sign(draw: &mut Draw, [w, h]: [f32; 2], t: Transform, sign: Option<&(Handle<StandardMaterial>, f32)>) -> Entity {
    let Some((material, aspect)) = sign else { return draw.commands.spawn((t, Visibility::Hidden)).id() };
    // Fit the text inside the board, keeping its shape, centred top to bottom.
    let (sw, sh) = if w / h > *aspect { (h * aspect, h) } else { (w, w / aspect) };
    let t = Transform { translation: t.translation + Vec3::Y * (h - sh) / 2.0, scale: t.scale * Vec3::new(sw, sh, 1.0), ..t };
    draw.commands.spawn((Mesh3d(draw.textures.card.clone()), MeshMaterial3d(material.clone()), t, Visibility::Hidden)).id()
}

/// The mesh and size of a plain solid shape.
fn solid(tx: &Textures, shape: &Shape) -> (Handle<Mesh>, Vec3) {
    match *shape {
        Shape::Box(size) => (tx.cube.clone(), Vec3::from(size)),
        Shape::Cylinder { radius, height } => (tx.cylinder.clone(), Vec3::new(radius, height, radius)),
        Shape::Cone { radius, height } => (tx.cone.clone(), Vec3::new(radius, height, radius)),
        Shape::Frustum { radius, height } => (tx.frustum.clone(), Vec3::new(radius, height, radius)),
        Shape::Ball(radius) => (tx.ball.clone(), Vec3::splat(radius)),
        Shape::Wedge(size) => (tx.wedge.clone(), Vec3::from(size)),
        _ => unreachable!(),
    }
}

/// One part of a building or wall, as a child entity.
pub(super) fn spawn_part(draw: &mut Draw, p: &Part, shade: f32, sign: Option<&(Handle<StandardMaterial>, f32)>) -> Entity {
    let t = Transform::from_translation(Vec3::from(p.pos))
        .with_rotation(Quat::from_rotation_y(-p.yaw_deg.to_radians()) * Quat::from_rotation_z(-p.tilt_deg.to_radians()))
        .with_scale(Vec3::from(p.scale));
    let [r, g, b] = p.color.unwrap_or([1.0; 3]);
    let color = Color::srgb(r * shade, g * shade, b * shade);
    match p.shape {
        Shape::Model(file) => return spawn_model(draw, file, t, color),
        Shape::Bricks(size) | Shape::Water(size) => return spawn_slab(draw, p, size, t, color),
        Shape::Lamp(_) | Shape::RoundLamp { .. } | Shape::Orb(_) => return spawn_lamp(draw, p, t, color),
        Shape::Mist(radius) => return spawn_mist(draw, radius, t, color),
        Shape::Fire { .. } | Shape::Glow(_) => return spawn_glow(draw, p, t, color),
        Shape::Sign(board) => return spawn_sign(draw, board, t, sign),
        // Listed, not `_`: a new shape must be placed here or the build fails.
        Shape::Box(_) | Shape::Cylinder { .. } | Shape::Cone { .. } | Shape::Frustum { .. } | Shape::Ball(_) | Shape::Wedge(_) => {}
    }
    let (mesh, size) = solid(&draw.textures, &p.shape);
    let material = draw.materials.add(StandardMaterial { base_color: color, perceptual_roughness: 0.9, ..default() });
    draw.commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform { scale: t.scale * size, ..t }, Visibility::Hidden)).id()
}
