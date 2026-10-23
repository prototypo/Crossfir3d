//! Tests for the key bindings screen.

use super::{binding_for_click, buttons, new_command, other_commands, KeysSel};
use crossfir3d::keys::{Action, Binding, Combo, Mods};

#[test]
fn typed_new_command_becomes_the_action() {
    let spells = ["burning hands".to_string()];
    assert_eq!(new_command("  say hello ", &spells, &[]), Some(Action::Command("say hello".into())));
    assert_eq!(new_command("cast Burning Hands", &spells, &[]), Some(Action::Spell("burning hands".into())));
    assert_eq!(new_command("   ", &spells, &[]), None);
}

#[test]
fn other_list_has_each_plain_command_once() {
    let b = |key: &str, command: &str| Binding { all: false, combo: Combo { mods: Mods::default(), key: key.into() }, edit: false, command: command.into() };
    let list = [b("Q", "say hi"), b("W", "cast burning hands"), b("E", "use_skill praying"), b("R", "say hi"), b("T", "cast fireball")];
    let (spells, skills) = (["burning hands".to_string()], ["praying".to_string()]);
    assert_eq!(other_commands(&list, &spells, &skills), ["cast fireball", "say hi"]);
}

#[test]
fn click_picks_the_binding_with_the_held_modifiers() {
    let b = |mods: Mods, command: &str| Binding { all: false, combo: Combo { mods, key: "G".into() }, edit: false, command: command.into() };
    let shift_cmd = Mods { shift: true, cmd: true, ..Default::default() };
    let list = [b(Mods::default(), "say plain"), b(shift_cmd, "say shift cmd")];
    assert_eq!(binding_for_click(&list, "G", shift_cmd).unwrap().command, "say shift cmd");
    assert_eq!(binding_for_click(&list, "G", Mods::default()).unwrap().command, "say plain");
    assert_eq!(binding_for_click(&list[1..], "G", Mods::default()).unwrap().command, "say shift cmd");
    assert!(binding_for_click(&list, "H", Mods::default()).is_none());
}

#[test]
fn reset_clears_all_but_labels() {
    let mut s = KeysSel { action: Some(Action::Command("x".into())), status: "y".into(), new_command: "z".into(), typing: true, ..Default::default() };
    s.labels.insert("Q", "a".into());
    s.reset();
    assert!(s.action.is_none() && s.status.is_empty() && s.new_command.is_empty() && !s.typing);
    assert_eq!(s.labels.get("Q").map(String::as_str), Some("a"));
}

#[test]
fn buttons_follow_input() {
    let mut s = KeysSel::default();
    assert_eq!(buttons(&s, false), [false, false, false, false]);
    s.action = Some(Action::Spell("burning hands".into()));
    assert_eq!(buttons(&s, false), [false, false, false, true]);
    s.picked = Some(Combo { mods: Mods { alt: true, ..Default::default() }, key: "6".into() });
    assert_eq!(buttons(&s, false), [true, false, false, true]);
    assert_eq!(buttons(&s, true), [false, true, true, true]);
}
