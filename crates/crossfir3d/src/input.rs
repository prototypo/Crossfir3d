//! Arrow keys to ncom move commands, and one-key actions.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use cfproto::ClientMsg;

use crossfir3d::camera::{compass, heading_of, screen_dir, Heading};
use crossfir3d::modes::{KeyName, Mode};
use crossfir3d::messages::{ground_box, ground_things, new_square, object_tags, standing_on};
use crossfir3d::keys::{action_of, skill_command, Action, Binding, Combo, Mods};
use crossfir3d::slots::{dir_number, slot_commands, Kit, Slot};

use crate::ui::PendingBind;
use crate::{Inv, KeyBindings, KeyMode, LastComc, Msgs, NcomSeq, Session};

/// Where the camera looks (45° steps); the world rig eases its yaw toward it.
/// It follows the player's facing, so the camera stays behind them.
#[derive(Resource, Default)]
pub struct CameraHeading(pub Heading);

/// Tab: the top view, north up. F (held): looking at the player's face.
#[derive(Resource)]
pub struct CameraView {
    pub top: bool,
    pub face: bool,
}

impl Default for CameraView {
    /// CROSSFIR3D_TOP: start in the top view (screenshots).
    fn default() -> Self {
        CameraView { top: std::env::var_os("CROSSFIR3D_TOP").is_some(), face: false }
    }
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
/// so the turn does not bend a held walk. The compass keys (number pad, vi keys) walk
/// by the compass instead, as in the old clients.
pub fn send_moves(mut p: MoveParams, mut held: Local<Option<Held>>, session: Option<NonSend<Session>>) {
    let Some(session) = session else { return };
    if p.pending.waiting_key() {
        *held = None; // the next arrow after the bind key is a new tap
        return;
    }
    let now = p.time.elapsed_secs();
    if p.door.came_out && may_send(p.wait.0, p.comc.packet, now) {
        p.step_out(&session, now);
        return;
    }
    let (Some((screen, north_up)), Mode::World) = (p.walk_screen(), p.mode.0) else {
        *held = None; // key released: the next press is a new tap
        return;
    };
    let Some(locked) = p.held_heading(&mut held, screen, north_up, now) else { return };
    let command = compass(screen, locked);
    let dir = dir_number(command).unwrap_or(p.facing.0);
    if p.turn_to(dir) {
        return;
    }
    p.send_step(&session, command, now);
}

/// A held arrow key: when its first step was sent, the screen direction, and the
/// camera heading at key-down (north for a compass key).
#[derive(Clone, Copy)]
pub struct Held {
    at: f32,
    screen: (i8, i8),
    north_up: bool,
    locked: Heading,
}

/// Keys that walk by the compass, as in the old clients: the number pad and the vi keys.
/// Each is a screen direction with north up.
const COMPASS_KEYS: [(KeyCode, (i8, i8)); 16] = [
    (KeyCode::Numpad8, (0, -1)), (KeyCode::Numpad9, (1, -1)), (KeyCode::Numpad6, (1, 0)), (KeyCode::Numpad3, (1, 1)),
    (KeyCode::Numpad2, (0, 1)), (KeyCode::Numpad1, (-1, 1)), (KeyCode::Numpad4, (-1, 0)), (KeyCode::Numpad7, (-1, -1)),
    (KeyCode::KeyK, (0, -1)), (KeyCode::KeyU, (1, -1)), (KeyCode::KeyL, (1, 0)), (KeyCode::KeyN, (1, 1)),
    (KeyCode::KeyJ, (0, 1)), (KeyCode::KeyB, (-1, 1)), (KeyCode::KeyH, (-1, 0)), (KeyCode::KeyY, (-1, -1)),
];

/// Everything `send_moves` reads or changes in a frame.
#[derive(SystemParam)]
pub struct MoveParams<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mode: Res<'w, KeyMode>,
    msgs: ResMut<'w, Msgs>,
    facing: ResMut<'w, Facing>,
    heading: ResMut<'w, CameraHeading>,
    comc: Res<'w, LastComc>,
    time: Res<'w, Time>,
    seq: ResMut<'w, NcomSeq>,
    wait: ResMut<'w, MoveWait>,
    door: ResMut<'w, DoorExit>,
    bindings: Res<'w, KeyBindings>,
    pending: Res<'w, PendingBind>,
}

