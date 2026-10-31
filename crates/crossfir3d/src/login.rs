//! Login state machine: handshake, account login, then the account's character list,
//! from which the player plays a character or creates one.

use cfproto::{CharacterInfo, ClientMsg, ServerMsg};

use crate::newchar::NewChar;
use crate::races::{classify_player_face, PlayerFace};

#[derive(Debug, Clone)]
pub struct Creds {
    pub account: String,
    pub password: String,
    pub new_account: bool,
    /// Play this character at once (picked on the login screen, or CROSSFIR3D_LOGIN).
    pub quick_character: Option<String>,
    /// Developer login only: quickly create the quick character (first race and
    /// class, all stats 12) if the account lacks it.
    pub create_if_missing: bool,
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

/// Before the character list or the new-character form shows, the screen says
/// "Connecting..." (a slow account login was a blank screen).
pub fn still_connecting(state: &LoginState) -> bool {
    matches!(state, LoginState::Handshake | LoginState::Account)
}

pub struct LoginFlow {
    creds: Creds,
    state: LoginState,
    characters: Vec<CharacterInfo>,
    playing: String,
    pub newchar: NewChar,
    /// The highlighted row of the character list.
    pub selected: Option<usize>,
    /// A `createplayer` was sent and has no answer yet.
    create_pending: bool,
    /// Why the last Play was refused ("Character X is already playing"), shown on the list.
    pub notice: Option<String>,
}

impl LoginFlow {
    /// Starts the login flow and returns the setup messages to send first.
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
            ("want_pickup", "1"),
            ("sound2", "3"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let flow = LoginFlow {
            creds,
            state: LoginState::Handshake,
            characters: vec![],
            playing: String::new(),
            newchar: NewChar::default(),
            selected: None,
            create_pending: false,
            notice: None,
        };
        (flow, vec![ClientMsg::Version, ClientMsg::Setup(setup.to_vec())])
    }

    /// Moves the login flow on for a server message and returns what to send back.
    pub fn on_message(&mut self, msg: &ServerMsg) -> Vec<ClientMsg> {
        match (&self.state, msg) {
            (_, ServerMsg::Failure { command, reason }) => self.on_failure(command, reason),
            (LoginState::Handshake, ServerMsg::Setup(_)) => self.on_setup(),
            (LoginState::Account | LoginState::Playing, ServerMsg::AccountPlayers(chars)) => self.on_players(chars),
            (LoginState::Creating, ServerMsg::AddmeSuccess | ServerMsg::Map2(_)) => {
                self.state = LoginState::Playing;
                self.create_pending = false;
                vec![]
            }
            (LoginState::Creating, ServerMsg::ReplyInfo { kind, data }) => self.on_reply(kind, data),
            _ => vec![],
        }
    }

    /// A server Failure: some are recoverable in the current state, the rest end the login.
    fn on_failure(&mut self, command: &str, reason: &str) -> Vec<ClientMsg> {
        match &self.state {
            LoginState::Creating if command == "createplayer" => {
                self.newchar.error = Some(reason.to_string());
                self.create_pending = false;
                vec![]
            }
            // A late answer to a repeated Create after the game started: not a failure.
            LoginState::Playing if command == "createplayer" => vec![],
            // Developer login with `:new` on a later run: the account is there, so log in.
            LoginState::Account if self.account_exists(command, reason) => {
                self.creds.new_account = false;
                vec![ClientMsg::AccountLogin { name: self.creds.account.clone(), password: self.creds.password.clone() }]
            }
            // Play refused (the character is still in the game after a crash, ...): back
            // to the list with the reason, not out of the session.
            LoginState::Playing if command == "accountplay" => {
                self.playing.clear();
                self.state = LoginState::Characters;
                self.notice = Some(reason.to_string());
                vec![]
            }
            _ => {
                self.state = LoginState::Failed(reason.to_string());
                vec![]
            }
        }
    }

