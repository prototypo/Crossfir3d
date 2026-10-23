//! Client-to-server messages.

#[derive(Debug, Clone, PartialEq)]
pub enum ClientMsg {
    Version,
    Setup(Vec<(String, String)>),
    RequestInfo(String),
    AccountLogin { name: String, password: String },
    AccountNew { name: String, password: String },
    AccountPlay(String),
    CreatePlayer {
        name: String,
        password: String,
        race: String,
        class: String,
        stats: Vec<(String, u8)>,
        starting_map: Option<String>,
        /// (choice key, chosen arch)
        choices: Vec<(String, String)>,
    },
    AskFace(u32),
    Ncom { packet: u16, repeat: i32, command: String },
    Beat,
    Apply(u32),
    /// Move `count` of item `tag` into `to` (0 = ground, player tag, or a container tag).
    Move { to: u32, tag: u32, count: u32 },
    Examine(u32),
    Reply(String),
}

impl ClientMsg {
    /// The payload, without the frame length.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ClientMsg::Version => b"version 1023 1030 Crossfir3d".to_vec(),
            ClientMsg::Setup(opts) => {
                let mut s = String::from("setup");
                for (k, v) in opts {
                    s += &format!(" {k} {v}");
                }
                s.into_bytes()
            }
            ClientMsg::RequestInfo(what) => format!("requestinfo {what}").into_bytes(),
            ClientMsg::AccountLogin { name, password } => with_lstrings("accountlogin", &[name, password]),
            ClientMsg::AccountNew { name, password } => with_lstrings("accountnew", &[name, password]),
            ClientMsg::AccountPlay(name) => format!("accountplay {name}").into_bytes(),
            ClientMsg::CreatePlayer { name, password, race, class, stats, starting_map, choices } => {
                let mut b = with_lstrings("createplayer", &[name, password]);
                // Each extra line is an lstring that includes a trailing null (see create_player_cmd).
                let mut lines = vec![format!("race {race}\0"), format!("class {class}\0")];
                lines.extend(stats.iter().map(|(k, v)| format!("{k} {v}\0")));
                lines.extend(starting_map.iter().map(|m| format!("starting_map {m}\0")));
                lines.extend(choices.iter().map(|(k, a)| format!("choice {k} {a}\0")));
                for line in &lines {
                    push_lstring(&mut b, line);
                }
                b
            }
            ClientMsg::AskFace(num) => format!("askface {num}").into_bytes(),
            ClientMsg::Ncom { packet, repeat, command } => {
                let mut b = b"ncom ".to_vec();
                b.extend_from_slice(&packet.to_be_bytes());
                b.extend_from_slice(&repeat.to_be_bytes());
                b.extend_from_slice(command.as_bytes());
                b
            }
            ClientMsg::Beat => b"beat".to_vec(),
            ClientMsg::Apply(tag) => format!("apply {tag}").into_bytes(),
            ClientMsg::Move { to, tag, count } => format!("move {to} {tag} {count}").into_bytes(),
            ClientMsg::Examine(tag) => format!("examine {tag}").into_bytes(),
            ClientMsg::Reply(text) => format!("reply {text}").into_bytes(),
        }
    }
}

fn with_lstrings(cmd: &str, parts: &[&String]) -> Vec<u8> {
    let mut b = format!("{cmd} ").into_bytes();
    for p in parts {
        push_lstring(&mut b, p);
    }
    b
}

