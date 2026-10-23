//! Applies the video settings to the window, camera, and lights; caps the frame rate;
//! remembers where the window was.

use std::time::{Duration, Instant};

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::RenderTarget;
use bevy::render::camera::{MipBias, TemporalJitter};
use bevy::render::render_resource::{Extent3d, TextureFormat};
use bevy_egui::PrimaryEguiContext;
use bevy::light::DirectionalLightShadowMap;
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
use bevy::window::{Monitor, MonitorSelection, PrimaryMonitor, PresentMode, PrimaryWindow, WindowMode, WindowMoved, WindowPosition, WindowResized};
use crossfir3d::settings::{
    default_settings_path, load_settings, save_settings, to_logical, to_main_screen_physical, AntiAlias, FrameRate,
    Quality, Settings, WindowPlace,
};


#[derive(Resource)]
pub struct CurrentSettings(pub Settings);

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scaled_view)
            .add_systems(
                Update,
                (apply_settings, fit_scene_image, keep_window_on_screen, remember_window, save_settings_later),
            )
            .add_systems(Last, (save_on_exit, frame_cap));
    }
}

/// The primary window as the saved settings describe it (used when building the app).
pub fn initial_window(s: &Settings) -> Window {
    let mut w = Window { title: "Crossfir3d".into(), ..default() };
    w.mode = window_mode(s);
    w.present_mode = present_mode(s.frame_rate);
    // The position is set once the monitors are known (`place_window`).
    if let Some(p) = s.window {
        w.resolution.set(p.width as f32, p.height as f32);
    }
    w
}

/// Loads the settings file once at start.
pub fn startup_settings() -> CurrentSettings {
    CurrentSettings(load_settings(&default_settings_path()))
}

fn window_mode(s: &Settings) -> WindowMode {
    if s.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed }
}

fn present_mode(f: FrameRate) -> PresentMode {
    if f == FrameRate::MatchScreen { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync }
}

fn ssao_level(q: Quality) -> Option<ScreenSpaceAmbientOcclusionQualityLevel> {
    match q {
        Quality::Off => None,
        Quality::Low => Some(ScreenSpaceAmbientOcclusionQualityLevel::Low),
        Quality::Medium => Some(ScreenSpaceAmbientOcclusionQualityLevel::Medium),
        Quality::High => Some(ScreenSpaceAmbientOcclusionQualityLevel::High),
    }
}

/// Everything except the window place, which changes on every move and needs no re-apply,
/// and music, which `audio.rs` handles.
fn look(s: &Settings) -> Settings {
    Settings { window: None, music: true, ..s.clone() }.normalized()
}

/// Applies changed settings to the window and the cameras.
#[allow(clippy::too_many_arguments)]
fn apply_settings(
    mut commands: Commands,
    settings: Res<CurrentSettings>,
    mut applied: Local<Option<Settings>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    cameras: Query<Entity, With<Camera3d>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
) {
    let s = look(&settings.0);
    // Cameras appear after startup; keep trying until there is one.
    if applied.as_ref() == Some(&s) && !cameras.is_empty() {
        return;
    }
    window.mode = window_mode(&s);
    window.present_mode = present_mode(s.frame_rate);
    for cam in &cameras {
        let mut e = commands.entity(cam);
        match ssao_level(s.ambient_occlusion) {
            Some(quality_level) => e.insert(ScreenSpaceAmbientOcclusion { quality_level, ..default() }),
            None => e.remove::<ScreenSpaceAmbientOcclusion>(),
        };
        match s.anti_alias {
            // TAA brought its jitter and mip bias with it: they go too, or textures shimmer.
            AntiAlias::Off => e.insert(Msaa::Off).remove::<(TemporalAntiAliasing, TemporalJitter, MipBias)>(),
            AntiAlias::Msaa4 => e.insert(Msaa::Sample4).remove::<(TemporalAntiAliasing, TemporalJitter, MipBias)>(),
            AntiAlias::Taa => e.insert((Msaa::Off, TemporalAntiAliasing::default())),
        };
        let section = ColorGradingSection { contrast: s.contrast, gamma: s.gamma, ..default() };
        e.insert(ColorGrading {
            global: ColorGradingGlobal { exposure: s.brightness, ..default() },
            shadows: section,
            midtones: section,
            highlights: section,
        });
    }
    shadow_map.size = match s.shadows {
        Quality::Off | Quality::Low => 1024,
        Quality::Medium => 2048,
        Quality::High => 4096,
    };
    if !cameras.is_empty() {
        *applied = Some(s);
    }
}

