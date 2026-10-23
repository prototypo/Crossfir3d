//! Camera rig: orbit position, gliding, following the player, and cards facing the camera.

use super::*;

/// The direction the camera looks along the ground for a yaw (0 = north, -Z).
fn look_dir(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, -yaw.cos())
}

/// Above the player's feet and `side` to the right.
fn shoulder(focus: Vec3, yaw: f32, side: f32) -> Vec3 {
    focus + Vec3::Y * LOOK_HEIGHT + look_dir(yaw + std::f32::consts::FRAC_PI_2) * side
}

fn camera_eye(focus: Vec3, distance: f32, yaw: f32, pitch: f32, side: f32) -> Vec3 {
    shoulder(focus, yaw, side) - look_dir(yaw) * pitch.cos() * distance + Vec3::Y * pitch.sin() * distance
}

/// Looks `ahead` past the shoulder point.
pub(super) fn camera_transform(focus: Vec3, distance: f32, yaw: f32, pitch: f32, side: f32, ahead: f32) -> Transform {
    let target = shoulder(focus, yaw, side) + look_dir(yaw) * ahead;
    Transform::from_translation(camera_eye(focus, distance, yaw, pitch, side)).looking_at(target, Vec3::Y)
}

pub(super) fn glide(time: Res<Time>, comc: Res<LastComc>, mut q: Query<(&mut Transform, &Glide), Without<CameraRig>>) {
    let step_secs = comc.time_ms.clamp(100, 400) as f32 / 1000.0;
    let max = time.delta_secs() / step_secs;
    for (mut t, g) in &mut q {
        let d = g.target - t.translation;
        // Far jumps (teleports, new maps) snap.
        t.translation = if d.length() > 2.0 { g.target } else { t.translation + d.clamp_length_max(max) };
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn follow_player(
    time: Res<Time>,
    comc: Res<LastComc>,
    view: Res<View>,
    size: Res<ViewSize>,
    heading: Res<CameraHeading>,
    cam: Res<CameraView>,
    solid: Res<Solid>,
    scroll: Res<AccumulatedMouseScroll>,
    mut pinch: MessageReader<PinchGesture>,
    doorstep: Res<Doorstep>,
    rig: Single<(&mut Transform, &mut CameraRig, &mut DistanceFog)>,
) {
    let (mut t, mut rig, mut fog) = rig.into_inner();
    // A trackpad's two-finger scroll comes in pixels, a mouse wheel in lines.
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_LINE,
    };
    let pinched: f32 = pinch.read().map(|p| p.0).sum();
    if cam.top {
        rig.top_distance = zoom(rig.top_distance, lines, pinched, TOP_ZOOM_MIN, TOP_ZOOM_MAX);
    } else {
        rig.distance = zoom(rig.distance, lines, pinched, ZOOM_MIN, ZOOM_MAX);
    }
    let (px, py) = view.0.world((size.0 / 2) as i8, (size.1 / 2) as i8);
    let target = doorstep.0.map_or(Vec3::new(px as f32 + 0.5, 0.0, py as f32 + 0.5), |d| d.with_y(0.0));
    let step_secs = comc.time_ms.clamp(100, 400) as f32 / 1000.0;
    let d = target - rig.focus;
    rig.focus = if d.length() > 2.0 { target } else { rig.focus + d.clamp_length_max(time.delta_secs() / step_secs) };
    let behind = heading.0 as f32 * std::f32::consts::FRAC_PI_4;
    let (goal, pitch, want, side, ahead) = if cam.top {
        (0.0, TOP_PITCH_DEG, rig.top_distance, 0.0, 0.0)
    } else if cam.face {
        (behind + std::f32::consts::PI, FACE_PITCH_DEG, FACE_DISTANCE, 0.0, 0.0)
    } else {
        (behind, PITCH_DEG, rig.distance, SHOULDER, AHEAD)
    };
    // Ease the yaw the short way round toward the goal (about 0.4 s per 45°).
    let diff = (goal - rig.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    rig.yaw += diff.clamp(-1.0, 1.0) * (time.delta_secs() * 4.0).min(1.0);
    // Pull in while a wall or building stands between player and camera; never rise
    // above them (that looked into upper floors).
    let (focus, yaw) = (rig.focus, rig.yaw);
    let blocked = |dist: f32| {
        let eye = camera_eye(focus, dist, yaw, pitch.to_radians(), side);
        sight_blocked((focus.x, focus.z), (eye.x, eye.z), |x, y| solid.0.contains(&(x, y)))
    };
    let distance = if cam.top { want } else { pull_in(want, blocked) };
    let step = pitch.to_radians() - rig.pitch;
    rig.pitch += step * (time.delta_secs() * 4.0).min(1.0);
    *t = camera_transform(rig.focus, distance, rig.yaw, rig.pitch, side, ahead);
    // Haze from the edge of what the server sends out to the edge of map memory: nearby
    // houses keep their colours, remembered ones fade.
    let radius = size.0.min(size.1) as f32 / 2.0;
    fog.falloff = FogFalloff::Linear { start: distance + radius, end: distance + MEMORY_SQUARES as f32 };
}

/// The player on a house's squares stands on its doorstep, outside, with the house in
/// view. One-square things (a well) are drawn with them; anything bigger they stand in
/// (a castle) is hidden.
pub(super) fn hide_entered(
    view: Res<View>,
    size: Res<ViewSize>,
    mut step: ResMut<Doorstep>,
    mut exit: ResMut<DoorExit>,
    mut q: Query<(&Span, Option<&Door>, &mut Visibility)>,
    mut own: Query<&mut Glide, With<OwnCharacter>>,
) {
    let (px, py) = view.0.world((size.0 / 2) as i8, (size.1 / 2) as i8);
    step.0 = None;
    for (Span(x0, y0, x1, y1), door, mut vis) in &mut q {
        let inside = (*x0..=*x1).contains(&px) && (*y0..=*y1).contains(&py);
        if let (true, Some(Door(at))) = (inside, door) {
            step.0 = Some(*at);
        }
        let big = x1 > x0 || y1 > y0;
        vis.set_if_neq(if inside && door.is_none() && big { Visibility::Hidden } else { Visibility::Inherited });
    }
    // The square in front of the door, one step out of the doorway.
    exit.came_out = step.0.is_some() && exit.new_map;
    exit.out = step.0.map(|at| {
        let d = (at.x.floor() as i32 - px, (at.z + 0.25).floor() as i32 - py);
        compass((d.0.signum() as i8, d.1.signum() as i8), 0)
    });
    // Moving off the house gives the character a new tile target (`sync_visuals`).
    if let (Some(at), Ok(mut glide)) = (step.0, own.single_mut()) {
        glide.target = at;
    }
}

/// Cards turn about the vertical axis to face the camera.
pub(super) fn face_camera(rig: Single<&CameraRig>, mut cards: Query<&mut Transform, With<Card>>) {
    let rotation = Quat::from_rotation_y(-rig.yaw);
    for mut t in &mut cards {
        t.rotation = rotation;
    }
}
