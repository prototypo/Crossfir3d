//! Arrow keys / WASD to ncom move commands.

use bevy::prelude::*;
use cfproto::ClientMsg;

use maccrossfire::modes::{act, Act, KeyName, Mode};
use maccrossfire::slots::{dir_number, slot_commands};

use crate::{HudState, Inv, KeyMode, LastAction, LastComc, Msgs, NcomSeq, Session, Slots, Spells};

/// The direction of the last step or step attempt (1 = north .. 8 = northwest).
#[derive(Resource)]
pub struct Facing(pub u8);

impl Default for Facing {
    fn default() -> Self {
        Facing(5) // south, towards the camera
    }
}

/// Opposite keys cancel out.
pub fn direction(up: bool, down: bool, left: bool, right: bool) -> Option<&'static str> {
    let dy = down as i8 - up as i8;
    let dx = right as i8 - left as i8;
    Some(match (dx, dy) {
        (0, -1) => "north",
        (0, 1) => "south",
        (-1, 0) => "west",
        (1, 0) => "east",
        (1, -1) => "northeast",
        (-1, -1) => "northwest",
        (1, 1) => "southeast",
        (-1, 1) => "southwest",
        _ => return None,
    })
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

/// A tap sends one move. A held key repeats after a short delay, one move per `comc`.
#[allow(clippy::too_many_arguments)]
pub fn send_moves(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<KeyMode>,
    mut msgs: ResMut<Msgs>,
    mut facing: ResMut<Facing>,
    comc: Res<LastComc>,
    time: Res<Time>,
    mut seq: ResMut<NcomSeq>,
    mut wait: ResMut<MoveWait>,
    mut first_step_at: Local<Option<f32>>,
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    let now = time.elapsed_secs();
    // just_pressed too: a press and release in one frame (a very fast tap) still steps.
    let held = |a, b| keys.any_pressed([a, b]) || keys.any_just_pressed([a, b]);
    let dir = direction(
        held(KeyCode::ArrowUp, KeyCode::KeyW),
        held(KeyCode::ArrowDown, KeyCode::KeyS),
        held(KeyCode::ArrowLeft, KeyCode::KeyA),
        held(KeyCode::ArrowRight, KeyCode::KeyD),
    );
    let (Some(command), Mode::World) = (dir, mode.0) else {
        *first_step_at = None; // key released: the next press is a new tap
        return;
    };
    if !may_send(wait.0, comc.packet, now) || !wants_step(first_step_at.map(|t| now - t)) {
        return;
    }
    first_step_at.get_or_insert(now);
    msgs.0.close_box(); // walking away closes the parchment box
    facing.0 = dir_number(command).unwrap_or(facing.0);
    let packet = seq.next();
    wait.0 = Some((packet, now));
    session.send([ClientMsg::Ncom { packet, repeat: 0, command: command.into() }]);
}

fn key_name(key: KeyCode) -> KeyName {
    match key {
        KeyCode::KeyE => KeyName::E,
        KeyCode::KeyG => KeyName::G,
        KeyCode::KeyD => KeyName::D,
        KeyCode::KeyX => KeyName::X,
        KeyCode::KeyI => KeyName::I,
        KeyCode::KeyL => KeyName::L,
        KeyCode::KeyM => KeyName::M,
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
    (facing, slots, inv, spells): (Res<Facing>, Res<Slots>, Res<Inv>, Res<Spells>),
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    let pressed: Vec<KeyName> = keys.get_just_pressed().map(|k| key_name(*k)).collect();
    for key in pressed {
        let mut command = |c: &str| {
            last.0 = Some(time.elapsed_secs());
            session.send([ClientMsg::Ncom { packet: seq.next(), repeat: 0, command: c.into() }]);
        };
        match act(mode.0, key, msgs.0.boxed.is_some()) {
            Act::Apply => command("apply"),
            Act::Get => command("get"),
            Act::CloseBox => msgs.0.close_box(),
            Act::OpenInventory => mode.0 = Mode::Inventory,
            Act::CloseInventory => mode.0 = Mode::World,
            Act::ToggleLog => hud.tall_log = !hud.tall_log,
            Act::Slot(i) => {
                let Some(slot) = &slots.0 .0[i] else { continue };
                for c in slot_commands(slot, &inv.0, &spells.0, facing.0) {
                    command(&c);
                }
            }
            Act::ToggleMute | Act::OpenCommand | Act::Nothing => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{direction, may_send, wants_step};

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

    #[test]
    fn cardinals() {
        assert_eq!(direction(true, false, false, false), Some("north"));
        assert_eq!(direction(false, true, false, false), Some("south"));
        assert_eq!(direction(false, false, true, false), Some("west"));
        assert_eq!(direction(false, false, false, true), Some("east"));
    }

    #[test]
    fn diagonals() {
        assert_eq!(direction(true, false, false, true), Some("northeast"));
        assert_eq!(direction(true, false, true, false), Some("northwest"));
        assert_eq!(direction(false, true, false, true), Some("southeast"));
        assert_eq!(direction(false, true, true, false), Some("southwest"));
    }

    #[test]
    fn opposites_cancel() {
        assert_eq!(direction(true, true, false, false), None);
        assert_eq!(direction(true, true, false, true), Some("east"));
        assert_eq!(direction(false, false, false, false), None);
    }
}