/// Puts the window where it was saved (logical points), converted with the main
/// screen's scale as winit expects. A place on a monitor that is gone: main screen.
fn keep_window_on_screen(
    mut done: Local<bool>,
    settings: Res<CurrentSettings>,
    monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if *done || monitors.is_empty() {
        return;
    }
    *done = true;
    let Some(place) = settings.0.window else { return };
    let (x, y) = (place.x as f64, place.y as f64);
    let visible = monitors.iter().any(|(m, _)| {
        let s = m.scale_factor;
        let (mx, my) = (m.physical_position.x as f64 / s, m.physical_position.y as f64 / s);
        let (mw, mh) = (m.physical_width as f64 / s, m.physical_height as f64 / s);
        x >= mx && y >= my && x < mx + mw && y < my + mh
    });
    let main_scale = monitors.iter().find(|(_, primary)| *primary).map_or(1.0, |(m, _)| m.scale_factor);
    window.position = if visible {
        let (px, py) = to_main_screen_physical((place.x, place.y), main_scale);
        WindowPosition::At(IVec2::new(px, py))
    } else {
        WindowPosition::Centered(MonitorSelection::Primary)
    };
}

/// Saves the windowed position and size into the settings when the window moves.
fn remember_window(
    mut moved: MessageReader<WindowMoved>,
    mut resized: MessageReader<WindowResized>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut last_pos: Local<Option<IVec2>>,
    mut settings: ResMut<CurrentSettings>,
    monitors: Query<&Monitor>,
) {
    if let Some(m) = moved.read().last() {
        *last_pos = Some(m.position);
    }
    let changed = last_pos.is_some() | (resized.read().count() > 0);
    if !changed || settings.0.fullscreen || window.mode != WindowMode::Windowed {
        return;
    }
    let p = match (*last_pos, window.position) {
        (Some(p), _) | (None, WindowPosition::At(p)) => p,
        _ => return,
    };
    // Saved in logical points: the screen the window is on may have another density.
    let (x, y) = to_logical((p.x, p.y), window.scale_factor() as f64);
    let place = WindowPlace { x, y, width: window.width() as u32, height: window.height() as u32 };
    // macOS's green-button full screen keeps the mode Windowed: do not save it as a place.
    let screens: Vec<(f32, f32)> =
        monitors.iter().map(|m| (m.physical_width as f32 / m.scale_factor as f32, m.physical_height as f32 / m.scale_factor as f32)).collect();
    if fills_a_screen(place, &screens) {
        return;
    }
    if settings.0.window != Some(place) {
        settings.0.window = Some(place);
    }
}

/// A window as big as one of the screens (logical points): full screen, not a place.
fn fills_a_screen(place: WindowPlace, screens: &[(f32, f32)]) -> bool {
    screens.iter().any(|&(w, h)| place.width as f32 >= w - 1.0 && place.height as f32 >= h - 40.0)
}

/// Saves at most once a second (window moves send many changes).
fn save_settings_later(settings: Res<CurrentSettings>, mut dirty: Local<bool>, mut last: Local<Option<Instant>>) {
    if settings.is_changed() && !settings.is_added() {
        *dirty = true;
    }
    let due = last.is_none_or(|t| t.elapsed() >= Duration::from_secs(1));
    if *dirty && due {
        if let Err(e) = save_settings(&default_settings_path(), &settings.0) {
            warn!("could not save settings: {e}");
        }
        *dirty = false;
        *last = Some(Instant::now());
    }
}

