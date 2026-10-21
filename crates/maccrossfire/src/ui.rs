//! egui screens: the login form and the in-game command line.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cfproto::ClientMsg;
use maccrossfire::login::{Creds, LoginFlow};

use maccrossfire::modes::{command_line_allowed, Mode};

use maccrossfire::keys::{bind_question, parse_typed, save_bindings, default_keys_path, Binding, Combo, Typed};

use crate::input::{held_mods, key_name, key_taken};
use crate::{KeyBindings, KeyMode, LastAction, Msgs, NcomSeq, PendingQuery, Session, Status};

/// A typed `bind` waiting for its key, or a yes/no question waiting for its answer.
#[derive(Resource, Default)]
pub struct PendingBind(pub Option<BindStep>);

pub enum BindStep {
    WaitKey { all: bool, edit: bool, command: String },
    Confirm { binding: Binding },
    ConfirmReset,
}

impl PendingBind {
    pub fn waiting_key(&self) -> bool {
        matches!(self.0, Some(BindStep::WaitKey { .. }))
    }
}

const MAC: bool = cfg!(target_os = "macos");

fn log(msgs: &mut Msgs, text: &str, now: f32) {
    msgs.0.push(0, 0, text, now, None);
}

fn save(bindings: &KeyBindings, session: &Option<NonSend<Session>>, msgs: &mut Msgs, now: f32) {
    let Some(s) = session else { return };
    if let Err(e) = save_bindings(&default_keys_path(), s.login.character(), &bindings.0) {
        log(msgs, &format!("Could not save the key bindings: {e}"), now);
    }
}

/// A typed `bind`/`unbind` line, or the answer to a bind question. False: not ours,
/// send it to the server.
fn typed_bind(text: &str, pending: &mut PendingBind, bindings: &mut KeyBindings, msgs: &mut Msgs, now: f32) -> Option<bool> {
    if let Some(step) = pending.0.take() {
        let answer = text.trim().to_ascii_lowercase();
        match step {
            BindStep::Confirm { binding } if answer == "y" => {
                log(msgs, &format!("Bound {} to {}.", binding.combo.text(MAC), binding.command), now);
                bindings.0.bind(binding);
                return Some(true);
            }
            BindStep::ConfirmReset if answer == "y" => {
                bindings.0.reset();
                log(msgs, "Key bindings removed.", now);
                return Some(true);
            }
            BindStep::WaitKey { .. } => pending.0 = Some(step), // still waiting for the key
            _ => {
                log(msgs, "Not changed.", now);
                if answer == "n" {
                    return Some(false);
                }
                // Any other text drops the question and is handled as typed.
            }
        }
    }
    let changed = match parse_typed(text)? {
        Typed::Bind { all, edit, command } => {
            log(msgs, &format!("Press a key for: {command}"), now);
            pending.0 = Some(BindStep::WaitKey { all, edit, command });
            false
        }
        Typed::Special(_) => {
            log(msgs, "Not supported.", now);
            false
        }
        Typed::Usage => {
            log(msgs, "bind [-nfregc] <command>   unbind [<number>|reset]", now);
            false
        }
        Typed::List if bindings.0.list.is_empty() => {
            log(msgs, "No key bindings.", now);
            false
        }
        Typed::List => {
            for line in bindings.0.numbered(MAC) {
                log(msgs, &line, now);
            }
            false
        }
        Typed::Remove(n) => match n.checked_sub(1).and_then(|i| bindings.0.remove(i)) {
            Some(b) => {
                log(msgs, &format!("Unbound {}.", b.combo.text(MAC)), now);
                true
            }
            None => {
                log(msgs, &format!("No binding {n}."), now);
                false
            }
        },
        Typed::Reset => {
            log(msgs, "Remove all key bindings of this character? Type y or n.", now);
            pending.0 = Some(BindStep::ConfirmReset);
            false
        }
    };
    Some(changed)
}

