//! Arrow keys / WASD to ncom move commands.

use bevy::prelude::*;
use cfproto::ClientMsg;

use crate::ui::CommandLine;
use crate::{LastComc, NcomSeq, Session};

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

/// While a key is held, send one move and wait for its `comc` before the next.
pub fn send_moves(
    keys: Res<ButtonInput<KeyCode>>,
    line: Res<CommandLine>,
    comc: Res<LastComc>,
    time: Res<Time>,
    mut seq: ResMut<NcomSeq>,
    mut wait: ResMut<MoveWait>,
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    let now = time.elapsed_secs();
    if !may_send(wait.0, comc.packet, now) {
        return;
    }
    wait.0 = None;
    if line.open {
        return;
    }
    let held = |a, b| keys.pressed(a) || keys.pressed(b);
    let dir = direction(
        held(KeyCode::ArrowUp, KeyCode::KeyW),
        held(KeyCode::ArrowDown, KeyCode::KeyS),
        held(KeyCode::ArrowLeft, KeyCode::KeyA),
        held(KeyCode::ArrowRight, KeyCode::KeyD),
    );
    if let Some(command) = dir {
        let packet = seq.next();
        wait.0 = Some((packet, now));
        session.send([ClientMsg::Ncom { packet, repeat: 0, command: command.into() }]);
    }
}

#[cfg(test)]
mod tests {
    use super::{direction, may_send};

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