    /// A developer login's account creation was refused because the account is already there.
    fn account_exists(&self, command: &str, reason: &str) -> bool {
        command == "accountnew" && reason.contains("already exists") && self.creds.create_if_missing
    }

    /// The server accepted the setup: log in to the account, or create it.
    fn on_setup(&mut self) -> Vec<ClientMsg> {
        self.state = LoginState::Account;
        let (name, password) = (self.creds.account.clone(), self.creds.password.clone());
        if self.creds.new_account {
            vec![ClientMsg::AccountNew { name, password }]
        } else {
            vec![ClientMsg::AccountLogin { name, password }]
        }
    }

    /// The account's character list arrived: show it, or quick-play or quick-create for a developer login.
    fn on_players(&mut self, chars: &[CharacterInfo]) -> Vec<ClientMsg> {
        self.characters = chars.iter().cloned().map(with_class).collect();
        self.selected = None;
        self.state = LoginState::Characters;
        match self.creds.quick_character.clone() {
            Some(q) if self.playing.is_empty() && chars.iter().any(|c| c.name == q) => self.play(&q),
            Some(q) if self.playing.is_empty() && self.creds.create_if_missing => {
                self.newchar.name = q;
                self.start_creating()
            }
            _ => vec![],
        }
    }

    /// An answer to a new-character request: feed the form, and finish a quick create once the lists are in.
    fn on_reply(&mut self, kind: &str, data: &[u8]) -> Vec<ClientMsg> {
        let mut out = self.newchar.on_reply(kind, data);
        let quick = self.creds.create_if_missing && self.creds.quick_character.is_some();
        if quick && kind == "class_list" && self.newchar.race.is_some() {
            self.newchar.stats = [12; 7];
            out.extend(self.create());
        }
        out
    }

    pub fn characters(&self) -> &[CharacterInfo] {
        &self.characters
    }

    pub fn play(&mut self, name: &str) -> Vec<ClientMsg> {
        self.notice = None;
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
        if self.create_pending {
            return vec![];
        }
        let Some(msg) = self.newchar.create_msg(&self.creds.password) else { return vec![] };
        self.create_pending = true;
        self.newchar.error = None;
        self.playing = self.newchar.name.trim().to_string();
        // Stay on the form until the game starts: the server may still refuse.
        vec![msg]
    }

    /// Back to the character list; not while a create waits for the server's answer.
    pub fn back_to_list(&mut self) -> bool {
        if self.create_pending {
            return false;
        }
        self.state = LoginState::Characters;
        true
    }

    pub fn create_pending(&self) -> bool {
        self.create_pending
    }

    pub fn account(&self) -> &str {
        &self.creds.account
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

/// The server never keeps a character's class, so it sends an empty one: take it from the
/// character's picture ("ranger.151" is a ranger), as "Ranger".
fn with_class(mut c: CharacterInfo) -> CharacterInfo {
    if let (true, Some(PlayerFace::Class(arch))) = (c.class.is_empty(), classify_player_face(&c.face_name)) {
        let name = arch.strip_suffix("_class").unwrap_or(arch);
        c.class = name[..1].to_ascii_uppercase() + &name[1..];
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_class_comes_from_the_picture() {
        let c = |class: &str, face: &str| CharacterInfo { class: class.into(), face_name: face.into(), ..Default::default() };
        assert_eq!(with_class(c("", "ranger.151")).class, "Ranger");
        assert_eq!(with_class(c("Wizard", "ranger.151")).class, "Wizard");
        assert_eq!(with_class(c("", "")).class, "");
    }

    #[test]
    fn refused_play_goes_back_to_the_list_with_the_reason() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![named("Ann")]));
        f.play("Ann");
        f.on_message(&ServerMsg::Failure { command: "accountplay".into(), reason: "Character Ann is already playing".into() });
        assert_eq!(*f.state(), LoginState::Characters);
        assert_eq!(f.notice.as_deref(), Some("Character Ann is already playing"));
    }