/// Quitting within a second of a change would lose it (saves wait a second): save now.
fn save_on_exit(mut exits: MessageReader<AppExit>, settings: Res<CurrentSettings>) {
    if exits.read().next().is_some()
        && let Err(e) = save_settings(&default_settings_path(), &settings.0)
    {
        warn!("could not save settings: {e}");
    }
}

/// ponytail: sleep-based cap; a frame-pacing crate if it judders.
fn frame_cap(settings: Res<CurrentSettings>, mut last: Local<Option<Instant>>) {
    let fps = match settings.0.frame_rate {
        FrameRate::Cap30 => 30.0,
        FrameRate::Cap60 => 60.0,
        FrameRate::Cap120 => 120.0,
        FrameRate::MatchScreen | FrameRate::Unlimited => {
            *last = None;
            return;
        }
    };
    let frame = Duration::from_secs_f64(1.0 / fps);
    if let Some(t) = *last {
        if let Some(rest) = frame.checked_sub(t.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    *last = Some(Instant::now());
}

/// The 3D view is drawn into this image (window size x render scale); the window camera
/// stretches it over the window and draws egui on top at full sharpness.
#[derive(Resource)]
pub struct SceneImage(pub Handle<Image>);

/// The scene image: drawn on by the GPU only, so it keeps no pixels in memory (a 5K
/// window would hold ~59 MB of zeros, reallocated while resizing).
fn scene_target() -> Image {
    let mut image = Image::new_target_texture(1, 1, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb));
    image.data = None;
    image
}

fn setup_scaled_view(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(scene_target());
    commands.insert_resource(SceneImage(handle.clone()));
    commands.spawn((Camera2d, Camera { order: 1, ..default() }, PrimaryEguiContext));
    commands.spawn((
        Node { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, ..default() },
        ImageNode::new(handle),
        GlobalZIndex(-1),
    ));
}

/// Points the 3D camera at the scene image and keeps the image at window size x scale.
fn fit_scene_image(
    mut commands: Commands,
    scene: Res<SceneImage>,
    settings: Res<CurrentSettings>,
    window: Single<&Window, With<PrimaryWindow>>,
    cameras: Query<(Entity, Option<&RenderTarget>), With<Camera3d>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (cam, target) in &cameras {
        if !matches!(target, Some(RenderTarget::Image(_))) {
            commands.entity(cam).insert((RenderTarget::Image(scene.0.clone().into()), Camera { order: 0, ..default() }));
        }
    }
    let scale = settings.0.clone().normalized().render_scale;
    let size = window.physical_size().as_vec2() * scale;
    let want = Extent3d { width: (size.x as u32).max(1), height: (size.y as u32).max(1), depth_or_array_layers: 1 };
    let Some(image) = images.get(&scene.0) else { return };
    if image.texture_descriptor.size != want {
        if let Some(mut image) = images.get_mut(&scene.0) {
            image.resize(want);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_filling_a_screen_is_not_remembered() {
        let place = |width, height| WindowPlace { x: 0, y: 0, width, height };
        assert!(fills_a_screen(place(1512, 982), &[(1512.0, 982.0)]));
        assert!(fills_a_screen(place(2560, 1440), &[(1512.0, 982.0), (2560.0, 1440.0)]));
        assert!(!fills_a_screen(place(1200, 800), &[(1512.0, 982.0)]));
    }

    #[test]
    fn scene_image_keeps_no_pixels_in_memory() {
        let mut image = scene_target();
        assert!(image.data.is_none());
        image.resize(Extent3d { width: 5120, height: 2880, depth_or_array_layers: 1 });
        assert!(image.data.is_none());
    }
}
