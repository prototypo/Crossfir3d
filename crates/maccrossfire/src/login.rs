//! Login state machine: handshake, account login, then the account's character list,
//! from which the player plays a character or creates one.

use cfproto::{CharacterInfo, ClientMsg, ServerMsg};

use crate::newchar::NewChar;

#[derive(Debug, Clone)]
pub struct Creds {
    pub account: String,
    pub password: String,
    pub new_account: bool,
    /// Developer login only (MACCROSSFIRE_LOGIN): play this character, or quickly
    /// create it (first race and class, all stats 12) if the account lacks it.
    pub quick_character: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoginState {
    Handshake,
    Account,
    /// Showing the character list.
    Characters,
    /// Showing the new-character screen (or quick-creating).
    Creating,
    Playing,
    Failed(String),
}

pub struct LoginFlow {
    creds: Creds,
    state: LoginState,
    characters: Vec<CharacterInfo>,
    playing: String,
    pub newchar: NewChar,
}

impl LoginFlow {
    pub fn start(creds: Creds) -> (Self, Vec<ClientMsg>) {
        let setup = [
            ("mapsize", "25x25"),
            ("facecache", "1"),
            ("loginmethod", "2"),
            ("darkness", "1"),
            ("tick", "1"),
            ("beat", "1"),
            ("faceset", "0"),
            ("spellmon", "1"),
            ("sound2", "3"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let flow = LoginFlow {
            creds,
            state: LoginState::Handshake,
            characters: vec![],
            playing: String::new(),
            newchar: NewChar::default(),
        };
        (flow, vec![ClientMsg::Version, ClientMsg::Setup(setup.to_vec())])
    }

    pub fn on_message(&mut self, msg: &ServerMsg) -> Vec<ClientMsg> {
        match (&self.state, msg) {
            (LoginState::Creating, ServerMsg::Failure { command, reason }) if command == "createplayer" => {
                self.newchar.error = Some(reason.clone());
                vec![]
            }
            (_, ServerMsg::Failure { reason, .. }) => {
                self.state = LoginState::Failed(reason.clone());
                vec![]
            }
            (LoginState::Handshake, ServerMsg::Setup(_)) => {
                self.state = LoginState::Account;
                let (name, password) = (self.creds.account.clone(), self.creds.password.clone());
                if self.creds.new_account {
                    vec![ClientMsg::AccountNew { name, password }]
                } else {
                    vec![ClientMsg::AccountLogin { name, password }]
                }
            }
            (LoginState::Account | LoginState::Playing, ServerMsg::AccountPlayers(chars)) => {
                self.characters = chars.clone();
                self.state = LoginState::Characters;
                match self.creds.quick_character.clone() {
                    Some(q) if self.playing.is_empty() && chars.iter().any(|c| c.name == q) => self.play(&q),
                    Some(q) if self.playing.is_empty() => {
                        self.newchar.name = q;
                        self.start_creating()
                    }
                    _ => vec![],
                }
            }
            (LoginState::Creating, ServerMsg::AddmeSuccess | ServerMsg::Map2(_)) => {
                self.state = LoginState::Playing;
                vec![]
            }
            (LoginState::Creating, ServerMsg::ReplyInfo { kind, data }) => {
                let mut out = self.newchar.on_reply(kind, data);
                let quick = self.creds.quick_character.is_some();
                if quick && kind == "class_list" && self.newchar.race.is_some() {
                    self.newchar.stats = [12; 7];
                    out.extend(self.create());
                }
                out
            }
            _ => vec![],
        }
    }

    pub fn characters(&self) -> &[CharacterInfo] {
        &self.characters
    }

    pub fn play(&mut self, name: &str) -> Vec<ClientMsg> {
        self.playing = name.to_string();
        self.state = LoginState::Playing;
        vec![ClientMsg::AccountPlay(name.to_string())]
    }

    pub fn start_creating(&mut self) -> Vec<ClientMsg> {
        let name = std::mem::take(&mut self.newchar.name);
        self.newchar = NewChar { name, ..NewChar::default() };
        self.state = LoginState::Creating;
        NewChar::requests()
    }

    /// Sends the new character from the form; nothing if the form has a problem.
    pub fn create(&mut self) -> Vec<ClientMsg> {
        let Some(msg) = self.newchar.create_msg(&self.creds.password) else { return vec![] };
        self.newchar.error = None;
        self.playing = self.newchar.name.trim().to_string();
        // Stay on the form until the game starts: the server may still refuse.
        vec![msg]
    }

    pub fn back_to_list(&mut self) {
        self.state = LoginState::Characters;
    }

    pub fn password(&self) -> &str {
        &self.creds.password
    }

    /// The character being played (for per-character quick slots).
    pub fn character(&self) -> &str {
        &self.playing
    }

    pub fn state(&self) -> &LoginState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds(new_account: bool, quick: Option<&str>) -> Creds {
        Creds { account: "acct".into(), password: "pw".into(), new_account, quick_character: quick.map(String::from) }
    }

    fn logged_in(quick: Option<&str>) -> LoginFlow {
        let (mut f, _) = LoginFlow::start(creds(false, quick));
        f.on_message(&ServerMsg::Setup(vec![]));
        f
    }

    fn named(n: &str) -> CharacterInfo {
        CharacterInfo { name: n.into(), ..Default::default() }
    }

    #[test]
    fn start_sends_version_and_setup() {
        let (f, out) = LoginFlow::start(creds(false, None));
        assert_eq!(out[0], ClientMsg::Version);
        let ClientMsg::Setup(opts) = &out[1] else { panic!("expected setup, got {:?}", out[1]) };
        for (k, v) in [("loginmethod", "2"), ("mapsize", "25x25"), ("spellmon", "1"), ("sound2", "3")] {
            assert!(opts.contains(&(k.into(), v.into())), "missing {k} {v}");
        }
        assert_eq!(f.state(), &LoginState::Handshake);
    }

    #[test]
    fn setup_reply_triggers_login() {
        let (mut f, _) = LoginFlow::start(creds(false, None));
        let out = f.on_message(&ServerMsg::Setup(vec![]));
        assert_eq!(out, vec![ClientMsg::AccountLogin { name: "acct".into(), password: "pw".into() }]);
        let (mut f, _) = LoginFlow::start(creds(true, None));
        let out = f.on_message(&ServerMsg::Setup(vec![]));
        assert_eq!(out, vec![ClientMsg::AccountNew { name: "acct".into(), password: "pw".into() }]);
    }

    #[test]
    fn account_login_shows_list() {
        let mut f = logged_in(None);
        let out = f.on_message(&ServerMsg::AccountPlayers(vec![named("Ann"), named("Bob")]));
        assert!(out.is_empty());
        assert_eq!(f.state(), &LoginState::Characters);
        assert_eq!(f.characters().len(), 2);
    }

    #[test]
    fn zero_characters_list_is_empty() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        assert_eq!(f.state(), &LoginState::Characters);
        assert!(f.characters().is_empty());
    }

    #[test]
    fn play_sends_accountplay() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![named("Bob")]));
        assert_eq!(f.play("Bob"), vec![ClientMsg::AccountPlay("Bob".into())]);
        assert_eq!(f.state(), &LoginState::Playing);
        assert_eq!(f.character(), "Bob");
    }

    #[test]
    fn creating_asks_for_data_and_routes_replies() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        assert_eq!(f.start_creating(), NewChar::requests());
        assert_eq!(f.state(), &LoginState::Creating);
        let out = f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|dwarf_player".to_vec() });
        assert_eq!(out, vec![ClientMsg::RequestInfo("race_info dwarf_player".into())]);
        assert_eq!(f.newchar.race.as_deref(), Some("dwarf_player"));
    }

    #[test]
    fn create_sends_form_with_account_password() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        f.start_creating();
        f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|dwarf_player".to_vec() });
        f.on_message(&ServerMsg::ReplyInfo { kind: "class_list".into(), data: b"|wizard_class".to_vec() });
        assert!(f.create().is_empty(), "no name yet");
        f.newchar.name = "Zed".into();
        let out = f.create();
        let [ClientMsg::CreatePlayer { name, password, .. }] = &out[..] else { panic!("{out:?}") };
        assert_eq!((name.as_str(), password.as_str()), ("Zed", "pw"));
        assert_eq!(f.character(), "Zed");
    }

    #[test]
    fn creating_failure_keeps_creating() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        f.start_creating();
        f.newchar.name = "Zed".into();
        f.on_message(&ServerMsg::Failure { command: "createplayer".into(), reason: "That name is already in use".into() });
        assert_eq!(f.state(), &LoginState::Creating);
        assert_eq!(f.newchar.error.as_deref(), Some("That name is already in use"));
        assert_eq!(f.newchar.name, "Zed");
    }

    #[test]
    fn refusal_after_create_keeps_the_form() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        f.start_creating();
        f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|dwarf_player".to_vec() });
        f.on_message(&ServerMsg::ReplyInfo { kind: "class_list".into(), data: b"|wizard_class".to_vec() });
        f.newchar.name = "Zed".into();
        assert!(!f.create().is_empty());
        f.on_message(&ServerMsg::Failure { command: "createplayer".into(), reason: "That name is already in use".into() });
        assert_eq!(f.state(), &LoginState::Creating);
        assert_eq!(f.newchar.error.as_deref(), Some("That name is already in use"));
        // The game starting means play has begun.
        f.on_message(&ServerMsg::AddmeSuccess);
        assert_eq!(f.state(), &LoginState::Playing);
    }

    #[test]
    fn other_failure_fails() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::Failure { command: "accountlogin".into(), reason: "wrong password".into() });
        assert_eq!(f.state(), &LoginState::Failed("wrong password".into()));
    }

    #[test]
    fn quick_character_still_auto_plays() {
        let mut f = logged_in(Some("Bob"));
        let out = f.on_message(&ServerMsg::AccountPlayers(vec![named("Bob")]));
        assert_eq!(out, vec![ClientMsg::AccountPlay("Bob".into())]);
        assert_eq!(f.state(), &LoginState::Playing);
    }

    #[test]
    fn quick_character_still_auto_creates() {
        let mut f = logged_in(Some("Bob"));
        let out = f.on_message(&ServerMsg::AccountPlayers(vec![]));
        assert_eq!(out, NewChar::requests());
        f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|half_orc_player|human_player".to_vec() });
        let out = f.on_message(&ServerMsg::ReplyInfo { kind: "class_list".into(), data: b"|fighter_class".to_vec() });
        let create = out.iter().find(|m| matches!(m, ClientMsg::CreatePlayer { .. })).expect("create sent");
        let ClientMsg::CreatePlayer { name, race, class, stats, .. } = create else { unreachable!() };
        assert_eq!((name.as_str(), race.as_str(), class.as_str()), ("Bob", "half_orc_player", "fighter_class"));
        assert!(stats.iter().all(|(_, v)| *v == 12));
        f.on_message(&ServerMsg::AddmeSuccess);
        assert_eq!(f.state(), &LoginState::Playing);
    }

    #[test]
    fn accountplayers_while_playing_returns_to_list() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![named("Bob")]));
        f.play("Bob");
        f.on_message(&ServerMsg::AccountPlayers(vec![named("Bob")]));
        assert_eq!(f.state(), &LoginState::Characters);
    }
}