impl MoveParams<'_> {
    /// Steps out of the door at once, facing away from it with the camera behind.
    fn step_out(&mut self, session: &Session, now: f32) {
        self.door.new_map = false;
        self.facing.0 = 5;
        self.heading.0 = heading_of(5);
        let command = self.door.out.unwrap_or("south");
        self.send_ncom(session, command, now);
    }

    /// The screen direction of the arrow keys held now, else of a compass key (north up:
    /// true); None when none is.
    fn walk_screen(&self) -> Option<((i8, i8), bool)> {
        // just_pressed too: a press and release in one frame (a very fast tap) still steps.
        // A key a binding took no longer moves.
        let key = |k| (self.keys.pressed(k) || self.keys.just_pressed(k)) && !key_taken(&self.bindings, &self.keys, k);
        let arrows = screen_dir(key(KeyCode::ArrowUp), key(KeyCode::ArrowDown), key(KeyCode::ArrowLeft), key(KeyCode::ArrowRight));
        let compass = || COMPASS_KEYS.iter().find(|(k, _)| key(*k)).map(|(_, s)| (*s, true));
        arrows.map(|s| (s, false)).or_else(compass)
    }

    /// Resets `held` when the keys changed, then returns the camera heading locked for this
    /// press if a step may be sent now (recording the press); None if it must wait.
    fn held_heading(&self, held: &mut Option<Held>, screen: (i8, i8), north_up: bool, now: f32) -> Option<Heading> {
        // A new key combination locks the camera heading of this moment.
        *held = held.filter(|h| h.screen == screen && h.north_up == north_up);
        let first = held.map(|h| h.at);
        if !may_send(self.wait.0, self.comc.packet, now) || !wants_step(first.map(|t| now - t)) {
            return None;
        }
        let locked = if north_up { 0 } else { heading_of(self.facing.0) };
        Some(held.get_or_insert(Held { at: now, screen, north_up, locked }).locked)
    }

    /// Sends one walking step, using the door's exit when the step is south, and closes the box.
    fn send_step(&mut self, session: &Session, command: &'static str, now: f32) {
        // The door is in the south wall.
        let command = match self.door.out {
            Some(out) if command.starts_with("south") => out,
            _ => command,
        };
        self.door.new_map = false;
        self.msgs.0.close_box(); // walking away closes the parchment box
        self.send_ncom(session, command, now);
    }

    /// Turns the player and the camera to face `dir`; false if they already do.
    fn turn_to(&mut self, dir: u8) -> bool {
        if dir == self.facing.0 {
            return false;
        }
        self.facing.0 = dir;
        self.heading.0 = heading_of(dir);
        true
    }

    /// Sends one move command and starts waiting for its comc.
    fn send_ncom(&mut self, session: &Session, command: &str, now: f32) {
        let packet = self.seq.next();
        self.wait.0 = Some((packet, now));
        session.send([ClientMsg::Ncom { packet, repeat: 0, command: command.into() }]);
    }
}

