//! Arrow keys to ncom move commands, and one-key actions.

use bevy::prelude::*;
use cfproto::ClientMsg;

use maccrossfire::camera::{compass, heading_of, nearest_dir, screen_dir, Heading, CREATURE_LAYERS};
use maccrossfire::modes::{act, Act, KeyName, Mode};
use maccrossfire::anim::event_for_command;
use maccrossfire::slots::{dir_number, fire_commands, slot_commands};

use crate::characters3d::PlayerAnimEvents;

use crate::{HudState, Inv, KeyMode, LastAction, LastComc, Msgs, NcomSeq, PlayerStats, Session, Slots, Spells, View, ViewSize};

/// Where the camera looks (45° steps); the world rig eases its yaw toward it.
/// It follows the player's facing, so the camera stays behind them.
#[derive(Resource, Default)]
pub struct CameraHeading(pub Heading);

/// Tab: the top view, north up. F (held): looking at the player's face.
#[derive(Resource, Default)]
pub struct CameraView {
    pub top: bool,
    pub face: bool,
}

/// In a house (shown on its doorstep): the move from the player's square to the square
/// in front of the door, which any step south takes. `came_out`: they got here on a
/// new map (out of a shop's own map), not by a step in, so they step out at once.
/// `new_map` is set by `newmap` and cleared by the next move sent.
#[derive(Resource, Default)]
pub struct DoorExit {
    pub out: Option<&'static str>,
    pub came_out: bool,
    pub new_map: bool,
}

/// The direction of the last step or step attempt (1 = north .. 8 = northwest).
#[derive(Resource)]
pub struct Facing(pub u8);

impl Default for Facing {
    fn default() -> Self {
        Facing(1) // north, away from the camera
    }
}

/// The move we sent and are waiting on: (packet, time sent in seconds).
#[derive(Resource, Default)]
pub struct MoveWait(pub Option<(u16, f32)>);

/// Give up waiting for a comc after this long (the server skips comc in some states).
const WAIT_TIMEOUT_SECS: f32 = 1.0;

/// True when the next move may be sent: nothing pending, the pending packet (or a later
/// one) was confirmed, or we waited too long.
pub fn may_send(wait: Option<(u16, f32)>, acked: u16, now: f32) -> bool {
    match wait {
        None => true,
        Some((packet, sent)) => acked.wrapping_sub(packet) as i16 >= 0 || now - sent > WAIT_TIMEOUT_SECS,
    }
}

/// Hold a key this long before steps repeat, so a tap is exactly one step.
const REPEAT_DELAY_SECS: f32 = 0.3;

/// `held_secs` is None for a key that has not stepped yet.
pub fn wants_step(held_secs: Option<f32>) -> bool {
    held_secs.is_none_or(|t| t >= REPEAT_DELAY_SECS)
}

/// A tap the way the player faces sends one move; a tap any other way only turns them
/// (and the camera) to face it. A held key repeats after a short delay, one move per
/// `comc`, so holding a new direction turns, then walks.
/// Directions are relative to the way the player faces, in every view: Up is forward,
/// Left/Right turn 90°, Down turns round. The compass direction is fixed at key-down,
/// so the turn does not bend a held walk.
#[allow(clippy::too_many_arguments)]
pub fn send_moves(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<KeyMode>,
    mut msgs: ResMut<Msgs>,
    mut facing: ResMut<Facing>,
    mut heading: ResMut<CameraHeading>,
    comc: Res<LastComc>,
    time: Res<Time>,
    mut seq: ResMut<NcomSeq>,
    mut wait: ResMut<MoveWait>,
    mut door: ResMut<DoorExit>,
    mut held: Local<Option<(f32, (i8, i8), Heading)>>, // (first step, screen dir, facing at key-down)
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    let now = time.elapsed_secs();
    if door.came_out && may_send(wait.0, comc.packet, now) {
        // Out of the door, facing away from it, the camera behind.
        door.new_map = false;
        facing.0 = 5;
        heading.0 = heading_of(5);
        let packet = seq.next();
        wait.0 = Some((packet, now));
        session.send([ClientMsg::Ncom { packet, repeat: 0, command: door.out.unwrap_or("south").into() }]);
        return;
    }
    // just_pressed too: a press and release in one frame (a very fast tap) still steps.
    let key = |k| keys.pressed(k) || keys.just_pressed(k);
    let screen = screen_dir(key(KeyCode::ArrowUp), key(KeyCode::ArrowDown), key(KeyCode::ArrowLeft), key(KeyCode::ArrowRight));
    let (Some(screen), Mode::World) = (screen, mode.0) else {
        *held = None; // key released: the next press is a new tap
        return;
    };
    // A new key combination locks the camera heading of this moment.
    if held.is_none_or(|(_, s, _)| s != screen) {
        *held = None;
    }
    let first = held.map(|(t, ..)| t);
    if !may_send(wait.0, comc.packet, now) || !wants_step(first.map(|t| now - t)) {
        return;
    }
    let (_, _, locked) = *held.get_or_insert((now, screen, heading_of(facing.0)));
    let command = compass(screen, locked);
    let dir = dir_number(command).unwrap_or(facing.0);
    if dir != facing.0 {
        facing.0 = dir;
        heading.0 = heading_of(dir);
        return;
    }
    // The door is in the south wall.
    let command = match door.out {
        Some(out) if command.starts_with("south") => out,
        _ => command,
    };
    door.new_map = false;
    msgs.0.close_box(); // walking away closes the parchment box
    let packet = seq.next();
    wait.0 = Some((packet, now));
    session.send([ClientMsg::Ncom { packet, repeat: 0, command: command.into() }]);
}

