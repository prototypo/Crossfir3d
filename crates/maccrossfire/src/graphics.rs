//! Applies the video settings to the window, camera, and lights; caps the frame rate;
//! remembers where the window was.

use std::time::{Duration, Instant};

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::light::DirectionalLightShadowMap;
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection};
use bevy::window::{Monitor, MonitorSelection, PresentMode, PrimaryWindow, WindowMode, WindowMoved, WindowPosition, WindowResized};
use maccrossfire::settings::{
    default_settings_path, load_settings, save_settings, AntiAlias, FrameRate, Quality, Settings, WindowPlace,
};


#[derive(Resource)]
pub struct CurrentSettings(pub Settings);

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_settings, keep_window_on_screen, remember_window, save_settings_later))
            .add_systems(Last, frame_cap);
    }
}

/// The primary window as the saved settings describe it (used when building the app).
pub fn initial_window(s: &Settings) -> Window {
    let mut w = Window { title: "MacCrossfire".into(), ..default() };
    w.mode = window_mode(s);
    w.present_mode = present_mode(s.frame_rate);
    if let Some(p) = s.window {
        w.position = WindowPosition::At(IVec2::new(p.x, p.y));
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

/// Everything except the window place, which changes on every move and needs no re-apply.
fn look(s: &Settings) -> Settings {
    Settings { window: None, ..s.clone() }.normalized()
}

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
            AntiAlias::Off => e.insert(Msaa::Off).remove::<TemporalAntiAliasing>(),
            AntiAlias::Msaa4 => e.insert(Msaa::Sample4).remove::<TemporalAntiAliasing>(),
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

/// A saved window position on a monitor that is gone: centre on the main screen.
fn keep_window_on_screen(
    mut done: Local<bool>,
    monitors: Query<&Monitor>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if *done || monitors.is_empty() {
        return;
    }
    *done = true;
    let WindowPosition::At(p) = window.position else { return };
    let visible = monitors.iter().any(|m| {
        let (min, size) = (m.physical_position, IVec2::new(m.physical_width as i32, m.physical_height as i32));
        p.x >= min.x && p.y >= min.y && p.x < min.x + size.x && p.y < min.y + size.y
    });
    if !visible {
        window.position = WindowPosition::Centered(MonitorSelection::Primary);
    }
}

fn remember_window(
    mut moved: MessageReader<WindowMoved>,
    mut resized: MessageReader<WindowResized>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut last_pos: Local<Option<IVec2>>,
    mut settings: ResMut<CurrentSettings>,
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
    let place = WindowPlace { x: p.x, y: p.y, width: window.width() as u32, height: window.height() as u32 };
    if settings.0.window != Some(place) {
        settings.0.window = Some(place);
    }
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
