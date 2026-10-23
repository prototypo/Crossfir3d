//! Clicks and button presses: selecting, binding, unbinding, and the Replace or Cancel question.

use super::*;

/// Applies what was typed and clicked while drawing.
pub(super) fn apply_clicks(sel: &mut KeysSel, out: Out, list: &[Binding], keys: &ButtonInput<KeyCode>, known: &Known) {
    (sel.new_command, sel.typing) = (out.new_text, out.typing);
    if let Some(a) = out.typed_action {
        sel.action = Some(a);
        sel.status = "Now pick keys, then press Bind.".into();
    }
    if let Some(a) = out.clicked_action {
        sel.action = if sel.action.as_ref() == Some(&a) { None } else { Some(a) };
    }
    match out.clicked_cap {
        Some(Cap::Mod(i)) => match &mut sel.picked {
            Some(c) => toggle(&mut c.mods, i),
            None => toggle(&mut sel.mods, i),
        },
        Some(Cap::Key(k)) => click_key(sel, k, list, keys, known),
        Some(Cap::Off) | None => {}
    }
}

/// A click on a key: selects what a bound key runs (if nothing is selected), else picks the key.
fn click_key(sel: &mut KeysSel, k: &'static str, list: &[Binding], keys: &ButtonInput<KeyCode>, known: &Known) {
    let mods = held_mods(keys);
    let mods = if mods.none() { sel.mods } else { mods };
    let here = binding_for_click(list, k, mods).map(|b| action_of(&b.command, &known.spells, &known.skills));
    match (&sel.action, here) {
        // Clicking a bound key with nothing selected selects what it runs.
        (None, Some(action)) if sel.picked.is_none() => sel.action = Some(action),
        _ => {
            let mods = sel.picked.as_ref().map_or(sel.mods, |c| c.mods);
            sel.picked = Some(Combo { mods, key: k.into() });
        }
    }
}

/// Ends the current selection and question, and says `status`.
fn done(sel: &mut KeysSel, status: String) {
    (sel.action, sel.picked, sel.mods, sel.ask) = (None, None, Mods::default(), None);
    sel.status = status;
}

/// Saves the key bindings for the logged-in character, if there is one.
fn save(bindings: &KeyBindings, session: Option<&Session>) {
    if let Some(s) = session
        && let Err(e) = save_bindings(&default_keys_path(), s.login.character(), &bindings.0)
    {
        warn!("could not save key bindings: {e}");
    }
}

/// Removes the selected action's bindings (if `remove`), adds `binding` (if any), and saves.
fn apply(sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env, remove: bool, binding: Option<Binding>) {
    let Some(action) = sel.action.clone() else { return };
    if remove {
        let drop: Vec<Binding> = bindings_of(&bindings.0.list, &action, &env.known.spells, &env.known.skills).into_iter().cloned().collect();
        bindings.0.list.retain(|b| !drop.contains(b));
    }
    let status = match &binding {
        Some(b) => format!("Bound {} to {}.", b.combo.text(MAC), action_name(&action)),
        None => format!("Unbound {}.", action_name(&action)),
    };
    if let Some(b) = binding {
        bindings.0.bind(b);
    }
    save(bindings, env.session);
    done(sel, status);
}

/// Handles a pressed button (0 Bind, 1 Change, 2 Unbind, 3 Clear); true means stop the frame here.
pub(super) fn handle_press(i: usize, sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env) -> bool {
    let action = sel.action.clone();
    let shared = action.as_ref().is_some_and(|a| is_shared(&bindings.0.list, a, env.known));
    match (i, action) {
        (3, _) => done(sel, "Selection cleared.".into()),
        (2, Some(_)) if shared => sel.ask = Some(("This removes it for all your characters.".into(), Ask::Change { remove: true, binding: None })),
        (2, Some(_)) => apply(sel, bindings, env, true, None),
        (0 | 1, Some(action)) => return bind_press(sel, bindings, env, i == 1, action),
        _ => {}
    }
    false
}

/// Bind or Change: applies the picked key now, or asks first; true means stop the frame here.
fn bind_press(sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env, remove: bool, action: Action) -> bool {
    let Some(combo) = sel.picked.clone() else { return true };
    let binding = Binding { all: false, combo, edit: false, command: command_of(&action) };
    let mut questions = Vec::new();
    match bind_question(&bindings.0, &binding.combo, &binding.command, MAC) {
        Err(reason) => {
            sel.status = format!("{} can't be bound: {reason}.", binding.combo.text(MAC));
            return true;
        }
        Ok(q) => questions.extend(q.map(|q| q.replace(" Type y or n.", ""))),
    }
    if remove && is_shared(&bindings.0.list, &action, env.known) {
        questions.push("This removes it for all your characters.".into());
    }
    if questions.is_empty() {
        apply(sel, bindings, env, remove, Some(binding));
    } else {
        sel.ask = Some((questions.join("\n"), Ask::Change { remove, binding: Some(binding) }));
    }
    false
}

/// The Replace or Cancel question, if one is waiting.
pub(super) fn ask_modal(ctx: &egui::Context, sel: &mut KeysSel, bindings: &mut KeyBindings, env: &Env) {
    let Some((question, _)) = &sel.ask else { return };
    match modal_answer(ctx, question.clone()) {
        Some(true) => {
            if let Some((_, Ask::Change { remove, binding })) = sel.ask.take() {
                apply(sel, bindings, env, remove, binding);
            }
        }
        Some(false) => done(sel, "Not changed.".into()),
        None => {}
    }
}

/// Shows the question with Replace and Cancel; Some(true) for Replace, Some(false) for Cancel.
fn modal_answer(ctx: &egui::Context, question: String) -> Option<bool> {
    let mut answer = None;
    egui::Modal::new(egui::Id::new("keys ask")).show(ctx, |ui| {
        ui.label(question);
        ui.horizontal(|ui| {
            if ui.button("Replace").clicked() {
                answer = Some(true);
            }
            if ui.button("Cancel").clicked() {
                answer = Some(false);
            }
        });
    });
    answer
}
