//! egui screens: the login form and the in-game command line.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cfproto::ClientMsg;
use crossfir3d::login::{Creds, LoginFlow};

use crossfir3d::modes::{command_line_allowed, Mode};

use crossfir3d::keys::{shadowed, bind_question, parse_typed, save_bindings, default_keys_path, Binding, Combo, Typed};

use crate::input::{held_mods, key_name, key_taken};
use crate::{KeyBindings, KeyMode, LastAction, Msgs, NcomSeq, PendingQuery, Session, Status};

/// The gold of overlay borders and headings.
pub const GOLD: egui::Color32 = egui::Color32::from_rgb(0xc9, 0xa2, 0x4a);

/// A full-screen overlay window: no title bar, fixed in place, dark with a gold border.
pub fn overlay_window(id: &str, screen: egui::Rect, margin: f32) -> egui::Window<'static> {
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(20, 22, 30, 238))
        .stroke(egui::Stroke::new(2.0, GOLD))
        .corner_radius(8.0)
        .inner_margin(margin);
    egui::Window::new(id.to_owned()).title_bar(false).resizable(false).movable(false).fixed_rect(screen).frame(frame)
}

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

/// "Bound g to say hi.", with a warning when this character's own binding hides it.
fn bound_message(b: &crossfir3d::keys::Bindings, binding: &Binding) -> String {
    let keys = binding.combo.text(MAC);
    let mut text = format!("Bound {keys} to {}.", binding.command);
    if shadowed(b, binding) {
        text += &format!(" This character's own binding on {keys} still wins (unbind it to use this one).");
    }
    text
}

fn log(msgs: &mut Msgs, text: &str, now: f32) {
    msgs.0.push((0, 0), text, now, None);
}

fn save(bindings: &KeyBindings, session: &Option<NonSend<Session>>, msgs: &mut Msgs, now: f32) {
    let Some(s) = session else { return };
    if let Err(e) = save_bindings(&default_keys_path(), s.login.character(), &bindings.0) {
        log(msgs, &format!("Could not save the key bindings: {e}"), now);
    }
}