/// The keys that act in the world, with the `KeyName` each one stands for.
const ACT_KEYS: [(KeyCode, KeyName); 20] = [
    (KeyCode::KeyA, KeyName::A), (KeyCode::KeyE, KeyName::E), (KeyCode::KeyG, KeyName::G), (KeyCode::KeyD, KeyName::D),
    (KeyCode::KeyX, KeyName::X), (KeyCode::KeyI, KeyName::I), (KeyCode::KeyR, KeyName::R), (KeyCode::KeyM, KeyName::M),
    (KeyCode::KeyC, KeyName::C), (KeyCode::KeyT, KeyName::T), (KeyCode::KeyO, KeyName::O), (KeyCode::KeyV, KeyName::V),
    (KeyCode::Space, KeyName::Space), (KeyCode::Tab, KeyName::Tab), (KeyCode::Escape, KeyName::Escape), (KeyCode::Enter, KeyName::Enter),
    (KeyCode::Digit1, KeyName::Digit(1)), (KeyCode::Digit2, KeyName::Digit(2)), (KeyCode::Digit3, KeyName::Digit(3)), (KeyCode::Digit4, KeyName::Digit(4)),
];

/// The `KeyName` of a key; `Other` for keys that do not act.
fn act_key(key: KeyCode) -> KeyName {
    ACT_KEYS.iter().find(|(k, _)| *k == key).map_or(KeyName::Other, |(_, n)| *n)
}

/// Key names used by bindings (`crossfir3d::keys`), by key position.
const KEY_NAMES: [(KeyCode, &str); 72] = [
    (KeyCode::KeyA, "A"), (KeyCode::KeyB, "B"), (KeyCode::KeyC, "C"), (KeyCode::KeyD, "D"), (KeyCode::KeyE, "E"),
    (KeyCode::KeyF, "F"), (KeyCode::KeyG, "G"), (KeyCode::KeyH, "H"), (KeyCode::KeyI, "I"), (KeyCode::KeyJ, "J"),
    (KeyCode::KeyK, "K"), (KeyCode::KeyL, "L"), (KeyCode::KeyM, "M"), (KeyCode::KeyN, "N"), (KeyCode::KeyO, "O"),
    (KeyCode::KeyP, "P"), (KeyCode::KeyQ, "Q"), (KeyCode::KeyR, "R"), (KeyCode::KeyS, "S"), (KeyCode::KeyT, "T"),
    (KeyCode::KeyU, "U"), (KeyCode::KeyV, "V"), (KeyCode::KeyW, "W"), (KeyCode::KeyX, "X"), (KeyCode::KeyY, "Y"),
    (KeyCode::KeyZ, "Z"),
    (KeyCode::Digit0, "0"), (KeyCode::Digit1, "1"), (KeyCode::Digit2, "2"), (KeyCode::Digit3, "3"), (KeyCode::Digit4, "4"),
    (KeyCode::Digit5, "5"), (KeyCode::Digit6, "6"), (KeyCode::Digit7, "7"), (KeyCode::Digit8, "8"), (KeyCode::Digit9, "9"),
    (KeyCode::F1, "F1"), (KeyCode::F2, "F2"), (KeyCode::F3, "F3"), (KeyCode::F4, "F4"), (KeyCode::F5, "F5"),
    (KeyCode::F6, "F6"), (KeyCode::F7, "F7"), (KeyCode::F8, "F8"), (KeyCode::F9, "F9"), (KeyCode::F10, "F10"),
    (KeyCode::F11, "F11"), (KeyCode::F12, "F12"),
    (KeyCode::ArrowUp, "Up"), (KeyCode::ArrowDown, "Down"), (KeyCode::ArrowLeft, "Left"), (KeyCode::ArrowRight, "Right"),
    (KeyCode::Space, "Space"), (KeyCode::Tab, "Tab"), (KeyCode::Enter, "Enter"), (KeyCode::Escape, "Escape"),
    (KeyCode::Backspace, "Backspace"), (KeyCode::CapsLock, "CapsLock"),
    (KeyCode::Backquote, "`"), (KeyCode::Minus, "-"), (KeyCode::Equal, "="), (KeyCode::BracketLeft, "["),
    (KeyCode::BracketRight, "]"), (KeyCode::Backslash, "\\"), (KeyCode::Semicolon, ";"), (KeyCode::Quote, "'"),
    (KeyCode::Comma, ","), (KeyCode::Period, "."), (KeyCode::Slash, "/"), (KeyCode::Fn, "fn"),
    (KeyCode::Delete, "Delete"), (KeyCode::Home, "Home"),
];