/// After a typed `bind`: the next key pressed (with its modifiers) gets the command.
/// Runs after `world_keys`, which ignores that key press.
pub fn capture_bind_key(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut pending: ResMut<PendingBind>,
    mut bindings: ResMut<KeyBindings>,
    mut msgs: ResMut<Msgs>,
    session: Option<NonSend<Session>>,
) {
    let Some(BindStep::WaitKey { all, edit, command }) = &pending.0 else { return };
    let Some(key) = keys.get_just_pressed().find_map(|k| key_name(*k)) else { return };
    let now = time.elapsed_secs();
    let combo = Combo { mods: held_mods(&keys), key: key.into() };
    if key == "Escape" && combo.mods.none() {
        log(&mut msgs, "Cancelled.", now);
        pending.0 = None;
        return;
    }
    let binding = Binding { all: *all, combo, edit: *edit, command: command.clone() };
    match bind_question(&bindings.0, &binding.combo, &binding.command, MAC) {
        Err(reason) => {
            log(&mut msgs, &format!("{}: {reason}.", binding.combo.text(MAC)), now);
            pending.0 = None;
        }
        Ok(Some(question)) => {
            log(&mut msgs, &question, now);
            pending.0 = Some(BindStep::Confirm { binding });
        }
        Ok(None) => {
            log(&mut msgs, &format!("Bound {} to {}.", binding.combo.text(MAC), binding.command), now);
            bindings.0.bind(binding);
            pending.0 = None;
            save(&bindings, &session, &mut msgs, now);
        }
    }
}

#[derive(Resource)]
pub struct LoginForm {
    server: String,
    account: String,
    password: String,
    new_account: bool,
}

impl Default for LoginForm {
    fn default() -> Self {
        LoginForm {
            server: "localhost:13327".into(),
            account: String::new(),
            password: String::new(),
            new_account: false,
        }
    }
}

pub fn login_screen(
    mut contexts: EguiContexts,
    mut form: ResMut<LoginForm>,
    status: Res<Status>,
    mut settings_open: ResMut<crate::settings_ui::SettingsOpen>,
    mut commands: Commands,
    session: Option<NonSend<Session>>,
) -> Result {
    // Connected: the character screens show instead.
    if settings_open.0 || session.is_some() {
        return Ok(());
    }
    let mut connect = false;
    egui::Window::new("MacCrossfire").collapsible(false).resizable(false).show(contexts.ctx_mut()?, |ui| {
        egui::Grid::new("login").num_columns(2).show(ui, |ui| {
            ui.label("Server");
            ui.text_edit_singleline(&mut form.server);
            ui.end_row();
            ui.label("Account");
            ui.text_edit_singleline(&mut form.account);
            ui.end_row();
            ui.label("Password");
            ui.add(egui::TextEdit::singleline(&mut form.password).password(true));
            ui.end_row();
        });
        ui.checkbox(&mut form.new_account, "New account");
        ui.horizontal(|ui| {
            connect = ui.button("Connect").clicked();
            if ui.button("Settings").clicked() {
                settings_open.0 = true;
            }
        });
        if !status.0.is_empty() {
            ui.label(&status.0);
        }
    });
    if connect {
        let creds = Creds {
            account: form.account.clone(),
            password: form.password.clone(),
            new_account: form.new_account,
            quick_character: None,
        };
        queue_connect(&mut commands, form.server.clone(), creds);
    }
    Ok(())
}

fn queue_connect(commands: &mut Commands, server: String, creds: Creds) {
    commands.queue(move |world: &mut World| {
        // A second click while connected would leave the first character in the game.
        if world.get_non_send::<Session>().is_some() {
            return;
        }
        connect(world, &server, creds)
    });
}

