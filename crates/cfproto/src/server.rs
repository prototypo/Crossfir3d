//! Server-to-client messages.

use crate::map2::{parse_map2, Map2Item};
use crate::stats::{parse_stats, StatValue};

#[derive(Debug, PartialEq)]
pub struct ParseError(pub String);

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMsg {
    Version { cs: u32, sc: u32, info: String },
    Setup(Vec<(String, String)>),
    Failure { command: String, reason: String },
    AccountPlayers(Vec<String>),
    ReplyInfo { kind: String, data: Vec<u8> },
    AddmeSuccess,
    AddmeFailed,
    NewMap,
    Map2(Vec<Map2Item>),
    Face2 { num: u16, set: u8, checksum: u32, name: String },
    Image2 { face: u32, set: u8, png: Vec<u8> },
    Anim { id: u16, faces: Vec<u16> },
    Smooth { face: u16, smooth: u16 },
    Comc { packet: u16, time_ms: u32 },
    Tick(u32),
    /// Stat updates read before any unknown code (the rest of that packet is dropped).
    Stats(Vec<(u8, StatValue)>),
    Text(String),
    Goodbye,
    /// A command this client does not handle. Holds the command word.
    Unknown(String),
}

const ACL_NAME: u8 = 1;

pub fn parse(payload: &[u8]) -> Result<ServerMsg, ParseError> {
    let (cmd, data) = match payload.iter().position(|&b| b == b' ') {
        Some(i) => (&payload[..i], &payload[i + 1..]),
        None => (payload, &[][..]),
    };
    let cmd = std::str::from_utf8(cmd)
        .ok()
        .filter(|c| c.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or_else(|| ParseError("command word is not ASCII".into()))?;
    let mut r = Reader(data);
    Ok(match cmd {
        "version" => {
            let text = r.text();
            let mut it = text.splitn(3, ' ');
            let mut num = || it.next().and_then(|v| v.parse().ok()).ok_or_else(|| err("bad version"));
            let (cs, sc) = (num()?, num()?);
            let info = text.splitn(3, ' ').nth(2).unwrap_or("").to_string();
            ServerMsg::Version { cs, sc, info }
        }
        "setup" => {
            let text = r.text();
            let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
            ServerMsg::Setup(words.chunks(2).map(|c| (c[0].into(), c.get(1).unwrap_or(&"").to_string())).collect())
        }
        "failure" => {
            let text = r.text();
            let (command, reason) = text.split_once(' ').unwrap_or((&text, ""));
            ServerMsg::Failure { command: command.into(), reason: reason.into() }
        }
        "accountplayers" => {
            let count = r.u8()?;
            let mut names = Vec::new();
            for _ in 0..count {
                loop {
                    let len = r.u8()? as usize;
                    if len == 0 {
                        break;
                    }
                    let field = r.take(len)?;
                    if field[0] == ACL_NAME {
                        names.push(String::from_utf8_lossy(&field[1..]).into_owned());
                    }
                }
            }
            ServerMsg::AccountPlayers(names)
        }
        "replyinfo" => {
            let (kind, rest) = match data.iter().position(|&b| b == b' ') {
                Some(i) => (&data[..i], &data[i + 1..]),
                None => (data, &[][..]),
            };
            ServerMsg::ReplyInfo { kind: String::from_utf8_lossy(kind).into_owned(), data: rest.to_vec() }
        }
        "addme_success" => ServerMsg::AddmeSuccess,
        "addme_failed" => ServerMsg::AddmeFailed,
        "newmap" => ServerMsg::NewMap,
        "goodbye" => ServerMsg::Goodbye,
        "map2" => ServerMsg::Map2(parse_map2(data)?),
        "face2" => {
            let (num, set, checksum) = (r.u16()?, r.u8()?, r.u32()?);
            ServerMsg::Face2 { num, set, checksum, name: r.text() }
        }
        "image2" => {
            let (face, set, len) = (r.u32()?, r.u8()?, r.u32()?);
            ServerMsg::Image2 { face, set, png: r.take(len as usize)?.to_vec() }
        }
        "anim" => {
            let id = r.u16()?;
            r.u16()?; // flags, unused
            let mut faces = Vec::new();
            while !r.0.is_empty() {
                faces.push(r.u16()?);
            }
            ServerMsg::Anim { id, faces }
        }
        "smooth" => ServerMsg::Smooth { face: r.u16()?, smooth: r.u16()? },
        "comc" => ServerMsg::Comc { packet: r.u16()?, time_ms: r.u32()? },
        "tick" => ServerMsg::Tick(r.u32()?),
        "stats" => {
            let (updates, err) = parse_stats(data);
            if let Some(e) = err {
                eprintln!("{}", e.0);
            }
            ServerMsg::Stats(updates)
        }
        "drawinfo" => ServerMsg::Text(r.text().splitn(2, ' ').nth(1).unwrap_or("").into()),
        "drawextinfo" => ServerMsg::Text(r.text().splitn(4, ' ').nth(3).unwrap_or("").into()),
        other => ServerMsg::Unknown(other.into()),
    })
}

fn err(msg: &str) -> ParseError {
    ParseError(msg.into())
}

/// Big-endian reader over a byte slice that returns ParseError on short data.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.0.len() < n {
            return Err(err("packet too short"));
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ParseError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, ParseError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    /// The rest of the data as text.
    fn text(&mut self) -> String {
        let s = String::from_utf8_lossy(self.0).into_owned();
        self.0 = &[];
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(bytes: &[u8]) -> ServerMsg {
        parse(bytes).unwrap()
    }

    #[test]
    fn parse_version() {
        assert_eq!(
            p(b"version 1023 1030 Crossfire Server"),
            ServerMsg::Version { cs: 1023, sc: 1030, info: "Crossfire Server".into() }
        );
    }

    #[test]
    fn parse_setup() {
        assert_eq!(
            p(b"setup mapsize 25x25 facecache 1"),
            ServerMsg::Setup(vec![("mapsize".into(), "25x25".into()), ("facecache".into(), "1".into())])
        );
    }

    #[test]
    fn parse_failure() {
        assert_eq!(
            p(b"failure accountlogin wrong user name or password"),
            ServerMsg::Failure { command: "accountlogin".into(), reason: "wrong user name or password".into() }
        );
    }

    #[test]
    fn parse_accountplayers() {
        let mut b = b"accountplayers \x01".to_vec();
        b.extend_from_slice(&[4, 1, b'B', b'o', b'b']); // ACL_NAME
        b.extend_from_slice(&[3, 4, 0x00, 0x05]); // ACL_LEVEL, skipped
        b.push(0);
        assert_eq!(p(&b), ServerMsg::AccountPlayers(vec!["Bob".into()]));
    }

    #[test]
    fn parse_accountplayers_empty() {
        assert_eq!(p(b"accountplayers \x00"), ServerMsg::AccountPlayers(vec![]));
    }

    #[test]
    fn parse_replyinfo() {
        assert_eq!(
            p(b"replyinfo race_list |dwarf_player|elf_player"),
            ServerMsg::ReplyInfo { kind: "race_list".into(), data: b"|dwarf_player|elf_player".to_vec() }
        );
    }

    #[test]
    fn parse_face2() {
        let mut b = b"face2 \x00\x07\x00\xDE\xAD\xBE\xEF".to_vec();
        b.extend_from_slice(b"grass.111");
        assert_eq!(
            p(&b),
            ServerMsg::Face2 { num: 7, set: 0, checksum: 0xDEADBEEF, name: "grass.111".into() }
        );
    }

    #[test]
    fn parse_image2() {
        let b = b"image2 \x00\x00\x00\x07\x00\x00\x00\x00\x03\x01\x02\x03";
        assert_eq!(p(b), ServerMsg::Image2 { face: 7, set: 0, png: vec![1, 2, 3] });
    }

    #[test]
    fn image2_short() {
        let b = b"image2 \x00\x00\x00\x07\x00\x00\x00\x00\x0a\x01\x02\x03";
        assert!(parse(b).is_err());
    }

    #[test]
    fn parse_anim() {
        let b = b"anim \x00\x05\x00\x00\x00\x01\x00\x02\x00\x03";
        assert_eq!(p(b), ServerMsg::Anim { id: 5, faces: vec![1, 2, 3] });
    }

    #[test]
    fn parse_smooth() {
        assert_eq!(p(b"smooth \x00\x09\x01\x02"), ServerMsg::Smooth { face: 9, smooth: 0x0102 });
    }

    #[test]
    fn parse_comc() {
        assert_eq!(p(b"comc \x00\x02\x00\x00\x00\xfa"), ServerMsg::Comc { packet: 2, time_ms: 250 });
    }

    #[test]
    fn parse_tick() {
        assert_eq!(p(b"tick \x00\x00\x01\x00"), ServerMsg::Tick(256));
    }

    #[test]
    fn parse_no_data_commands() {
        assert_eq!(p(b"newmap"), ServerMsg::NewMap);
        assert_eq!(p(b"goodbye"), ServerMsg::Goodbye);
        assert_eq!(p(b"addme_success"), ServerMsg::AddmeSuccess);
        assert_eq!(p(b"addme_failed"), ServerMsg::AddmeFailed);
    }

    #[test]
    fn parse_map2_empty() {
        assert_eq!(p(b"map2 "), ServerMsg::Map2(vec![]));
    }

    #[test]
    fn parse_drawextinfo() {
        assert_eq!(p(b"drawextinfo 0 1 2 hello world"), ServerMsg::Text("hello world".into()));
        assert_eq!(p(b"drawinfo 0 hi"), ServerMsg::Text("hi".into()));
    }

    #[test]
    fn parse_stats_msg() {
        assert_eq!(p(b"stats \x01\x00\x05"), ServerMsg::Stats(vec![(1, StatValue::Int(5))]));
    }

    #[test]
    fn parse_unknown() {
        assert_eq!(p(b"music NONE"), ServerMsg::Unknown("music".into()));
        assert!(parse(b"\xff\xfe").is_err());
    }

    #[test]
    fn short_binary_is_error() {
        assert!(parse(b"comc \x00").is_err());
        assert!(parse(b"face2 \x00").is_err());
    }
}