fn key_name(key: KeyCode) -> KeyName {
    match key {
        KeyCode::KeyA => KeyName::A,
        KeyCode::KeyE => KeyName::E,
        KeyCode::KeyG => KeyName::G,
        KeyCode::KeyD => KeyName::D,
        KeyCode::KeyX => KeyName::X,
        KeyCode::KeyI => KeyName::I,
        KeyCode::KeyL => KeyName::L,
        KeyCode::KeyM => KeyName::M,
        KeyCode::KeyC => KeyName::C,
        KeyCode::KeyT => KeyName::T,
        KeyCode::Space => KeyName::Space,
        KeyCode::Tab => KeyName::Tab,
        KeyCode::Escape => KeyName::Escape,
        KeyCode::Enter => KeyName::Enter,
        KeyCode::Digit1 => KeyName::Digit(1),
        KeyCode::Digit2 => KeyName::Digit(2),
        KeyCode::Digit3 => KeyName::Digit(3),
        KeyCode::Digit4 => KeyName::Digit(4),
        _ => KeyName::Other,
    }
}

/// One-key actions (the spec's key table, via `modes::act`). Enter is handled by the
/// command line, M by the audio plugin.
#[allow(clippy::too_many_arguments)]
pub fn world_keys(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut mode: ResMut<KeyMode>,
    mut msgs: ResMut<Msgs>,
    mut hud: ResMut<HudState>,
    mut last: ResMut<LastAction>,
    mut seq: ResMut<NcomSeq>,
    (mut facing, slots, inv, spells, stats): (ResMut<Facing>, Res<Slots>, Res<Inv>, Res<Spells>, Res<PlayerStats>),
    (map, size): (Res<View>, Res<ViewSize>),
    mut heading: ResMut<CameraHeading>,
    mut view: ResMut<CameraView>,
    mut anim: ResMut<PlayerAnimEvents>,
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    view.face = mode.0 == Mode::World && keys.pressed(KeyCode::KeyF);
    let pressed: Vec<KeyName> = keys.get_just_pressed().map(|k| key_name(*k)).collect();
    for key in pressed {
        let mut command = |c: &str| {
            last.0 = Some(time.elapsed_secs());
            anim.0.extend(event_for_command(c));
            session.send([ClientMsg::Ncom { packet: seq.next(), repeat: 0, command: c.into() }]);
        };
        match act(mode.0, key, msgs.0.boxed.is_some()) {
            Act::Apply => command("apply"),
            Act::Get => command("get"),
            Act::CloseBox => msgs.0.close_box(),
            Act::OpenInventory => mode.0 = Mode::Inventory,
            Act::CloseInventory | Act::CloseSettings => mode.0 = Mode::World,
            Act::OpenSettings => mode.0 = Mode::Settings,
            Act::ToggleLog => hud.tall_log = !hud.tall_log,
            Act::Slot(i) => {
                let Some(slot) = &slots.0 .0[i] else { continue };
                for c in slot_commands(slot, &inv.0, &spells.0, facing.0, &stats.0.range) {
                    command(&c);
                }
            }
            Act::CameraBehind => {
                heading.0 = heading_of(facing.0);
                view.top = false;
            }
            Act::Fire => {
                let fire = fire_commands(&inv.0, facing.0, &stats.0.range);
                if fire.is_empty() {
                    let now = time.elapsed_secs();
                    msgs.0.push(0, 0, "Nothing to fire: ready a bow or wand first (I, select it, E).", now, None);
                }
                for c in &fire {
                    command(c);
                }
            }
            Act::FaceNearest => {
                let (cx, cy) = (size.0 as i32 / 2, size.1 as i32 / 2);
                let creatures = map.0.cells().filter(|(_, c)| c.layers[CREATURE_LAYERS].iter().any(|f| *f != 0));
                if let Some(dir) = nearest_dir(creatures.map(|((x, y), _)| (x as i32 - cx, y as i32 - cy)), 4) {
                    facing.0 = dir_number(dir).unwrap_or(facing.0);
                    heading.0 = heading_of(facing.0);
                }
            }
            Act::TopView => view.top = !view.top,
            Act::ToggleMute | Act::OpenCommand | Act::Nothing => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{may_send, wants_step};

    #[test]
    fn tap_is_one_step() {
        assert!(wants_step(None));
        assert!(!wants_step(Some(0.1)));
    }

    #[test]
    fn hold_repeats() {
        assert!(wants_step(Some(0.31)));
    }

    #[test]
    fn wait_until_confirmed() {
        assert!(may_send(None, 0, 0.0));
        assert!(!may_send(Some((5, 0.0)), 4, 0.5));
        assert!(may_send(Some((5, 0.0)), 5, 0.5));
    }

    #[test]
    fn later_confirmation_counts() {
        assert!(may_send(Some((5, 0.0)), 6, 0.5));
        assert!(may_send(Some((65535, 0.0)), 1, 0.5)); // packet numbers wrap
    }

    #[test]
    fn wait_times_out() {
        assert!(may_send(Some((5, 0.0)), 4, 1.1));
    }



}