fn connect(world: &mut World, server: &str, creds: Creds) {
    match maccrossfire::net::connect(server) {
        Ok(net) => {
            let (login, first) = LoginFlow::start(creds);
            world.resource_mut::<Status>().0 = "Connecting...".into();
            crate::start_session(world, net, login, first);
        }
        Err(e) => world.resource_mut::<Status>().0 = format!("Cannot connect to {server}: {e}"),
    }
}

/// Developer shortcut: MACCROSSFIRE_LOGIN="account:password:character[:new]" logs in to
/// localhost at startup; with an empty character it stops at the character list.
pub fn auto_login(mut commands: Commands) {
    let Ok(v) = std::env::var("MACCROSSFIRE_LOGIN") else { return };
    let parts: Vec<&str> = v.split(':').collect();
    let [account, password, character, rest @ ..] = parts.as_slice() else {
        warn!("MACCROSSFIRE_LOGIN must be account:password:character[:new]");
        return;
    };
    let creds = Creds {
        account: account.to_string(),
        password: password.to_string(),
        new_account: rest.first() == Some(&"new"),
        // An empty character name stops at the character list.
        quick_character: (!character.is_empty()).then(|| character.to_string()),
    };
    queue_connect(&mut commands, LoginForm::default().server, creds);
}

#[derive(Resource, Default)]
pub struct CommandLine {
    pub open: bool,
    pub text: String,
}

#[derive(Debug, PartialEq)]
pub enum LineAction {
    Open,
    Submit,
    Cancel,
    Nothing,
}

/// Enter opens the box and, when open, sends it. Escape closes it without sending.
/// Uses Bevy's key state: egui's text box swallows its own Enter key event.
pub fn line_action(open: bool, enter: bool, escape: bool) -> LineAction {
    match (open, enter, escape) {
        (false, true, _) => LineAction::Open,
        (true, true, _) => LineAction::Submit,
        (true, false, true) => LineAction::Cancel,
        _ => LineAction::Nothing,
    }
}

/// Enter opens a one-line box; Enter again sends the text as a command; Escape cancels.
pub fn command_line(
    mut contexts: EguiContexts,
    mut line: ResMut<CommandLine>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut mode: ResMut<KeyMode>,
    mut last: ResMut<LastAction>,
    mut seq: ResMut<NcomSeq>,
    mut query: ResMut<PendingQuery>,
    mut anim: ResMut<crate::characters3d::PlayerAnimEvents>,
    (mut pending, mut bindings, mut msgs): (ResMut<PendingBind>, ResMut<KeyBindings>, ResMut<Msgs>),
    session: Option<NonSend<Session>>,
) -> Result {
    if !command_line_allowed(mode.0) {
        return Ok(());
    }
    // A bound shift+Enter (or the like) runs its binding, not the command box.
    let enter = keys.just_pressed(KeyCode::Enter) && (line.open || !key_taken(&bindings, &keys, KeyCode::Enter));
    match line_action(line.open, enter, keys.just_pressed(KeyCode::Escape)) {
        LineAction::Open => {
            line.open = true;
            mode.0 = Mode::CommandLine;
        }
        LineAction::Cancel => {
            line.open = false;
            line.text.clear();
            mode.0 = Mode::World;
        }
        LineAction::Submit => {
            let command = std::mem::take(&mut line.text);
            line.open = false;
            mode.0 = Mode::World;
            let now = time.elapsed_secs();
            if let Some(changed) = typed_bind(&command, &mut pending, &mut bindings, &mut msgs, now) {
                if changed {
                    save(&bindings, &session, &mut msgs, now);
                }
                return Ok(());
            }
            last.0 = Some(now);
            if let (Some(session), false) = (session, command.trim().is_empty()) {
                anim.0.extend(maccrossfire::anim::event_for_command(command.trim()));
                session.send([submit_message(command, query.0, seq.next())]);
                query.0 = false;
            }
        }
        LineAction::Nothing => {}
    }
    if line.open {
        let bottom = egui::Window::new("command").title_bar(false).resizable(false);
        bottom.anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0]).show(contexts.ctx_mut()?, |ui| {
            ui.text_edit_singleline(&mut line.text).request_focus();
        });
    }
    Ok(())
}

