//! The buttons, the SPELLS, SKILLS and OTHER lists, and the New command box.

use super::*;

/// Draws the Bind, Change, Unbind and Clear selection buttons and the status.
pub(super) fn draw_buttons(ui: &mut egui::Ui, v: &View, out: &mut Out) {
    ui.horizontal(|ui| {
        let [can_bind, can_change, can_unbind, can_clear] = v.buttons;
        for (i, (name, on)) in [("Bind", can_bind), ("Change", can_change), ("Unbind", can_unbind), ("Clear selection", can_clear)].into_iter().enumerate() {
            if ui.add_enabled(on && v.sel.ask.is_none(), egui::Button::new(name)).clicked() {
                out.pressed = Some(i);
            }
        }
        ui.label(&v.sel.status);
    });
}

/// Draws the SPELLS, SKILLS and OTHER columns.
pub(super) fn draw_lists(ui: &mut egui::Ui, v: &View, out: &mut Out, lists_w: f32) {
    let w = lists_w / 3.0;
    ui.vertical(|ui| {
        ui.set_width(w);
        heading(ui, "SPELLS");
        scroll_list(ui, v, &mut out.clicked_action, "spells", v.known.spells.iter().map(|n| Action::Spell(n.clone())).collect());
    });
    ui.vertical(|ui| {
        ui.set_width(w);
        heading(ui, "SKILLS");
        scroll_list(ui, v, &mut out.clicked_action, "skills", v.known.skills.iter().map(|n| Action::Skill(n.clone())).collect());
    });
    ui.vertical(|ui| {
        ui.set_width(w);
        heading(ui, "OTHER");
        scroll_list(ui, v, &mut out.clicked_action, "other", other_actions(v));
        new_command_box(ui, v, out);
    });
}

fn heading(ui: &mut egui::Ui, t: &str) {
    ui.label(egui::RichText::new(t).color(GOLD).strong());
}

/// A scrolling list of actions, each with its keys; a click is noted in `clicked`.
fn scroll_list(ui: &mut egui::Ui, v: &View, clicked: &mut Option<Action>, salt: &str, actions: Vec<Action>) {
    egui::ScrollArea::vertical().id_salt(salt).show(ui, |ui| {
        for action in actions {
            action_row(ui, v, action, clicked);
        }
    });
}

/// One selectable line: the action's name and the keys bound to it.
fn action_row(ui: &mut egui::Ui, v: &View, action: Action, clicked: &mut Option<Action>) {
    let combos: Vec<String> = bindings_of(v.list, &action, &v.known.spells, &v.known.skills).iter().map(|b| b.combo.text(MAC)).collect();
    let text = format!("{}   {}", action_name(&action), combos.join(", "));
    if ui.selectable_label(v.sel.action.as_ref() == Some(&action), text).clicked() {
        *clicked = Some(action);
    }
}

/// The OTHER list; a new command typed below shows here, selected, until it is bound.
fn other_actions(v: &View) -> Vec<Action> {
    let mut others = other_commands(v.list, &v.known.spells, &v.known.skills);
    if let Some(Action::Command(c)) = &v.sel.action
        && !others.contains(c)
    {
        others.insert(0, c.clone());
    }
    others.into_iter().map(Action::Command).collect()
}

/// The "New command" text box under OTHER; Enter selects what was typed.
fn new_command_box(ui: &mut egui::Ui, v: &View, out: &mut Out) {
    ui.add_space(8.0);
    ui.label(egui::RichText::new("New command (Enter selects it):").small());
    let edit = ui.add(egui::TextEdit::singleline(&mut out.new_text).hint_text("say hello"));
    out.typing = edit.has_focus();
    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        out.typed_action = new_command(&out.new_text, &v.known.spells, &v.known.skills);
        out.new_text.clear();
    }
}
