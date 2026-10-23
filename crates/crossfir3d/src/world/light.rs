//! Lighting: the sun and moon, and the darkness of the player's square.

use super::*;

/// The game clock sets where the light comes from and its colour; its strength follows
/// the darkness of the player's square (the server darkens by hour, season, indoors).
/// The player's square's light, 0 (dark) ..= 255 (the server darkens by hour, season,
/// indoors).
pub(crate) fn darkness(view: &View, size: &ViewSize) -> u8 {
    view.0.cell((size.0 / 2) as i8, (size.1 / 2) as i8).map_or(255, |c| c.darkness)
}

/// Where the sun (or moon) is now, and the direction toward it.
pub(crate) fn sun_now(clock: &Clock) -> (SunPose, Vec3) {
    // CROSSFIR3D_HOUR fixes the hour for screenshots; read once, not every frame.
    static FORCED: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    let forced = *FORCED.get_or_init(|| std::env::var("CROSSFIR3D_HOUR").ok().and_then(|h| h.parse().ok()));
    let pose = sun_pose(forced.or(clock.0.hour()).unwrap_or(FALLBACK_HOUR));
    // azimuth 0 = east (+X), PI/2 = south (+Z), PI = west (-X)
    let dir = Vec3::new(
        pose.azimuth.cos() * pose.elevation.cos(),
        pose.elevation.sin(),
        pose.azimuth.sin() * pose.elevation.cos(),
    );
    (pose, dir)
}

pub(super) fn lighting(
    view: Res<View>,
    size: Res<ViewSize>,
    clock: Res<Clock>,
    settings: Res<CurrentSettings>,
    indoors: Res<Indoors>,
    sun: Single<(&mut DirectionalLight, &mut Transform), With<Sun>>,
) {
    let (mut light, mut transform) = sun.into_inner();
    let dark = darkness(&view, &size);
    // Indoors no sun reaches: a dim, warm torchlight from high up, whatever the hour.
    if indoors.0 {
        *transform = Transform::from_xyz(3.0, 20.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y);
        light.color = Color::srgb(1.0, 0.75, 0.5);
        light.illuminance = 0.35 * SUN_LUX * dark as f32 / 255.0;
        light.shadow_maps_enabled = settings.0.shadows != Quality::Off;
        return;
    }
    let (pose, dir) = sun_now(&clock);
    *transform = Transform::from_translation(dir * 20.0).looking_at(Vec3::ZERO, Vec3::Y);
    let (color, strength) = if pose.moon {
        (Color::srgb(0.6, 0.7, 1.0), MOON_LUX)
    } else if pose.elevation < 15f32.to_radians() {
        (Color::srgb(1.0, 0.72, 0.45), SUN_LUX)
    } else {
        (Color::srgb(1.0, 0.97, 0.9), SUN_LUX)
    };
    light.color = color;
    light.illuminance = strength * dark as f32 / 255.0;
    light.shadow_maps_enabled = !pose.moon && settings.0.shadows != Quality::Off;
}
