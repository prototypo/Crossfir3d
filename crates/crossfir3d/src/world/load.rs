//! Texture and model loading: face textures, model scenes, reveal, swap, and tinting.

use super::*;

/// The average colour of the opaque pixels of RGBA8 (sRGB) data; grey when none are.
pub(super) fn mean_colour(rgba: &[u8]) -> Color {
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for px in rgba.as_chunks::<4>().0.iter().filter(|px| px[3] >= 128) {
        for i in 0..3 {
            sum[i] += px[i] as u64;
        }
        n += 1;
    }
    if n == 0 {
        return Color::srgb(0.5, 0.5, 0.5);
    }
    let c = |i: usize| sum[i] as f32 / n as f32 / 255.0;
    Color::srgb(c(0), c(1), c(2))
}

/// Turns newly arrived face PNGs into GPU images.
pub(super) fn load_textures(
    mut faces: ResMut<Faces>,
    mut textures: ResMut<Textures>,
    mut images: ResMut<Assets<Image>>,
    session: Option<NonSend<Session>>,
) {
    for num in faces.0.take_new() {
        let Some(png) = faces.0.png(num) else { continue };
        let image = Image::from_buffer(
            png,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::nearest(),
            RenderAssetUsages::RENDER_WORLD,
        );
        match image {
            Ok(image) if usable_size(image.size()) => {
                let size = image.size().as_vec2();
                let mean = mean_colour(image.data.as_deref().unwrap_or_default());
                textures.images.insert(num, (images.add(image), size, mean));
            }
            Ok(image) => warn!("face {num}: size {} is too big, skipped", image.size()),
            Err(e) => {
                warn!("face {num}: bad PNG: {e}");
                let ask = faces.0.on_bad_png(num);
                if let Some(s) = &session {
                    s.send(ask);
                }
            }
        }
    }
}

/// Server images become GPU textures; a huge one would fail on the GPU and crash.
pub(super) fn usable_size(size: UVec2) -> bool {
    size.x > 0 && size.y > 0 && size.max_element() <= MAX_FACE_PX
}

/// Logs each model file that fails to load, once.
pub(super) fn check_models(server: Res<AssetServer>, mut models: ResMut<Models>) {
    let Models { scenes, failed, .. } = &mut *models;
    for (file, handle) in scenes.iter() {
        if failed.contains(file) {
            continue;
        }
        if let RecursiveDependencyLoadState::Failed(e) = server.recursive_dependency_load_state(handle) {
            warn!("model {file}: {e}; showing the picture instead");
            failed.insert(file.clone());
        }
    }
}

impl Models {
    fn load_state(&self, server: &AssetServer, file: &str) -> Load {
        match self.scenes.get(file) {
            _ if self.failed.contains(file) => Load::Failed,
            Some(h) if server.is_loaded_with_dependencies(h) => Load::Loaded,
            _ => Load::Loading,
        }
    }
}

/// What becomes of an assembled thing's part (or its picture) once its files are done:
/// shown (true), gone (false), or None while they load.
pub(super) fn on_load(state: Load, picture: bool) -> Option<bool> {
    match state {
        Load::Loading => None,
        Load::Loaded => Some(!picture),
        Load::Failed => Some(picture),
    }
}

/// Shows an assembled thing once all its files have loaded (its picture goes); if one
/// fails, its parts go and the picture shows.
pub(super) fn reveal_assembled(
    mut commands: Commands,
    server: Res<AssetServer>,
    models: Res<Models>,
    things: Query<(Entity, &Assembled, &Children)>,
    stand_ins: Query<(), With<StandIn>>,
) {
    for (e, Assembled(files), children) in &things {
        let state = all_loaded(files.iter().map(|f| models.load_state(&server, f)));
        if state == Load::Loading {
            continue;
        }
        for &c in children {
            match on_load(state, stand_ins.contains(c)) {
                Some(true) => {
                    commands.entity(c).try_insert(Visibility::Inherited);
                }
                Some(false) => commands.entity(c).try_despawn(),
                None => {}
            }
        }
        commands.entity(e).try_remove::<Assembled>();
    }
}

/// A loaded model takes the place of its picture card; a failed one leaves the card.
pub(super) fn swap_models(
    mut commands: Commands,
    server: Res<AssetServer>,
    models: Res<Models>,
    pending: Query<(Entity, &PendingModel)>,
) {
    for (e, p) in &pending {
        if models.failed.contains(&p.file) {
            commands.entity(e).try_remove::<PendingModel>();
        } else if server.is_loaded_with_dependencies(&p.handle) {
            commands
                .entity(e)
                .try_remove::<(PendingModel, Card, Mesh3d, MeshMaterial3d<StandardMaterial>)>()
                .try_insert((WorldAssetRoot(p.handle.clone()), p.transform, ModelTint(p.tint)));
        }
    }
}