/// The binding name of a key; None for modifier keys and keys bindings do not use.
pub fn key_name(key: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(k, _)| *k == key).map(|(_, n)| *n)
}

#[cfg(test)]
pub fn key_code(name: &str) -> Option<KeyCode> {
    KEY_NAMES.iter().find(|(_, n)| *n == name).map(|(k, _)| *k)
}

pub fn held_mods(keys: &ButtonInput<KeyCode>) -> Mods {
    let any = |a, b| keys.pressed(a) || keys.pressed(b);
    Mods {
        shift: any(KeyCode::ShiftLeft, KeyCode::ShiftRight),
        ctrl: any(KeyCode::ControlLeft, KeyCode::ControlRight),
        alt: any(KeyCode::AltLeft, KeyCode::AltRight),
        cmd: any(KeyCode::SuperLeft, KeyCode::SuperRight),
    }
}

/// The ncom commands a binding sends: a known spell is cast in the facing direction
/// (as a quick slot does), a known skill is used, anything else goes as typed.
pub fn binding_commands(b: &Binding, kit: &Kit, skills: &[String]) -> Vec<String> {
    let names: Vec<String> = kit.spells.all().iter().map(|s| s.name.clone()).collect();
    match action_of(&b.command, &names, skills) {
        Action::Spell(n) => slot_commands(&Slot::Spell(n), kit),
        Action::Skill(n) => vec![skill_command(&n)],
        Action::Command(c) => c.split(';').map(str::trim).filter(|c| !c.is_empty()).map(String::from).collect(),
    }
}

/// Whether pressing `code` with the modifiers held now runs a binding: then the key's
/// own game job (walk, face view, mute, command box) must not run too.
pub fn key_taken(bindings: &KeyBindings, keys: &ButtonInput<KeyCode>, code: KeyCode) -> bool {
    let mods = held_mods(keys);
    key_name(code).is_some_and(|key| bindings.0.lookup(&Combo { mods, key: key.into() }).is_some())
}

/// When the things under the player change: one log line, and on a new square the
/// parchment box too if the "every new square" setting is on.
pub fn watch_ground(
    inv: Res<Inv>,
    time: Res<Time>,
    settings: Res<crate::graphics::CurrentSettings>,
    mut msgs: ResMut<Msgs>,
    mut last: Local<Vec<u32>>,
) {
    let ground = ground_things(&inv.0);
    let tags = object_tags(&ground); // walking from floor to floor stays quiet
    if tags == *last {
        return;
    }
    if let Some(line) = standing_on(&ground) {
        msgs.0.push((0, 0), &line, time.elapsed_secs(), None);
    }
    if settings.0.ground_popup && new_square(&last, &tags) {
        msgs.0.boxed = ground_box(&ground);
    }
    *last = tags;
}

mod world_keys;
pub use world_keys::world_keys;

#[cfg(test)]
mod tests {
    use super::{binding_commands, key_code, key_name, may_send, wants_step};
    use bevy::prelude::KeyCode;
    use cfproto::{Inventory, SpellBook};
    use crossfir3d::slots::Kit;
    use crossfir3d::keys::{Binding, Combo, Mods};

