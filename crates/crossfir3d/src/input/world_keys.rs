//! The one-key actions system and its helpers.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use cfproto::ClientMsg;

use super::{act_key, binding_commands, fire_dir, held_mods, key_name, key_taken, shift_only, CameraHeading, CameraView, Facing};
use crate::characters3d::PlayerAnimEvents;
use crate::ui::{CommandLine, PendingBind};
use crate::{known_skills, HudState, Inv, KeyBindings, KeyMode, LastAction, Msgs, NcomSeq, PlayerStats, Session, SkillNames, Slots, Spells, View, ViewSize};
use crossfir3d::anim::event_for_command;
use crossfir3d::camera::{heading_of, nearest_dir, CREATURE_LAYERS};
use crossfir3d::keys::{Binding, Combo, Mods};
use crossfir3d::messages::{ground_box, ground_things};
use crossfir3d::modes::{act, bindings_active, Act, KeyName, Mode};
use crossfir3d::slots::{dir_number, fire_commands, slot_commands, Kit};

/// Everything `world_keys` reads or changes in a frame.
#[derive(SystemParam)]
pub struct WorldKeyParams<'w> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    time: Res<'w, Time>,
    mode: ResMut<'w, KeyMode>,
    msgs: ResMut<'w, Msgs>,
    hud: ResMut<'w, HudState>,
    last: ResMut<'w, LastAction>,
    seq: ResMut<'w, NcomSeq>,
    facing: ResMut<'w, Facing>,
    slots: Res<'w, Slots>,
    inv: Res<'w, Inv>,
    spells: Res<'w, Spells>,
    stats: Res<'w, PlayerStats>,
    map: Res<'w, View>,
    size: Res<'w, ViewSize>,
    heading: ResMut<'w, CameraHeading>,
    view: ResMut<'w, CameraView>,
    anim: ResMut<'w, PlayerAnimEvents>,
    bindings: Res<'w, KeyBindings>,
    skill_names: Res<'w, SkillNames>,
    line: ResMut<'w, CommandLine>,
    pending: Res<'w, PendingBind>,
    keys_sel: ResMut<'w, crate::keys_ui::KeysSel>,
}

/// One-key actions (the spec's key table, via `modes::act`). Enter is handled by the
/// command line, M by the audio plugin.
pub fn world_keys(mut p: WorldKeyParams, session: Option<NonSend<Session>>) {
    let Some(session) = session else { return };
    if p.pending.waiting_key() {
        return; // `capture_bind_key` takes this key press
    }
    p.view.face = p.mode.0 == Mode::World && p.keys.pressed(KeyCode::KeyF) && !key_taken(&p.bindings, &p.keys, KeyCode::KeyF);
    let mods = held_mods(&p.keys);
    let pressed: Vec<KeyCode> = p.keys.get_just_pressed().copied().collect();
    for code in pressed {
        p.press(&session, code, mods);
    }
}