/// What `tint_model` reads and writes: the tint and part queries and the image and material stores.
#[derive(SystemParam)]
pub(super) struct TintParams<'w, 's> {
    tints: Query<'w, 's, (&'static ModelTint, Option<&'static SkinTint>, Has<Recolour>)>,
    images: ResMut<'w, Assets<Image>>,
    part_of: Query<'w, 's, &'static PartOf>,
    statues: Query<'w, 's, (), With<StatueFigure>>,
    names: Query<'w, 's, &'static Name>,
    children: Query<'w, 's, &'static Children>,
    meshes: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

/// The tint for one mesh: the skin tint on a skin mesh, else the model's tint.
fn mesh_tint(skin: Option<&SkinTint>, model: Color, is_skin: bool) -> LinearRgba {
    match skin {
        Some(SkinTint(s)) if is_skin => s.to_linear(),
        _ => model.to_linear(),
    }
}

/// The greyed copy of a texture (made once per source image).
fn grey_texture(tex: &Handle<Image>, greys: &mut HashMap<AssetId<Image>, Handle<Image>>, images: &mut Assets<Image>) -> Handle<Image> {
    let grey = greys.entry(tex.id()).or_insert_with(|| {
        let grey = images.get(tex).map(grey_image);
        grey.map_or(tex.clone(), |g| images.add(g))
    });
    grey.clone()
}

/// Turns a material into plain stone of the given tint, with no picture on it.
fn stone_material(copy: &mut StandardMaterial, tint: LinearRgba) {
    // Darker than the tint: the plinth's own marble colour darkens it too.
    copy.base_color = LinearRgba::new(tint.red * STONE_SHADE, tint.green * STONE_SHADE, tint.blue * STONE_SHADE, 1.0).into();
    copy.base_color_texture = None;
    // Outfits can be shiny or metallic; stone is neither (it mirrored the sky).
    copy.metallic = 0.0;
    copy.metallic_roughness_texture = None;
    copy.perceptual_roughness = 0.9;
    copy.reflectance = 0.2;
    copy.emissive = LinearRgba::BLACK;
    copy.emissive_texture = None;
}

/// Multiplies a spawned model's materials by its tint (each instance gets its own copy).
/// ponytail: one material copy per instance; share copies per (material, tint) if draw calls hurt.
pub(super) fn tint_model(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    mut greys: Local<HashMap<AssetId<Image>, Handle<Image>>>,
    mut p: TintParams,
) {
    let Ok((ModelTint(tint), skin, recolour)) = p.tints.get(ready.entity) else { return };
    if *tint == Color::WHITE && skin.is_none() {
        return;
    }
    // A statue's figure is plain stone: its tint, with no picture on it.
    let stone = p.part_of.get(ready.entity).is_ok_and(|q| p.statues.contains(q.0));
    for d in p.children.iter_descendants(ready.entity) {
        let Ok(m) = p.meshes.get(d) else { continue };
        let is_skin = p.names.get(d).is_ok_and(|n| n.as_str().contains("MI_Regular"));
        let tint = mesh_tint(skin, *tint, is_skin);
        let Some(mut copy) = p.materials.get(&m.0).cloned() else { continue };
        let c = copy.base_color.to_linear();
        copy.base_color = LinearRgba::new(c.red * tint.red, c.green * tint.green, c.blue * tint.blue, c.alpha * tint.alpha).into();
        // A see-through tint (a ghost) needs blending to show.
        if tint.alpha < 1.0 {
            copy.alpha_mode = AlphaMode::Blend;
        }
        if recolour && let Some(tex) = copy.base_color_texture.clone() {
            copy.base_color_texture = Some(grey_texture(&tex, &mut greys, &mut p.images));
        }
        if stone {
            stone_material(&mut copy, tint);
        }
        commands.entity(d).try_insert(MeshMaterial3d(p.materials.add(copy)));
    }
}

/// A copy of an sRGB 8-bit picture in grey, brightened so a mid-dark colour comes out
/// near white (then the tint alone sets the colour).
fn grey_image(image: &Image) -> Image {
    let mut grey = image.clone();
    if let Some(data) = grey.data.as_mut() {
        for px in data.chunks_exact_mut(4) {
            let l = (0.3 * px[0] as f32 + 0.59 * px[1] as f32 + 0.11 * px[2] as f32) / 255.0;
            // ponytail: one fixed lift for every atlas; tune per file if one looks washed out.
            let v = ((l * 1.6).min(1.0) * 255.0) as u8;
            px[..3].fill(v);
        }
    }
    grey
}
