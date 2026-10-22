//! The sky: blue with drifting clouds by day, dark with stars at night, and a disc where
//! the game clock puts the sun, with a warm glow round it while it is low. A dome round
//! the camera carries the colours (one per vertex); stars, clouds, and sun sit inside it,
//! all past the fog, which takes the sky's colour so the edge of the view fades into it.

use bevy::light::NotShadowCaster;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

use crate::world::{darkness, sun_now, CameraRig};
use crate::{Clock, View, ViewSize};

/// Sky things sit about this far from the camera: past the fog, inside the far plane.
const RADIUS: f32 = 150.0;
const STARS: usize = 500;
const CLOUDS: usize = 16;
/// Radians per second the clouds drift round.
const CLOUD_DRIFT: f32 = 0.004;

const NIGHT: [f32; 3] = [0.01, 0.015, 0.05];
const TWILIGHT: [f32; 3] = [0.15, 0.2, 0.4];
const DAY: [f32; 3] = [0.42, 0.65, 0.95];
const GLOW: [f32; 3] = [0.95, 0.5, 0.3];
/// The low sun's glow reaches this far round the sky from it.
const GLOW_DEG: f32 = 60.0;

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| a[i] * (1.0 - t) + b[i] * t)
}

/// For the sun `elevation_deg` above the horizon (or the moon up): the sky colour away
/// from the sun, how much the stars show, and how much the clouds show, each 0..=1.
/// Night turns to twilight over the sun's first 5°, twilight to day over the next 10°.
pub fn sky(elevation_deg: f32, moon: bool) -> ([f32; 3], f32, f32) {
    if moon {
        return (NIGHT, 1.0, 0.0);
    }
    let dawn = elevation_deg / 5.0;
    let day = (elevation_deg - 5.0) / 10.0;
    let colour = if dawn < 1.0 { mix(NIGHT, TWILIGHT, dawn) } else { mix(TWILIGHT, DAY, day) };
    (colour, 1.0 - dawn.clamp(0.0, 1.0), (elevation_deg / 10.0).clamp(0.0, 1.0))
}

/// The sky colour toward `dir`, with the sun toward `sun`: `base` (from `sky`) warmed
/// within GLOW_DEG of a low sun (fading out as it climbs from 5° to 20°).
pub fn sky_toward(dir: Vec3, sun: Vec3, base: [f32; 3], elevation_deg: f32, moon: bool) -> [f32; 3] {
    if moon {
        return base;
    }
    let low = 1.0 - ((elevation_deg - 5.0) / 15.0).clamp(0.0, 1.0);
    let near = 1.0 - dir.angle_between(sun).to_degrees() / GLOW_DEG;
    mix(base, GLOW, near.clamp(0.0, 1.0).powf(1.5) * low)
}

#[derive(Component)]
pub struct SkyRoot;

#[derive(Component)]
pub struct SunDisc;

#[derive(Component)]
pub struct Clouds;

/// The dark stone ceiling over an indoor map (in place of the sky).
#[derive(Component)]
pub struct Ceiling;

/// How high the ceiling is: on top of the indoor walls (the stone wall is 1.2 high).
const CEILING: f32 = 1.2 * crate::world::INDOOR_WALLS;

#[derive(Resource)]
pub struct SkyMaterials {
    stars: Handle<StandardMaterial>,
    clouds: Handle<StandardMaterial>,
    dome: Handle<Mesh>,
}

fn sky_material(colour: Color, blend: bool) -> StandardMaterial {
    StandardMaterial {
        base_color: colour,
        unlit: true,
        fog_enabled: false,
        alpha_mode: if blend { AlphaMode::Blend } else { AlphaMode::Opaque },
        // The dome is seen from inside.
        cull_mode: None,
        ..default()
    }
}

