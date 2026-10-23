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

/// One part of a building or wall, as a child entity.
#[allow(clippy::too_many_arguments)]
/// The lava pictures are lava floors on the floor layer, and alchemy baths above it.
pub(super) fn lava_floor(recipe: &str, layer: u8) -> bool {
    layer == 0 && recipe == "alchemy_bath"
}

pub(super) fn spawn_part(
    commands: &mut Commands,
    p: &Part,
    shade: f32,
    textures: &Textures,
    sign: Option<&(Handle<StandardMaterial>, f32)>,
    server: &AssetServer,
    models: &mut Models,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    let t = Transform::from_translation(Vec3::from(p.pos))
        .with_rotation(Quat::from_rotation_y(-p.yaw_deg.to_radians()) * Quat::from_rotation_z(-p.tilt_deg.to_radians()))
        .with_scale(Vec3::from(p.scale));
    let [r, g, b] = p.color.unwrap_or([1.0; 3]);
    let color = Color::srgb(r * shade, g * shade, b * shade);
    let (mesh, size) = match p.shape {
        Shape::Model(file) => {
            let handle = models.scene(server, file);
            let mut e = commands.spawn((WorldAssetRoot(handle), t, ModelTint(color), Visibility::Hidden));
            if file.starts_with("kaykit/wall") {
                e.insert(StoneWall(t.scale.y));
            }
            return e.id();
        }
        Shape::Box(size) => (textures.cube.clone(), Vec3::from(size)),
        Shape::Bricks([x, y, z]) | Shape::Water([x, y, z]) => {
            // One scale for every face of the box: bricks by length and height, waves
            // across the top, so the pattern keeps its size on long and short boxes.
            let (image, repeats) = match p.shape {
                Shape::Bricks(_) => (&textures.bricks, Vec2::new(x.max(z) / BRICK_TILE[0], y / BRICK_TILE[1])),
                _ => (&textures.waves, Vec2::new(x, z) / WAVE_TILE),
            };
            let material = materials.add(StandardMaterial {
                base_color: color,
                base_color_texture: Some(image.clone()),
                uv_transform: bevy::math::Affine2::from_scale(repeats),
                perceptual_roughness: 0.9,
                ..default()
            });
            let t = Transform { scale: t.scale * Vec3::new(x, y, z), ..t };
            let mut part = commands.spawn((Mesh3d(textures.cube.clone()), MeshMaterial3d(material), t, Visibility::Hidden));
            if let Shape::Water(_) = p.shape {
                part.insert(Waves(repeats));
            }
            return part.id();
        }
        Shape::Cylinder { radius, height } => (textures.cylinder.clone(), Vec3::new(radius, height, radius)),
        Shape::Cone { radius, height } => (textures.cone.clone(), Vec3::new(radius, height, radius)),
        Shape::Frustum { radius, height } => (textures.frustum.clone(), Vec3::new(radius, height, radius)),
        Shape::Ball(radius) => (textures.ball.clone(), Vec3::splat(radius)),
        Shape::Wedge(size) => (textures.wedge.clone(), Vec3::from(size)),
        Shape::Lamp(_) | Shape::RoundLamp { .. } | Shape::Orb(_) => {
            let (mesh, size) = match p.shape {
                Shape::Orb(radius) => (textures.ball.clone(), Vec3::splat(radius)),
                Shape::RoundLamp { radius, height } => (textures.cylinder.clone(), Vec3::new(radius, height, radius)),
                Shape::Lamp(size) => (textures.cube.clone(), Vec3::from(size)),
                _ => unreachable!(),
            };
            let glow = materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * LAMP_GLOW,
                unlit: true,
                ..default()
            });
            let t = Transform { scale: t.scale * size, ..t };
            let light = PointLight { color, intensity: LAMP_LUMENS, range: LAMP_RANGE, shadow_maps_enabled: false, ..default() };
            // The light is a child of the scaled shape: undo the scale so its range is in squares.
            let unscaled = Transform::from_scale(Vec3::ONE / t.scale);
            return commands.spawn((Mesh3d(mesh), MeshMaterial3d(glow), t, Visibility::Hidden, NotShadowCaster)).with_child((light, unscaled)).id();
        }
        Shape::Mist(radius) => {
            let mist = materials.add(StandardMaterial {
                base_color: color.with_alpha(0.45),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                ..default()
            });
            let t = Transform { scale: t.scale * radius, ..t };
            return commands.spawn((Mesh3d(textures.ball.clone()), MeshMaterial3d(mist), t, Visibility::Hidden, NotShadowCaster)).id();
        }
        Shape::Fire { .. } | Shape::Glow(_) => {
            let glow = materials.add(StandardMaterial {
                base_color: color,
                emissive: LinearRgba::from(color) * LAMP_GLOW,
                unlit: true,
                ..default()
            });
            let (mesh, size) = match p.shape {
                Shape::Fire { radius, height } => (textures.cone.clone(), Vec3::new(radius, height, radius)),
                Shape::Glow(size) => (textures.cube.clone(), Vec3::from(size)),
                _ => unreachable!(),
            };
            let t = Transform { scale: t.scale * size, ..t };
            return commands.spawn((Mesh3d(mesh), MeshMaterial3d(glow), t, Visibility::Hidden, NotShadowCaster)).id();
        }
        Shape::Sign([w, h]) => {
            let Some((material, aspect)) = sign else { return commands.spawn((t, Visibility::Hidden)).id() };
            // Fit the text inside the board, keeping its shape, centred top to bottom.
            let (sw, sh) = if w / h > *aspect { (h * aspect, h) } else { (w, w / aspect) };
            let t = Transform { translation: t.translation + Vec3::Y * (h - sh) / 2.0, scale: t.scale * Vec3::new(sw, sh, 1.0), ..t };
            return commands.spawn((Mesh3d(textures.card.clone()), MeshMaterial3d(material.clone()), t, Visibility::Hidden)).id();
        }
    };
    let material = materials.add(StandardMaterial { base_color: color, perceptual_roughness: 0.9, ..default() });
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform { scale: t.scale * size, ..t }, Visibility::Hidden)).id()
}