    #[test]
    fn still_connecting_until_the_list_or_form_shows() {
        assert!(still_connecting(&LoginState::Handshake));
        assert!(still_connecting(&LoginState::Account));
        assert!(!still_connecting(&LoginState::Characters));
        assert!(!still_connecting(&LoginState::Creating));
    }

    fn creds(new_account: bool, quick: Option<&str>) -> Creds {
        Creds { account: "acct".into(), password: "pw".into(), new_account, quick_character: quick.map(String::from), create_if_missing: quick.is_some() }
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

    fn creating_ready() -> LoginFlow {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        f.start_creating();
        f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|dwarf_player".to_vec() });
        f.on_message(&ServerMsg::ReplyInfo { kind: "class_list".into(), data: b"|wizard_class".to_vec() });
        f.newchar.name = "Zed".into();
        f
    }

    #[test]
    fn create_twice_sends_once() {
        let mut f = creating_ready();
        assert_eq!(f.create().len(), 1);
        assert!(f.create().is_empty());
        assert!(f.create_pending());
        // The game starts; a late refusal of the second click must not end the session.
        f.on_message(&ServerMsg::AddmeSuccess);
        f.on_message(&ServerMsg::Failure { command: "createplayer".into(), reason: "Not allowed right now".into() });
        assert_eq!(f.state(), &LoginState::Playing);
    }

    #[test]
    fn refusal_reenables_create() {
        let mut f = creating_ready();
        f.create();
        f.on_message(&ServerMsg::Failure { command: "createplayer".into(), reason: "name in use".into() });
        assert!(!f.create_pending());
        assert_eq!(f.create().len(), 1);
    }

    #[test]
    fn back_ignored_while_create_pending() {
        let mut f = creating_ready();
        f.create();
        assert!(!f.back_to_list());
        assert_eq!(f.state(), &LoginState::Creating);
        f.on_message(&ServerMsg::Failure { command: "createplayer".into(), reason: "x".into() });
        assert!(f.back_to_list());
        assert_eq!(f.state(), &LoginState::Characters);
    }

    #[test]
    fn new_list_clears_selection() {
        let mut f = logged_in(None);
        f.on_message(&ServerMsg::AccountPlayers(vec![named("Ann"), named("Bob")]));
        f.selected = Some(1);
        f.play("Bob");
        f.on_message(&ServerMsg::AccountPlayers(vec![named("Ann")]));
        assert_eq!(f.selected, None);
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
    fn picked_character_that_is_gone_shows_the_list() {
        let (mut f, _) = LoginFlow::start(Creds { create_if_missing: false, ..creds(false, Some("Bob")) });
        f.on_message(&ServerMsg::Setup(vec![]));
        assert!(f.on_message(&ServerMsg::AccountPlayers(vec![named("Ann")])).is_empty());
        assert_eq!(f.state(), &LoginState::Characters);
        // A new character from the list is the player's own form, not a quick create.
        f.start_creating();
        f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|dwarf_player".to_vec() });
        let out = f.on_message(&ServerMsg::ReplyInfo { kind: "class_list".into(), data: b"|wizard_class".to_vec() });
        assert!(!out.iter().any(|m| matches!(m, ClientMsg::CreatePlayer { .. })));
    }

    #[test]
    fn quick_new_account_that_exists_logs_in() {
        let exists = ServerMsg::Failure { command: "accountnew".into(), reason: "That account already exists on this server".into() };
        let (mut f, _) = LoginFlow::start(creds(true, Some("Bob")));
        f.on_message(&ServerMsg::Setup(vec![]));
        assert_eq!(f.on_message(&exists), vec![ClientMsg::AccountLogin { name: "acct".into(), password: "pw".into() }]);
        assert_eq!(f.state(), &LoginState::Account);
        // The login form keeps showing the refusal.
        let (mut f, _) = LoginFlow::start(creds(true, None));
        f.on_message(&ServerMsg::Setup(vec![]));
        assert!(f.on_message(&exists).is_empty());
        assert!(matches!(f.state(), LoginState::Failed(_)));
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