pub fn spawn_sky(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let ball = meshes.add(Sphere::new(1.0).mesh().ico(2).unwrap());
    let stars = materials.add(sky_material(Color::WHITE, true));
    let clouds = materials.add(sky_material(Color::WHITE, true));
    let sun = materials.add(sky_material(Color::srgb(1.0, 0.95, 0.7), false));
    let root = commands.spawn((SkyRoot, Transform::default(), Visibility::default())).id();
    // Seen from below only: from above (the top view) it is not drawn.
    commands.spawn((
        Ceiling,
        Mesh3d(meshes.add(Plane3d::new(Vec3::NEG_Y, Vec2::splat(60.0)))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.16, 0.15, 0.14), perceptual_roughness: 1.0, ..default() })),
        Transform::from_xyz(0.0, CEILING, 0.0),
        NotShadowCaster,
        Visibility::Hidden,
    ));
    let dome = meshes.add(Sphere::new(1.0).mesh().ico(4).unwrap());
    commands.spawn((
        Mesh3d(dome.clone()),
        MeshMaterial3d(materials.add(sky_material(Color::WHITE, false))),
        Transform::from_scale(Vec3::splat(RADIUS * 1.2)),
        NotShadowCaster,
        ChildOf(root),
    ));
    // Stars spread evenly over the sky (a Fibonacci spiral), above the rooftops.
    for i in 0..STARS {
        let y = 0.1 + 0.9 * (i as f32 + 0.5) / STARS as f32;
        let r = (1.0 - y * y).sqrt();
        let a = i as f32 * 2.399_963;
        let size = [0.25, 0.35, 0.5][i * 7 % 3];
        commands.spawn((
            Mesh3d(ball.clone()),
            MeshMaterial3d(stars.clone()),
            Transform::from_translation(Vec3::new(r * a.cos(), y, r * a.sin()) * RADIUS).with_scale(Vec3::splat(size)),
            NotShadowCaster,
            ChildOf(root),
        ));
    }
    // Clouds: a few flattened puffs each, low in the sky so the shoulder view sees them.
    let ring = commands.spawn((Clouds, Transform::default(), Visibility::default(), ChildOf(root))).id();
    let puffs = [(Vec3::new(-6.0, 0.0, 0.0), 6.0), (Vec3::new(0.0, 1.5, 0.0), 8.0), (Vec3::new(6.0, 0.0, 1.0), 6.0), (Vec3::new(2.0, -0.5, -3.0), 5.0)];
    for c in 0..CLOUDS {
        let a = c as f32 * std::f32::consts::TAU / CLOUDS as f32 + (c * 37 % 10) as f32 * 0.05;
        let dist = 90.0 + (c * 37 % 50) as f32;
        let at = Vec3::new(a.cos() * dist, 16.0 + (c * 13 % 14) as f32, a.sin() * dist);
        let cloud = commands
            .spawn((Transform::from_translation(at).looking_at(Vec3::new(0.0, at.y, 0.0), Vec3::Y), Visibility::default(), ChildOf(ring)))
            .id();
        for (offset, size) in puffs {
            commands.spawn((
                Mesh3d(ball.clone()),
                MeshMaterial3d(clouds.clone()),
                Transform::from_translation(offset).with_scale(Vec3::new(size, size * 0.45, size * 0.8)),
                NotShadowCaster,
                ChildOf(cloud),
            ));
        }
    }
    commands.spawn((SunDisc, Mesh3d(ball), MeshMaterial3d(sun), Transform::from_scale(Vec3::splat(6.0)), NotShadowCaster, ChildOf(root)));
    commands.insert_resource(SkyMaterials { stars, clouds, dome });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_sky(
    time: Res<Time>,
    clock: Res<Clock>,
    view: Res<View>,
    size: Res<ViewSize>,
    mats: Res<SkyMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clear: ResMut<ClearColor>,
    camera: Single<(&Transform, &mut DistanceFog), With<CameraRig>>,
    mut root: Single<&mut Transform, (With<SkyRoot>, Without<CameraRig>)>,
    sun: Single<(&mut Transform, &mut Visibility), (With<SunDisc>, Without<SkyRoot>, Without<CameraRig>)>,
    mut clouds: Single<&mut Transform, (With<Clouds>, Without<SunDisc>, Without<SkyRoot>, Without<CameraRig>)>,
    (indoors, mut sky_shown, ceiling): (
        Res<crate::world::Indoors>,
        Single<&mut Visibility, (With<SkyRoot>, Without<SunDisc>, Without<Ceiling>)>,
        Single<(&mut Transform, &mut Visibility), (With<Ceiling>, Without<SkyRoot>, Without<SunDisc>, Without<Clouds>, Without<CameraRig>)>,
    ),
) {
    let (eye, mut fog) = camera.into_inner();
    let (pose, dir) = sun_now(&clock);
    let (colour, stars, cloudy) = sky(pose.elevation.to_degrees(), pose.moon);
    // Dark places (dungeons, deep indoors) get a dark sky too.
    let light = darkness(&view, &size) as f32 / 255.0;
    if let Some(mut dome) = meshes.get_mut(&mats.dome)
        && let Some(VertexAttributeValues::Float32x3(points)) = dome.attribute(Mesh::ATTRIBUTE_POSITION)
    {
        let colours: Vec<[f32; 4]> = points
            .iter()
            .map(|p| {
                let [r, g, b] = sky_toward(Vec3::from(*p), dir, colour, pose.elevation.to_degrees(), pose.moon);
                // The dome colours are linear; the named colours are sRGB.
                let c = Color::srgb(r * light, g * light, b * light).to_linear();
                [c.red, c.green, c.blue, 1.0]
            })
            .collect();
        dome.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    }
    let colour = Color::srgb(colour[0] * light, colour[1] * light, colour[2] * light);
    // Indoors: a stone ceiling over the camera's square, darkness beyond, no sky.
    let (mut ceiling_t, mut ceiling_shown) = ceiling.into_inner();
    let inside = indoors.0;
    sky_shown.set_if_neq(if inside { Visibility::Hidden } else { Visibility::Inherited });
    ceiling_shown.set_if_neq(if inside { Visibility::Inherited } else { Visibility::Hidden });
    ceiling_t.translation = Vec3::new(eye.translation.x, CEILING, eye.translation.z);
    let colour = if inside { Color::srgb(0.02, 0.02, 0.025) } else { colour };
    clear.0 = colour;
    fog.color = colour;
    root.translation = eye.translation;
    clouds.rotation = Quat::from_rotation_y(time.elapsed_secs() * CLOUD_DRIFT);
    let (mut sun_t, mut sun_vis) = sun.into_inner();
    sun_t.translation = dir * RADIUS;
    sun_vis.set_if_neq(if pose.moon { Visibility::Hidden } else { Visibility::Inherited });
    if let Some(mut m) = materials.get_mut(&mats.stars) {
        m.base_color = Color::srgba(1.0, 1.0, 1.0, stars * light);
    }
    if let Some(mut m) = materials.get_mut(&mats.clouds) {
        // White by day, greyer as the light fails.
        let [r, g, b] = mix([0.7, 0.7, 0.75], [1.0, 1.0, 1.0], cloudy);
        m.base_color = Color::srgba(r * light, g * light, b * light, 0.9 * cloudy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_dawn_day() {
        assert_eq!(sky(60.0, true), (NIGHT, 1.0, 0.0));
        assert_eq!(sky(0.0, false), (NIGHT, 1.0, 0.0));
        assert_eq!(sky(5.0, false).0, TWILIGHT);
        assert_eq!(sky(5.0, false).1, 0.0);
        assert_eq!(sky(45.0, false), (DAY, 0.0, 1.0));
        let (c, _, clouds) = sky(10.0, false);
        assert!(c[2] > TWILIGHT[2] && c[2] < DAY[2] && clouds == 1.0);
    }

    #[test]
    fn glow_only_near_a_low_sun() {
        let east = Vec3::X;
        let base = sky(2.0, false).0;
        assert_eq!(sky_toward(east, east, base, 2.0, false), GLOW, "at the sun");
        assert_eq!(sky_toward(-east, east, base, 2.0, false), base, "opposite");
        assert_eq!(sky_toward(Vec3::Z, east, base, 2.0, false), base, "90° away");
        let near = sky_toward(Vec3::new(1.0, 0.0, 0.3).normalize(), east, base, 2.0, false);
        assert!(near[0] > base[0] && near[0] < GLOW[0], "partly warm 17° away");
        assert_eq!(sky_toward(east, east, DAY, 45.0, false), DAY, "no glow high up");
    }
}