/// Answers a pending bind question: Some(changed) when the text was the answer, None when it is handled as typed.
fn answer_pending(text: &str, pending: &mut PendingBind, bindings: &mut KeyBindings, msgs: &mut Msgs, now: f32) -> Option<bool> {
    let step = pending.0.take()?;
    let answer = text.trim().to_ascii_lowercase();
    match step {
        BindStep::Confirm { binding } if answer == "y" => {
            log(msgs, &bound_message(&bindings.0, &binding), now);
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
    None
}

/// Logs the numbered key bindings, or that there are none.
fn list_bindings(bindings: &KeyBindings, msgs: &mut Msgs, now: f32) {
    if bindings.0.list.is_empty() {
        log(msgs, "No key bindings.", now);
        return;
    }
    for line in bindings.0.numbered(MAC) {
        log(msgs, &line, now);
    }
}

/// Removes the 1-based binding `n` and says so; true when one was removed.
fn remove_binding(n: usize, bindings: &mut KeyBindings, msgs: &mut Msgs, now: f32) -> bool {
    match n.checked_sub(1).and_then(|i| bindings.0.remove(i)) {
        Some(b) => {
            log(msgs, &format!("Unbound {}.", b.combo.text(MAC)), now);
            true
        }
        None => {
            log(msgs, &format!("No binding {n}."), now);
            false
        }
    }
}

/// Runs one parsed `bind`/`unbind` command; true when the bindings changed.
fn run_typed(typed: Typed, pending: &mut PendingBind, bindings: &mut KeyBindings, msgs: &mut Msgs, now: f32) -> bool {
    match typed {
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
        Typed::List => {
            list_bindings(bindings, msgs, now);
            false
        }
        Typed::Remove(n) => remove_binding(n, bindings, msgs, now),
        Typed::Reset => {
            log(msgs, "Remove all key bindings of this character? Type y or n.", now);
            pending.0 = Some(BindStep::ConfirmReset);
            false
        }
    }
}

/// A typed `bind`/`unbind` line, or the answer to a bind question. False: not ours,
/// send it to the server.
fn typed_bind(text: &str, pending: &mut PendingBind, bindings: &mut KeyBindings, msgs: &mut Msgs, now: f32) -> Option<bool> {
    if let Some(changed) = answer_pending(text, pending, bindings, msgs, now) {
        return Some(changed);
    }
    Some(run_typed(parse_typed(text)?, pending, bindings, msgs, now))
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
            log(&mut msgs, &bound_message(&bindings.0, &binding), now);
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

/// Draws the login window and starts the connection.
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
    egui::Window::new("Crossfir3d").collapsible(false).resizable(false).show(contexts.ctx_mut()?, |ui| {
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

/// Opens the connection and starts the login flow, or shows why it failed.
fn connect(world: &mut World, server: &str, creds: Creds) {
    match crossfir3d::net::connect(server) {
        Ok(net) => {
            let (login, first) = LoginFlow::start(creds);
            world.resource_mut::<Status>().0 = "Connecting...".into();
            crate::start_session(world, net, login, first);
        }
        Err(e) => world.resource_mut::<Status>().0 = format!("Cannot connect to {server}: {e}"),
    }
}

/// Developer shortcut: CROSSFIR3D_LOGIN="account:password:character[:new]" logs in to
/// localhost at startup; with an empty character it stops at the character list.
pub fn auto_login(mut commands: Commands) {
    let Ok(v) = std::env::var("CROSSFIR3D_LOGIN") else { return };
    let parts: Vec<&str> = v.split(':').collect();
    let [account, password, character, rest @ ..] = parts.as_slice() else {
        warn!("CROSSFIR3D_LOGIN must be account:password:character[:new]");
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

/// Everything `command_line` reads or changes in a frame, except egui and the session.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LineParams<'w> {
    line: ResMut<'w, CommandLine>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    time: Res<'w, Time>,
    mode: ResMut<'w, KeyMode>,
    last: ResMut<'w, LastAction>,
    seq: ResMut<'w, NcomSeq>,
    query: ResMut<'w, PendingQuery>,
    anim: ResMut<'w, crate::characters3d::PlayerAnimEvents>,
    pending: ResMut<'w, PendingBind>,
    bindings: ResMut<'w, KeyBindings>,
    msgs: ResMut<'w, Msgs>,
}

/// Enter opens a one-line box; Enter again sends the text as a command; Escape cancels.
pub fn command_line(mut contexts: EguiContexts, mut p: LineParams, session: Option<NonSend<Session>>) -> Result {
    if !command_line_allowed(p.mode.0) {
        return Ok(());
    }
    // A bound shift+Enter (or the like) runs its binding, not the command box.
    let enter = p.keys.just_pressed(KeyCode::Enter) && (p.line.open || !key_taken(&p.bindings, &p.keys, KeyCode::Enter));
    match line_action(p.line.open, enter, p.keys.just_pressed(KeyCode::Escape)) {
        LineAction::Open => {
            p.line.open = true;
            p.mode.0 = Mode::CommandLine;
        }
        LineAction::Cancel => {
            p.line.open = false;
            p.line.text.clear();
            p.mode.0 = Mode::World;
        }
        LineAction::Submit => submit_line(&mut p, session),
        LineAction::Nothing => {}
    }
    if p.line.open {
        line_window(contexts.ctx_mut()?, &mut p.line.text);
    }
    Ok(())
}

/// The one-line text box anchored at the bottom of the screen.
fn line_window(ctx: &egui::Context, text: &mut String) {
    let bottom = egui::Window::new("command").title_bar(false).resizable(false);
    bottom.anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0]).show(ctx, |ui| {
        ui.text_edit_singleline(text).request_focus();
    });
}

/// Closes the box: typed_bind first (save on change, then stop), else stamp `last`, and send to the
/// server only when there is a session and the text is not blank, with the anim event and `query.0 = false`.
fn submit_line(p: &mut LineParams, session: Option<NonSend<Session>>) {
    let command = std::mem::take(&mut p.line.text);
    p.line.open = false;
    p.mode.0 = Mode::World;
    let now = p.time.elapsed_secs();
    if let Some(changed) = typed_bind(&command, &mut p.pending, &mut p.bindings, &mut p.msgs, now) {
        if changed {
            save(&p.bindings, &session, &mut p.msgs, now);
        }
        return;
    }
    p.last.0 = Some(now);
    if let (Some(session), false) = (session, command.trim().is_empty()) {
        p.anim.0.extend(crossfir3d::anim::event_for_command(command.trim()));
        session.send([submit_message(command, p.query.0, p.seq.next())]);
        p.query.0 = false;
    }
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
    use crossfir3d::keys::{Binding, Combo, Mods};

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

    fn g() -> Binding {
        Binding { all: false, combo: Combo { mods: Mods::default(), key: "G".into() }, edit: false, command: "say g".into() }
    }

    /// Runs `text` with `step` pending and `n` bindings: (return, logged lines, pending after, bindings after).
    fn run(text: &str, step: Option<BindStep>, n: usize) -> (Option<bool>, Vec<String>, PendingBind, KeyBindings) {
        let (mut p, mut b, mut m) = (PendingBind(step), KeyBindings::default(), Msgs::default());
        for i in 0..n {
            b.0.bind(Binding { combo: Combo { mods: Mods::default(), key: ["G", "H"][i].into() }, ..g() });
        }
        let r = typed_bind(text, &mut p, &mut b, &mut m, 0.0);
        (r, m.0.log.iter().cloned().collect(), p, b)
    }

    #[test]
    fn typed_bind_answers() {
        let (r, l, p, b) = run(" Y ", Some(BindStep::Confirm { binding: g() }), 0);
        assert_eq!((r, l, p.0.is_none(), b.0.list.len()), (Some(true), vec!["Bound g to say g.".to_string()], true, 1));
        let (r, l, p, b) = run("n", Some(BindStep::Confirm { binding: g() }), 0);
        assert_eq!((r, l, p.0.is_none(), b.0.list.len()), (Some(false), vec!["Not changed.".to_string()], true, 0));
        let (r, l, p, b) = run("y", Some(BindStep::ConfirmReset), 2);
        assert_eq!((r, l, p.0.is_none(), b.0.list.len()), (Some(true), vec!["Key bindings removed.".to_string()], true, 0));
        let (r, l, p, b) = run("n", Some(BindStep::ConfirmReset), 2);
        assert_eq!((r, l, p.0.is_none(), b.0.list.len()), (Some(false), vec!["Not changed.".to_string()], true, 2));
    }

    #[test]
    fn typed_bind_unrelated_text_after_question() {
        for step in [BindStep::Confirm { binding: g() }, BindStep::ConfirmReset] {
            let (r, l, p, b) = run("shout hi", Some(step), 1);
            assert_eq!((r, l, p.0.is_none(), b.0.list.len()), (None, vec!["Not changed.".to_string()], true, 1));
        }
        let (r, l, p, _) = run("bind say x", Some(BindStep::ConfirmReset), 0);
        assert_eq!(r, Some(false));
        assert_eq!(l, ["Not changed.", "Press a key for: say x"]);
        assert!(p.waiting_key());
    }

    #[test]
    fn typed_bind_wait_key_keeps_waiting() {
        let wait = || BindStep::WaitKey { all: true, edit: false, command: "say w".into() };
        for text in ["y", "n", "shout hi"] {
            let (r, l, p, _) = run(text, Some(wait()), 0);
            assert_eq!((r, l.is_empty(), p.waiting_key()), (None, true, true), "{text}");
        }
        let (r, l, p, _) = run("unbind 9", Some(wait()), 0);
        assert_eq!((r, l, p.waiting_key()), (Some(false), vec!["No binding 9.".to_string()], true));
        let (r, l, p, _) = run("bind say x", Some(wait()), 0);
        assert_eq!((r, l, p.waiting_key()), (Some(false), vec!["Press a key for: say x".to_string()], true));
        assert!(matches!(p.0, Some(BindStep::WaitKey { all: false, edit: false, ref command }) if command == "say x"));
    }

    #[test]
    fn typed_bind_commands() {
        assert_eq!(run("say hi", None, 0).0, None);
        for (text, line) in [
            ("bind firekey1", "Not supported."),
            ("bind", "bind [-nfregc] <command>   unbind [<number>|reset]"),
            ("unbind", "No key bindings."),
            ("unbind 0", "No binding 0."),
            ("unbind 1", "No binding 1."),
            ("unbind x", "bind [-nfregc] <command>   unbind [<number>|reset]"),
        ] {
            let (r, l, p, b) = run(text, None, 0);
            assert_eq!((r, l, p.0.is_none(), b.0.list.len()), (Some(false), vec![line.to_string()], true, 0), "{text}");
        }
        let (r, l, _, b) = run("unbind", None, 2);
        assert_eq!((r, l, b.0.list.len()), (Some(false), vec!["1. g  say g".to_string(), "2. h  say g".to_string()], 2));
        let (r, l, _, b) = run("unbind 0", None, 1);
        assert_eq!((r, l, b.0.list.len()), (Some(false), vec!["No binding 0.".to_string()], 1));
        let (r, l, _, b) = run("unbind 2", None, 1);
        assert_eq!((r, l, b.0.list.len()), (Some(false), vec!["No binding 2.".to_string()], 1));
        let (r, l, _, b) = run("unbind 1", None, 2);
        assert_eq!((r, l, b.0.list.len()), (Some(true), vec!["Unbound g.".to_string()], 1));
        let (r, l, p, b) = run("unbind reset", None, 1);
        assert_eq!((r, l, b.0.list.len()), (Some(false), vec!["Remove all key bindings of this character? Type y or n.".to_string()], 1));
        assert!(matches!(p.0, Some(BindStep::ConfirmReset)));
        let (r, l, p, _) = run("bind -g say hello", None, 0);
        assert_eq!((r, l), (Some(false), vec!["Press a key for: say hello".to_string()]));
        assert!(matches!(p.0, Some(BindStep::WaitKey { all: true, edit: false, ref command }) if command == "say hello"));
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
        assert_eq!(b.0.numbered(true), ["1. g  say g"]);
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
