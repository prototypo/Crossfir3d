//! Login state machine: version/setup handshake, account login, pick or create a character.

use cfproto::{ClientMsg, ServerMsg};

#[derive(Debug, Clone)]
pub struct Creds {
    pub account: String,
    pub password: String,
    pub character: String,
    pub new_account: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoginState {
    Handshake,
    Account,
    Characters,
    Creating,
    Playing,
    Failed(String),
}

pub struct LoginFlow {
    creds: Creds,
    state: LoginState,
    race: Option<String>,
    class: Option<String>,
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
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()));
        let flow = LoginFlow { creds, state: LoginState::Handshake, race: None, class: None };
        (flow, vec![ClientMsg::Version, ClientMsg::Setup(setup.to_vec())])
    }

    pub fn on_message(&mut self, msg: &ServerMsg) -> Vec<ClientMsg> {
        let c = &self.creds;
        match (&self.state, msg) {
            (_, ServerMsg::Failure { reason, .. }) => {
                self.state = LoginState::Failed(reason.clone());
                vec![]
            }
            (LoginState::Handshake, ServerMsg::Setup(_)) => {
                self.state = LoginState::Account;
                let (name, password) = (c.account.clone(), c.password.clone());
                if c.new_account {
                    vec![ClientMsg::AccountNew { name, password }]
                } else {
                    vec![ClientMsg::AccountLogin { name, password }]
                }
            }
            (LoginState::Account, ServerMsg::AccountPlayers(names)) => {
                if names.contains(&c.character) {
                    self.state = LoginState::Playing;
                    vec![ClientMsg::AccountPlay(c.character.clone())]
                } else {
                    self.state = LoginState::Creating;
                    vec![ClientMsg::RequestInfo("race_list".into()), ClientMsg::RequestInfo("class_list".into())]
                }
            }
            (LoginState::Creating, ServerMsg::ReplyInfo { kind, data }) => {
                // Data is "|name|name...". Take the first name.
                let first = String::from_utf8_lossy(data).split('|').find(|s| !s.is_empty()).map(String::from);
                match kind.as_str() {
                    "race_list" => self.race = Some(first.unwrap_or_default()),
                    "class_list" => self.class = Some(first.unwrap_or_default()),
                    _ => return vec![],
                }
                let (Some(race), Some(class)) = (&self.race, &self.class) else { return vec![] };
                if race.is_empty() || class.is_empty() {
                    self.state = LoginState::Failed("server sent no races or classes".into());
                    return vec![];
                }
                // 7 stats of 12 = 84, within the server's default 85 points (settings starting_stat_points).
                let stats = ["Str", "Dex", "Con", "Wis", "Cha", "Int", "Pow"].map(|s| (s.to_string(), 12)).to_vec();
                let msg = ClientMsg::CreatePlayer {
                    name: c.character.clone(),
                    password: c.password.clone(),
                    race: race.clone(),
                    class: class.clone(),
                    stats,
                };
                self.state = LoginState::Playing;
                vec![msg]
            }
            _ => vec![],
        }
    }

    pub fn state(&self) -> &LoginState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds(new_account: bool) -> Creds {
        Creds { account: "acct".into(), password: "pw".into(), character: "Bob".into(), new_account }
    }

    fn logged_in() -> LoginFlow {
        let (mut f, _) = LoginFlow::start(creds(false));
        f.on_message(&ServerMsg::Setup(vec![]));
        f
    }

    #[test]
    fn start_sends_version_and_setup() {
        let (f, out) = LoginFlow::start(creds(false));
        assert_eq!(out[0], ClientMsg::Version);
        let ClientMsg::Setup(opts) = &out[1] else { panic!("expected setup, got {:?}", out[1]) };
        let want = [
            ("mapsize", "25x25"),
            ("facecache", "1"),
            ("loginmethod", "2"),
            ("darkness", "1"),
            ("tick", "1"),
            ("beat", "1"),
            ("faceset", "0"),
        ];
        assert_eq!(opts.len(), want.len());
        for (k, v) in want {
            assert!(opts.contains(&(k.into(), v.into())), "missing {k} {v}");
        }
        assert_eq!(f.state(), &LoginState::Handshake);
    }

    #[test]
    fn setup_reply_triggers_login() {
        let (mut f, _) = LoginFlow::start(creds(false));
        let out = f.on_message(&ServerMsg::Setup(vec![("mapsize".into(), "25x25".into())]));
        assert_eq!(out, vec![ClientMsg::AccountLogin { name: "acct".into(), password: "pw".into() }]);
        assert_eq!(f.state(), &LoginState::Account);
    }

    #[test]
    fn setup_reply_new_account() {
        let (mut f, _) = LoginFlow::start(creds(true));
        let out = f.on_message(&ServerMsg::Setup(vec![]));
        assert_eq!(out, vec![ClientMsg::AccountNew { name: "acct".into(), password: "pw".into() }]);
    }

    #[test]
    fn known_character_plays() {
        let mut f = logged_in();
        let out = f.on_message(&ServerMsg::AccountPlayers(vec!["Ann".into(), "Bob".into()]));
        assert_eq!(out, vec![ClientMsg::AccountPlay("Bob".into())]);
        assert_eq!(f.state(), &LoginState::Playing);
    }

    #[test]
    fn unknown_character_creates() {
        let mut f = logged_in();
        let out = f.on_message(&ServerMsg::AccountPlayers(vec![]));
        assert_eq!(
            out,
            vec![ClientMsg::RequestInfo("race_list".into()), ClientMsg::RequestInfo("class_list".into())]
        );
        assert_eq!(f.state(), &LoginState::Creating);
        let reply = |kind: &str, data: &str| ServerMsg::ReplyInfo { kind: kind.into(), data: data.into() };
        assert!(f.on_message(&reply("race_list", "|half_orc_player|human_player")).is_empty());
        let out = f.on_message(&reply("class_list", "|fighter_class"));
        let stats = ["Str", "Dex", "Con", "Wis", "Cha", "Int", "Pow"].map(|s| (s.to_string(), 12)).to_vec();
        assert_eq!(
            out,
            vec![ClientMsg::CreatePlayer {
                name: "Bob".into(),
                password: "pw".into(),
                race: "half_orc_player".into(),
                class: "fighter_class".into(),
                stats,
            }]
        );
        assert_eq!(f.state(), &LoginState::Playing);
    }

    #[test]
    fn failure_sets_failed() {
        let mut f = logged_in();
        let out = f.on_message(&ServerMsg::Failure {
            command: "accountlogin".into(),
            reason: "wrong user name or password".into(),
        });
        assert!(out.is_empty());
        assert_eq!(f.state(), &LoginState::Failed("wrong user name or password".into()));
    }

    #[test]
    fn empty_race_list_fails() {
        let mut f = logged_in();
        f.on_message(&ServerMsg::AccountPlayers(vec![]));
        f.on_message(&ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"".to_vec() });
        f.on_message(&ServerMsg::ReplyInfo { kind: "class_list".into(), data: b"|fighter_class".to_vec() });
        assert!(matches!(f.state(), LoginState::Failed(_)));
    }
}
