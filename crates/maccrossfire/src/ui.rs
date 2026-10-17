//! egui screens: the login form and the in-game command line.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use cfproto::ClientMsg;
use maccrossfire::login::{Creds, LoginFlow};

use crate::{NcomSeq, Session, Status};

#[derive(Resource)]
pub struct LoginForm {
    server: String,
    account: String,
    password: String,
    character: String,
    new_account: bool,
}

impl Default for LoginForm {
    fn default() -> Self {
        LoginForm {
            server: "localhost:13327".into(),
            account: String::new(),
            password: String::new(),
            character: String::new(),
            new_account: false,
        }
    }
}

pub fn login_screen(
    mut contexts: EguiContexts,
    mut form: ResMut<LoginForm>,
    status: Res<Status>,
    mut commands: Commands,
) -> Result {
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
            ui.label("Character");
            ui.text_edit_singleline(&mut form.character);
            ui.end_row();
        });
        ui.checkbox(&mut form.new_account, "New account");
        connect = ui.button("Connect").clicked();
        if !status.0.is_empty() {
            ui.label(&status.0);
        }
    });
    if connect {
        let creds = Creds {
            account: form.account.clone(),
            password: form.password.clone(),
            character: form.character.clone(),
            new_account: form.new_account,
        };
        queue_connect(&mut commands, form.server.clone(), creds);
    }
    Ok(())
}

fn queue_connect(commands: &mut Commands, server: String, creds: Creds) {
    commands.queue(move |world: &mut World| match maccrossfire::net::connect(&server) {
        Ok(net) => {
            let (login, first) = LoginFlow::start(creds);
            world.resource_mut::<Status>().0 = "Connecting...".into();
            crate::start_session(world, net, login, first);
        }
        Err(e) => world.resource_mut::<Status>().0 = format!("Cannot connect to {server}: {e}"),
    });
}

/// Developer shortcut: MACCROSSFIRE_LOGIN="account:password:character[:new]" logs in to
/// localhost at startup.
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
        character: character.to_string(),
        new_account: rest.first() == Some(&"new"),
    };
    queue_connect(&mut commands, LoginForm::default().server, creds);
}

#[derive(Resource, Default)]
pub struct CommandLine {
    pub open: bool,
    text: String,
}

/// Enter opens a one-line box; Enter again sends the text as a command.
pub fn command_line(
    mut contexts: EguiContexts,
    mut line: ResMut<CommandLine>,
    keys: Res<ButtonInput<KeyCode>>,
    mut seq: ResMut<NcomSeq>,
    session: Option<NonSend<Session>>,
) -> Result {
    if !line.open {
        line.open = keys.just_pressed(KeyCode::Enter);
        return Ok(());
    }
    let mut submit = false;
    let bottom = egui::Window::new("command").title_bar(false).resizable(false);
    bottom.anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0]).show(contexts.ctx_mut()?, |ui| {
        let edit = ui.text_edit_singleline(&mut line.text);
        edit.request_focus();
        submit = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    });
    if submit {
        let command = std::mem::take(&mut line.text);
        line.open = false;
        if let (Some(session), false) = (session, command.trim().is_empty()) {
            session.send([ClientMsg::Ncom { packet: seq.next(), repeat: 0, command }]);
        }
    }
    Ok(())
}