    #[test]
    fn act_key_maps_every_row() {
        use crossfir3d::modes::KeyName as N;
        let rows = [
            (KeyCode::KeyA, N::A), (KeyCode::KeyE, N::E), (KeyCode::KeyG, N::G), (KeyCode::KeyD, N::D),
            (KeyCode::KeyX, N::X), (KeyCode::KeyI, N::I), (KeyCode::KeyR, N::R), (KeyCode::KeyM, N::M),
            (KeyCode::KeyC, N::C), (KeyCode::KeyT, N::T), (KeyCode::KeyO, N::O), (KeyCode::KeyV, N::V),
            (KeyCode::Space, N::Space), (KeyCode::Tab, N::Tab), (KeyCode::Escape, N::Escape), (KeyCode::Enter, N::Enter),
            (KeyCode::Digit1, N::Digit(1)), (KeyCode::Digit2, N::Digit(2)), (KeyCode::Digit3, N::Digit(3)), (KeyCode::Digit4, N::Digit(4)),
            (KeyCode::KeyZ, N::Other), (KeyCode::KeyK, N::Other), (KeyCode::KeyB, N::Other), (KeyCode::Digit5, N::Other), (KeyCode::Digit0, N::Other), (KeyCode::F1, N::Other),
        ];
        for (k, n) in rows {
            assert_eq!(super::act_key(k), n, "{k:?}");
        }
    }

    #[test]
    fn compass_keys_walk_by_the_compass() {
        let dir = |k| super::COMPASS_KEYS.iter().find(|(c, _)| *c == k).map(|(_, s)| crossfir3d::camera::compass(*s, 0));
        let rows = [
            (KeyCode::KeyK, "north"), (KeyCode::KeyU, "northeast"), (KeyCode::KeyL, "east"), (KeyCode::KeyN, "southeast"),
            (KeyCode::KeyJ, "south"), (KeyCode::KeyB, "southwest"), (KeyCode::KeyH, "west"), (KeyCode::KeyY, "northwest"),
            (KeyCode::Numpad8, "north"), (KeyCode::Numpad9, "northeast"), (KeyCode::Numpad6, "east"), (KeyCode::Numpad3, "southeast"),
            (KeyCode::Numpad2, "south"), (KeyCode::Numpad1, "southwest"), (KeyCode::Numpad4, "west"), (KeyCode::Numpad7, "northwest"),
        ];
        for (k, want) in rows {
            assert_eq!(dir(k), Some(want), "{k:?}");
        }
    }

    #[test]
    fn modified_bound_key_is_taken() {
        use bevy::input::ButtonInput;
        let mut b = crate::KeyBindings::default();
        let shift = Mods { shift: true, ..Default::default() };
        b.0.bind(Binding { all: false, combo: Combo { mods: shift, key: "Up".into() }, edit: false, command: "say up".into() });
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ArrowUp);
        assert!(!super::key_taken(&b, &keys, KeyCode::ArrowUp)); // plain Up still walks
        keys.press(KeyCode::ShiftLeft);
        assert!(super::key_taken(&b, &keys, KeyCode::ArrowUp)); // shift+Up runs the binding only
        assert!(!super::key_taken(&b, &keys, KeyCode::ArrowDown));
    }

    #[test]
    fn key_names_roundtrip() {
        for k in [KeyCode::KeyG, KeyCode::Digit6, KeyCode::F3, KeyCode::ArrowUp, KeyCode::Slash] {
            assert_eq!(key_code(key_name(k).unwrap()), Some(k));
        }
        assert_eq!(key_name(KeyCode::ShiftLeft), None);
    }

    #[test]
    fn binding_beats_game_key() {
        let bind = |command: &str| Binding { all: false, combo: Combo { mods: Mods::default(), key: "G".into() }, edit: false, command: command.into() };
        let (inv, spells) = (Inventory::default(), SpellBook::default());
        assert_eq!(binding_commands(&bind("say hi"), &Kit { inv: &inv, spells: &spells, facing: 1, range: "" }, &[]), ["say hi"]);
        assert_eq!(
            binding_commands(&bind("use_skill sense curse; use_skill sense magic;"), &Kit { inv: &inv, spells: &spells, facing: 1, range: "" }, &[]),
            ["use_skill sense curse", "use_skill sense magic"]
        );
        let skills = ["praying".to_string()];
        assert_eq!(binding_commands(&bind("use_skill Praying"), &Kit { inv: &inv, spells: &spells, facing: 1, range: "" }, &skills), ["use_skill praying"]);
    }

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