/// The command box text as a message: an answer if the server asked a question.
pub fn submit_message(text: String, pending_query: bool, packet: u16) -> ClientMsg {
    if pending_query {
        ClientMsg::Reply(text)
    } else {
        ClientMsg::Ncom { packet, repeat: 0, command: text }
    }
}

#[cfg(test)]
mod tests {
    use super::{line_action, submit_message, typed_bind, BindStep, LineAction, PendingBind};
    use crate::{KeyBindings, Msgs};
    use maccrossfire::keys::{Binding, Combo, Mods};

    #[test]
    fn other_text_drops_a_waiting_question() {
        let (mut p, mut b, mut m) = (PendingBind::default(), KeyBindings::default(), Msgs::default());
        let g = Binding { all: false, combo: Combo { mods: Mods::default(), key: "G".into() }, edit: false, command: "say g".into() };
        p.0 = Some(BindStep::Confirm { binding: g });
        assert_eq!(typed_bind("shout help", &mut p, &mut b, &mut m, 0.0), None); // goes to the server
        assert!(p.0.is_none());
        assert!(b.0.list.is_empty());
        p.0 = Some(BindStep::ConfirmReset);
        assert_eq!(typed_bind("unbind", &mut p, &mut b, &mut m, 0.0), Some(false)); // handled as typed
        assert!(p.0.is_none());
    }

    #[test]
    fn typed_bind_flow() {
        let (mut p, mut b, mut m) = (PendingBind::default(), KeyBindings::default(), Msgs::default());
        assert_eq!(typed_bind("say hi", &mut p, &mut b, &mut m, 0.0), None);
        assert_eq!(typed_bind("bind say hello", &mut p, &mut b, &mut m, 0.0), Some(false));
        assert!(p.waiting_key());
        assert_eq!(m.0.log.back().map(String::as_str), Some("Press a key for: say hello"));
        // A replace question answered n changes nothing and is not sent to the server.
        let g = Binding { all: false, combo: Combo { mods: Mods::default(), key: "G".into() }, edit: false, command: "say g".into() };
        p.0 = Some(BindStep::Confirm { binding: g.clone() });
        assert_eq!(typed_bind("n", &mut p, &mut b, &mut m, 0.0), Some(false));
        assert!(b.0.list.is_empty());
        p.0 = Some(BindStep::Confirm { binding: g });
        assert_eq!(typed_bind("y", &mut p, &mut b, &mut m, 0.0), Some(true));
        assert_eq!(b.0.numbered(true), ["1. G  say g"]);
        assert_eq!(typed_bind("unbind 2", &mut p, &mut b, &mut m, 0.0), Some(false));
        assert_eq!(typed_bind("unbind 1", &mut p, &mut b, &mut m, 0.0), Some(true));
        assert!(b.0.list.is_empty());
    }
    use cfproto::ClientMsg;

    #[test]
    fn answer_goes_as_reply_after_query() {
        assert_eq!(submit_message("y".into(), true, 3), ClientMsg::Reply("y".into()));
        assert_eq!(submit_message("north".into(), false, 3), ClientMsg::Ncom { packet: 3, repeat: 0, command: "north".into() });
    }

    #[test]
    fn enter_opens_then_submits() {
        assert_eq!(line_action(false, true, false), LineAction::Open);
        assert_eq!(line_action(true, true, false), LineAction::Submit);
    }

    #[test]
    fn escape_cancels_only_when_open() {
        assert_eq!(line_action(true, false, true), LineAction::Cancel);
        assert_eq!(line_action(false, false, true), LineAction::Nothing);
    }

    #[test]
    fn no_keys_nothing() {
        assert_eq!(line_action(true, false, false), LineAction::Nothing);
        assert_eq!(line_action(false, false, false), LineAction::Nothing);
    }
}