impl WorldKeyParams<'_> {
    /// The player's carried items, spells, facing and range, as slots and bindings read them.
    fn kit(&self) -> Kit<'_> {
        Kit { inv: &self.inv.0, spells: &self.spells.0, facing: self.facing.0, range: &self.stats.0.range }
    }

    /// One key press: a binding if one matches, else the key's own action.
    fn press(&mut self, session: &Session, code: KeyCode, mods: Mods) {
        if let Some(b) = self.bound(code, mods) {
            return self.run_binding(session, &b);
        }
        if self.mode.0 == Mode::World && shift_only(mods) && let Some(dir) = fire_dir(code, self.facing.0) {
            return self.fire(session, dir);
        }
        let key = act_key(code);
        let a = act(self.mode.0, key, self.msgs.0.boxed.is_some());
        if !self.screen_act(a, key, mods) {
            self.world_act(session, a);
        }
    }

    /// The binding the key press runs, if any.
    fn bound(&self, code: KeyCode, mods: Mods) -> Option<Binding> {
        key_name(code)
            .filter(|_| bindings_active(self.mode.0))
            .and_then(|key| self.bindings.0.lookup(&Combo { mods, key: key.into() }))
            .cloned()
    }

    /// Send one ncom command, noting it as the last action and for the player's animation.
    fn send(&mut self, session: &Session, c: &str) {
        self.last.0 = Some(self.time.elapsed_secs());
        self.anim.0.extend(event_for_command(c));
        session.send([ClientMsg::Ncom { packet: self.seq.next(), repeat: 0, command: c.into() }]);
    }

    /// A binding: open the command line on its text (edit), or send its commands.
    fn run_binding(&mut self, session: &Session, b: &Binding) {
        if b.edit {
            self.line.open = true;
            self.line.text = b.command.clone();
            self.mode.0 = Mode::CommandLine;
            return;
        }
        self.last.0 = Some(self.time.elapsed_secs());
        let skills = known_skills(&self.stats.0, &self.skill_names);
        for c in binding_commands(b, &self.kit(), &skills) {
            self.anim.0.extend(event_for_command(&c));
            session.send([ClientMsg::Ncom { packet: self.seq.next(), repeat: 0, command: c }]);
        }
    }

    /// Acts that open, close or change a screen or box; false for any other act.
    fn screen_act(&mut self, a: Act, key: KeyName, mods: Mods) -> bool {
        match a {
            Act::CloseBox => self.msgs.0.close_box(),
            Act::OpenInventory => self.mode.0 = Mode::Inventory,
            Act::CloseInventory | Act::CloseSettings => self.mode.0 = Mode::World,
            Act::ShowGround => match ground_box(&ground_things(&self.inv.0)) {
                Some(text) => self.msgs.0.boxed = Some(text),
                None => self.msgs.0.push((0, 0), "There is nothing under you.", self.time.elapsed_secs(), None),
            },
            Act::OpenSettings => self.mode.0 = Mode::Settings,
            Act::ToggleLog => self.hud.tall_log = !self.hud.tall_log,
            _ => return self.keys_act(a, key, mods),
        }
        true
    }

    /// Acts that open or close the bindings screen; false for any other act.
    fn keys_act(&mut self, a: Act, key: KeyName, mods: Mods) -> bool {
        match a {
            // shift+O and the like are picked as keys on the bindings screen, not closing it.
            Act::CloseKeys if !mods.none() || (key == KeyName::O && self.keys_sel.typing) => {}
            Act::CloseKeys => {
                self.keys_sel.reset();
                self.mode.0 = Mode::World;
            }
            Act::OpenKeys => {
                self.keys_sel.reset();
                self.mode.0 = Mode::Keys;
            }
            _ => return false,
        }
        true
    }

    /// Acts on the world: commands, quick slots, firing, camera and facing.
    fn world_act(&mut self, session: &Session, a: Act) {
        match a {
            Act::Apply => self.send(session, "apply"),
            Act::Get => self.send(session, "get"),
            Act::Slot(i) => {
                let Some(slot) = &self.slots.0 .0[i] else { return };
                for c in slot_commands(slot, &self.kit()) {
                    self.send(session, &c);
                }
            }
            Act::CameraBehind => {
                self.heading.0 = heading_of(self.facing.0);
                self.view.top = false;
            }
            Act::Fire => self.fire(session, self.facing.0),
            Act::FaceNearest => self.face_nearest(),
            Act::TopView => self.view.top = !self.view.top,
            _ => {}
        }
    }

    /// Fire the prepared spell or readied bow or wand toward `dir`, or say there is nothing to fire.
    fn fire(&mut self, session: &Session, dir: u8) {
        let fire = fire_commands(&self.inv.0, dir, &self.stats.0.range);
        if fire.is_empty() {
            let now = self.time.elapsed_secs();
            self.msgs.0.push((0, 0), "Nothing to fire: prepare a spell, or ready a bow or wand (I, select it, E).", now, None);
        }
        for c in &fire {
            self.send(session, c);
        }
    }

    /// Turn the player and the camera to the nearest creature.
    fn face_nearest(&mut self) {
        let (cx, cy) = (self.size.0 as i32 / 2, self.size.1 as i32 / 2);
        let creatures = self.map.0.cells().filter(|(_, c)| c.layers[CREATURE_LAYERS].iter().any(|f| *f != 0));
        if let Some(dir) = nearest_dir(creatures.map(|((x, y), _)| (x as i32 - cx, y as i32 - cy)), 4) {
            self.facing.0 = dir_number(dir).unwrap_or(self.facing.0);
            self.heading.0 = heading_of(self.facing.0);
        }
    }
}