/// Appends a 1-byte length prefix and the string. Strings over 255 bytes are cut.
fn push_lstring(b: &mut Vec<u8>, s: &str) {
    let bytes = &s.as_bytes()[..s.len().min(255)];
    b.push(bytes.len() as u8);
    b.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_string()
    }

    fn check(cases: Vec<(ClientMsg, Vec<u8>)>) {
        for (m, want) in cases {
            assert_eq!(m.encode(), want, "{m:?}");
        }
    }

    fn lstr(b: &mut Vec<u8>, s: &[u8]) {
        b.push(s.len() as u8);
        b.extend_from_slice(s);
    }

    #[test]
    fn golden_text() {
        check(vec![
            (ClientMsg::Setup(vec![]), b"setup".to_vec()),
            (ClientMsg::Setup(vec![(s("é"), String::new()), (s("日本"), s("1"))]), "setup é  日本 1".as_bytes().to_vec()),
            (ClientMsg::RequestInfo(String::new()), b"requestinfo ".to_vec()),
            (ClientMsg::AccountPlay(s("héllo")), "accountplay héllo".as_bytes().to_vec()),
            (ClientMsg::Reply("日".repeat(100)), format!("reply {}", "日".repeat(100)).into_bytes()),
        ]);
    }

    #[test]
    fn golden_account() {
        let mut long = b"accountlogin ".to_vec();
        lstr(&mut long, &[b'x'; 255]);
        lstr(&mut long, b"");
        let mut cut = b"accountnew ".to_vec();
        lstr(&mut cut, &"日".repeat(100).as_bytes()[..255]);
        lstr(&mut cut, "日本".as_bytes());
        check(vec![
            (ClientMsg::AccountLogin { name: "x".repeat(300), password: String::new() }, long),
            (ClientMsg::AccountNew { name: "日".repeat(100), password: s("日本") }, cut),
        ]);
    }

    #[test]
    fn golden_createplayer() {
        let mut bare = b"createplayer \x01n\x00".to_vec();
        for l in ["race r\0", "class c\0", "starting_map é\0"] {
            lstr(&mut bare, l.as_bytes());
        }
        let mut full = b"createplayer \x01n\x00".to_vec();
        for l in ["race r\0", "class c\0", "Str 0\0", "日 255\0", "choice k é\0"] {
            lstr(&mut full, l.as_bytes());
        }
        let mk = |stats, starting_map, choices| ClientMsg::CreatePlayer { name: s("n"), password: String::new(), race: s("r"), class: s("c"), stats, starting_map, choices };
        check(vec![
            (mk(vec![], Some(s("é")), vec![]), bare),
            (mk(vec![(s("Str"), 0), (s("日"), 255)], None, vec![(s("k"), s("é"))]), full),
        ]);
    }

    #[test]
    fn golden_game() {
        check(vec![
            (ClientMsg::AskFace(u32::MAX), b"askface 4294967295".to_vec()),
            (ClientMsg::Apply(0), b"apply 0".to_vec()),
            (ClientMsg::Examine(u32::MAX), b"examine 4294967295".to_vec()),
            (ClientMsg::Move { to: u32::MAX, tag: 0, count: 1 }, b"move 4294967295 0 1".to_vec()),
            (ClientMsg::Ncom { packet: u16::MAX, repeat: i32::MIN, command: s("日") }, b"ncom \xff\xff\x80\x00\x00\x00\xe6\x97\xa5".to_vec()),
            (ClientMsg::Ncom { packet: 0, repeat: i32::MAX, command: String::new() }, b"ncom \x00\x00\x7f\xff\xff\xff".to_vec()),
        ]);
    }

    #[test]
    fn encode_version() {
        assert_eq!(ClientMsg::Version.encode(), b"version 1023 1030 Crossfir3d");
    }

    #[test]
    fn encode_setup() {
        let m = ClientMsg::Setup(vec![(s("mapsize"), s("25x25")), (s("facecache"), s("1"))]);
        assert_eq!(m.encode(), b"setup mapsize 25x25 facecache 1");
    }

    #[test]
    fn encode_accountlogin() {
        let m = ClientMsg::AccountLogin { name: s("bob"), password: s("pw") };
        assert_eq!(m.encode(), b"accountlogin \x03bob\x02pw");
    }

    #[test]
    fn encode_accountnew() {
        let m = ClientMsg::AccountNew { name: s("bob"), password: s("pw") };
        assert_eq!(m.encode(), b"accountnew \x03bob\x02pw");
    }

    #[test]
    fn encode_createplayer() {
        let m = ClientMsg::CreatePlayer {
            name: s("Bob"),
            password: s("pw"),
            race: s("human_player"),
            class: s("fighter_class"),
            stats: vec![(s("Str"), 12)],
            starting_map: None,
            choices: vec![],
        };
        let mut want = b"createplayer \x03Bob\x02pw".to_vec();
        for line in ["race human_player\0", "class fighter_class\0", "Str 12\0"] {
            want.push(line.len() as u8);
            want.extend_from_slice(line.as_bytes());
        }
        assert_eq!(m.encode(), want);
    }

    #[test]
    fn createplayer_with_map_and_choice() {
        let m = ClientMsg::CreatePlayer {
            name: s("Bob"),
            password: s("pw"),
            race: s("dwarf_player"),
            class: s("wizard_class"),
            stats: vec![(s("Str"), 12)],
            starting_map: Some(s("map_scorn")),
            choices: vec![(s("race_choice_1"), s("Gaea"))],
        };
        let mut want = b"createplayer \x03Bob\x02pw".to_vec();
        for line in ["race dwarf_player\0", "class wizard_class\0", "Str 12\0", "starting_map map_scorn\0", "choice race_choice_1 Gaea\0"] {
            want.push(line.len() as u8);
            want.extend_from_slice(line.as_bytes());
        }
        assert_eq!(m.encode(), want);
    }

    #[test]
    fn encode_ncom() {
        let m = ClientMsg::Ncom { packet: 1, repeat: 0, command: s("north") };
        assert_eq!(m.encode(), b"ncom \x00\x01\x00\x00\x00\x00north");
    }

    #[test]
    fn encode_askface() {
        assert_eq!(ClientMsg::AskFace(42).encode(), b"askface 42");
    }

    #[test]
    fn encode_apply_move_examine_reply() {
        assert_eq!(ClientMsg::Apply(12).encode(), b"apply 12");
        assert_eq!(ClientMsg::Move { to: 0, tag: 12, count: 3 }.encode(), b"move 0 12 3");
        assert_eq!(ClientMsg::Examine(12).encode(), b"examine 12");
        assert_eq!(ClientMsg::Reply("y".into()).encode(), b"reply y");
    }

    #[test]
    fn encode_simple() {
        assert_eq!(ClientMsg::AccountPlay(s("Bob")).encode(), b"accountplay Bob");
        assert_eq!(ClientMsg::RequestInfo(s("race_list")).encode(), b"requestinfo race_list");
        assert_eq!(ClientMsg::Beat.encode(), b"beat");
    }
}
