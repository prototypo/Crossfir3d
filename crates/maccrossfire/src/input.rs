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

/// While a key is held, send one move and wait for its `comc` before the next.
pub fn send_moves(
    keys: Res<ButtonInput<KeyCode>>,
    line: Res<CommandLine>,
    comc: Res<LastComc>,
    mut seq: ResMut<NcomSeq>,
    mut waiting: Local<Option<u16>>,
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    if waiting.is_some_and(|p| p != comc.packet) {
        return;
    }
    *waiting = None;
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
        *waiting = Some(packet);
        session.send([ClientMsg::Ncom { packet, repeat: 0, command: command.into() }]);
    }
}

#[cfg(test)]
mod tests {
    use super::direction;

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
