//! Arrow keys to ncom move commands, and one-key actions.

use bevy::prelude::*;
use cfproto::{ClientMsg, Inventory, SpellBook};

use maccrossfire::camera::{compass, heading_of, nearest_dir, screen_dir, Heading, CREATURE_LAYERS};
use maccrossfire::modes::{act, bindings_active, Act, KeyName, Mode};
use maccrossfire::anim::event_for_command;
use maccrossfire::messages::{ground_box, ground_things, new_square, object_tags, standing_on};
use maccrossfire::keys::{action_of, skill_command, Action, Binding, Combo, Mods};
use maccrossfire::slots::{dir_number, fire_commands, slot_commands, Slot};

use crate::characters3d::PlayerAnimEvents;

use crate::ui::{CommandLine, PendingBind};
use crate::{known_skills, HudState, Inv, KeyBindings, KeyMode, SkillNames, LastAction, LastComc, Msgs, NcomSeq, PlayerStats, Session, Slots, Spells, View, ViewSize};

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
    /// MACCROSSFIRE_TOP: start in the top view (screenshots).
    fn default() -> Self {
        CameraView { top: std::env::var_os("MACCROSSFIRE_TOP").is_some(), face: false }
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
    (bindings, pending): (Res<KeyBindings>, Res<PendingBind>),
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    if pending.waiting_key() {
        return;
    }
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
    // An arrow a binding took no longer moves.
    let key = |k| (keys.pressed(k) || keys.just_pressed(k)) && !key_taken(&bindings, &keys, k);
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

fn act_key(key: KeyCode) -> KeyName {
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
        KeyCode::KeyK => KeyName::K,
        KeyCode::KeyB => KeyName::B,
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

/// Key names used by bindings (`maccrossfire::keys`), by key position.
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
pub fn binding_commands(b: &Binding, inv: &Inventory, spells: &SpellBook, skills: &[String], facing: u8, range: &str) -> Vec<String> {
    let names: Vec<String> = spells.all().iter().map(|s| s.name.clone()).collect();
    match action_of(&b.command, &names, skills) {
        Action::Spell(n) => slot_commands(&Slot::Spell(n), inv, spells, facing, range),
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
        msgs.0.push(0, 0, &line, time.elapsed_secs(), None);
    }
    if settings.0.ground_popup && new_square(&last, &tags) {
        msgs.0.boxed = ground_box(&ground);
    }
    *last = tags;
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
    (bindings, skill_names, mut line, pending): (Res<KeyBindings>, Res<SkillNames>, ResMut<CommandLine>, Res<PendingBind>),
    session: Option<NonSend<Session>>,
) {
    let Some(session) = session else { return };
    if pending.waiting_key() {
        return; // `capture_bind_key` takes this key press
    }
    view.face = mode.0 == Mode::World && keys.pressed(KeyCode::KeyF) && !key_taken(&bindings, &keys, KeyCode::KeyF);
    let mods = held_mods(&keys);
    let pressed: Vec<KeyCode> = keys.get_just_pressed().copied().collect();
    for code in pressed {
        let bound = key_name(code)
            .filter(|_| bindings_active(mode.0))
            .and_then(|key| bindings.0.lookup(&Combo { mods, key: key.into() }))
            .cloned();
        if let Some(b) = bound {
            if b.edit {
                line.open = true;
                line.text = b.command.clone();
                mode.0 = Mode::CommandLine;
                continue;
            }
            last.0 = Some(time.elapsed_secs());
            let skills = known_skills(&stats.0, &skill_names);
            for c in binding_commands(&b, &inv.0, &spells.0, &skills, facing.0, &stats.0.range) {
                anim.0.extend(event_for_command(&c));
                session.send([ClientMsg::Ncom { packet: seq.next(), repeat: 0, command: c }]);
            }
            continue;
        }
        let key = act_key(code);
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
            // shift+K and the like are picked as keys on the bindings screen, not closing it.
            Act::CloseKeys if !mods.none() => {}
            Act::CloseInventory | Act::CloseSettings | Act::CloseKeys => mode.0 = Mode::World,
            Act::OpenKeys => mode.0 = Mode::Keys,
            Act::ShowGround => match ground_box(&ground_things(&inv.0)) {
                Some(text) => msgs.0.boxed = Some(text),
                None => msgs.0.push(0, 0, "There is nothing under you.", time.elapsed_secs(), None),
            },
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
    use super::{binding_commands, key_code, key_name, may_send, wants_step};
    use bevy::prelude::KeyCode;
    use cfproto::{Inventory, SpellBook};
    use maccrossfire::keys::{Binding, Combo, Mods};

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
        assert_eq!(binding_commands(&bind("say hi"), &inv, &spells, &[], 1, ""), ["say hi"]);
        assert_eq!(
            binding_commands(&bind("use_skill sense curse; use_skill sense magic;"), &inv, &spells, &[], 1, ""),
            ["use_skill sense curse", "use_skill sense magic"]
        );
        let skills = ["praying".to_string()];
        assert_eq!(binding_commands(&bind("use_skill Praying"), &inv, &spells, &skills, 1, ""), ["use_skill praying"]);
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
