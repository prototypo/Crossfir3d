//! Client-to-server messages.

#[derive(Debug, Clone, PartialEq)]
pub enum ClientMsg {
    Version,
    Setup(Vec<(String, String)>),
    RequestInfo(String),
    AccountLogin { name: String, password: String },
    AccountNew { name: String, password: String },
    AccountPlay(String),
    CreatePlayer { name: String, password: String, race: String, class: String, stats: Vec<(String, u8)> },
    AskFace(u32),
    Ncom { packet: u16, repeat: i32, command: String },
    Beat,
}

impl ClientMsg {
    /// The payload, without the frame length.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ClientMsg::Version => b"version 1023 1030 MacCrossfire".to_vec(),
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
            ClientMsg::CreatePlayer { name, password, race, class, stats } => {
                let mut b = with_lstrings("createplayer", &[name, password]);
                // Each extra line is an lstring that includes a trailing null (see create_player_cmd).
                let mut lines = vec![format!("race {race}\0"), format!("class {class}\0")];
                lines.extend(stats.iter().map(|(k, v)| format!("{k} {v}\0")));
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

    #[test]
    fn encode_version() {
        assert_eq!(ClientMsg::Version.encode(), b"version 1023 1030 MacCrossfire");
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
        };
        let mut want = b"createplayer \x03Bob\x02pw".to_vec();
        for line in ["race human_player\0", "class fighter_class\0", "Str 12\0"] {
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
    fn encode_simple() {
        assert_eq!(ClientMsg::AccountPlay(s("Bob")).encode(), b"accountplay Bob");
        assert_eq!(ClientMsg::RequestInfo(s("race_list")).encode(), b"requestinfo race_list");
        assert_eq!(ClientMsg::Beat.encode(), b"beat");
    }
}
