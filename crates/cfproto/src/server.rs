//! Server-to-client messages.

mod commands;

use crate::map2::Map2Item;
use crate::items::{Item, ItemChange};
use crate::spells::Spell;
use crate::stats::StatValue;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharacterInfo {
    pub name: String,
    pub race: String,
    pub class: String,
    pub level: u16,
    pub face: Option<u16>,
}

#[derive(Debug, PartialEq)]
pub struct ParseError(pub String);

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMsg {
    Version { cs: u32, sc: u32, info: String },
    Setup(Vec<(String, String)>),
    Failure { command: String, reason: String },
    AccountPlayers(Vec<CharacterInfo>),
    ReplyInfo { kind: String, data: Vec<u8> },
    AddmeSuccess,
    AddmeFailed,
    NewMap,
    Map2(Vec<Map2Item>),
    Face2 { num: u16, set: u8, checksum: u32, name: String },
    Image2 { face: u32, set: u8, png: Vec<u8> },
    Anim { id: u16, faces: Vec<u16> },
    Smooth { face: u16, smooth: u16 },
    /// `time_us`: how long the command takes, in microseconds (server `MAX_TIME / speed`).
    Comc { packet: u16, time_us: u32 },
    Tick(u32),
    /// Stat updates read before any unknown code (the rest of that packet is dropped).
    Stats(Vec<(u8, StatValue)>),
    Item2 { location: u32, items: Vec<Item> },
    UpdItem { tag: u32, changes: Vec<ItemChange> },
    DelItem(Vec<u32>),
    /// Delete everything inside this tag (0 = the ground).
    DelInv(u32),
    Player { tag: u32, weight: u32, face: u32, name: String },
    /// drawextinfo / drawinfo. `kind` and `subtype` are the MSG_TYPE values (0 for drawinfo).
    Text { kind: u8, subtype: u8, text: String },
    AddSpell(Vec<Spell>),
    UpdSpell { tag: u32, mana: Option<i16>, grace: Option<i16>, damage: Option<i16> },
    DelSpell(u32),
    Sound { x: i8, y: i8, dir: u8, volume: u8, kind: u8, action: String, source: String },
    Music(String),
    Query { flags: u32, text: String },
    Goodbye,
    /// A command this client does not handle. Holds the command word.
    Unknown(String),
}

const ACL_NAME: u8 = 1;
const ACL_CLASS: u8 = 2;
const ACL_RACE: u8 = 3;
const ACL_LEVEL: u8 = 4;
const ACL_FACE_NUM: u8 = 8;

/// True the first time this text is seen: a newer server's unknown stat would
/// otherwise fill the log, once per `stats` packet.
fn first_report(text: &str) -> bool {
    static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    let new = !seen.iter().any(|s| s == text);
    if new {
        seen.push(text.to_string());
    }
    new
}

/// Splits a server payload into its command word and data, then dispatches it.
pub fn parse(payload: &[u8]) -> Result<ServerMsg, ParseError> {
    let (cmd, data) = match payload.iter().position(|&b| b == b' ') {
        Some(i) => (&payload[..i], &payload[i + 1..]),
        None => (payload, &[][..]),
    };
    let cmd = std::str::from_utf8(cmd)
        .ok()
        .filter(|c| c.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or_else(|| ParseError("command word is not ASCII".into()))?;
    commands::dispatch(cmd, data)
}

fn err(msg: &str) -> ParseError {
    ParseError(msg.into())
}

/// Big-endian reader over a byte slice that returns ParseError on short data.
pub(crate) struct Reader<'a>(pub(crate) &'a [u8]);

impl<'a> Reader<'a> {
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.0.len() < n {
            return Err(err("packet too short"));
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }
    pub(crate) fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }
    pub(crate) fn u16(&mut self) -> Result<u16, ParseError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().map_err(|_| err("packet too short"))?))
    }
    pub(crate) fn u32(&mut self) -> Result<u32, ParseError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().map_err(|_| err("packet too short"))?))
    }
    /// The rest of the data as text.
    pub(crate) fn text(&mut self) -> String {
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
        b.extend_from_slice(&[3, 4, 0x00, 0x05]); // ACL_LEVEL
        b.extend_from_slice(&[0, 0]);
        let ServerMsg::AccountPlayers(c) = p(&b) else { panic!() };
        assert_eq!((c[0].name.as_str(), c[0].level), ("Bob", 5));
    }

    #[test]
    fn accountplayers_full_records() {
        let mut b = b"accountplayers \x02".to_vec();
        b.extend_from_slice(&[4, 1, b'A', b'n', b'n']);
        b.extend_from_slice(&[6, 3, b'd', b'w', b'a', b'r', b'f']); // race
        b.extend_from_slice(&[7, 2, b'w', b'i', b'z', b'a', b'r', b'd']); // class
        b.extend_from_slice(&[3, 4, 0x00, 0x0c]); // level 12
        b.extend_from_slice(&[3, 8, 0x01, 0x02]); // face 258
        b.extend_from_slice(&[3, 7, b'x', b'y']); // map: skipped
        b.push(0);
        b.extend_from_slice(&[4, 1, b'B', b'o', b'b']);
        b.push(0);
        let ServerMsg::AccountPlayers(c) = p(&b) else { panic!() };
        assert_eq!(
            c[0],
            CharacterInfo { name: "Ann".into(), race: "dwarf".into(), class: "wizard".into(), level: 12, face: Some(258) }
        );
        assert_eq!(c[1].name, "Bob");
    }

    #[test]
    fn accountplayers_count_includes_dead_without_records() {
        // The server counts dead characters but sends no record for them.
        let mut b = b"accountplayers \x02".to_vec();
        b.extend_from_slice(&[4, 1, b'A', b'n', b'n']);
        b.push(0);
        let ServerMsg::AccountPlayers(c) = p(&b) else { panic!() };
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "Ann");
    }

    #[test]
    fn parse_accountplayers_empty() {
        assert_eq!(p(b"accountplayers \x00"), ServerMsg::AccountPlayers(vec![]));
    }

    #[test]
    fn replyinfo_kind_ends_at_newline() {
        assert_eq!(
            p(b"replyinfo startingmap\n\x01\x00\x02ab"),
            ServerMsg::ReplyInfo { kind: "startingmap".into(), data: b"\x01\x00\x02ab".to_vec() }
        );
        assert_eq!(
            p(b"replyinfo race_info dwarf_player\nname x"),
            ServerMsg::ReplyInfo { kind: "race_info".into(), data: b"dwarf_player\nname x".to_vec() }
        );
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
        assert_eq!(p(b"comc \x00\x02\x00\x00\x00\xfa"), ServerMsg::Comc { packet: 2, time_us: 250 });
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
        assert_eq!(p(b"drawextinfo 0 1 2 hello world"), ServerMsg::Text { kind: 1, subtype: 2, text: "hello world".into() });
        assert_eq!(p(b"drawinfo 0 hi"), ServerMsg::Text { kind: 0, subtype: 0, text: "hi".into() });
    }

    #[test]
    fn first_report_only() {
        assert!(first_report("stats: unknown code 250"));
        assert!(!first_report("stats: unknown code 250"));
        assert!(first_report("stats: unknown code 251"));
    }

    #[test]
    fn parse_stats_msg() {
        assert_eq!(p(b"stats \x01\x00\x05"), ServerMsg::Stats(vec![(1, StatValue::Int(5))]));
    }

    #[test]
    fn parse_item_messages() {
        let mut b = b"item2 ".to_vec();
        b.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(p(&b), ServerMsg::Item2 { location: 0, items: vec![] });
        assert_eq!(p(b"delitem \x00\x00\x00\x05\x00\x00\x00\x06"), ServerMsg::DelItem(vec![5, 6]));
        assert_eq!(p(b"delinv 77"), ServerMsg::DelInv(77));
        assert_eq!(p(b"delinv 0"), ServerMsg::DelInv(0));
        let mut b = b"upditem \x80\x00\x00\x00\x09".to_vec();
        b.extend_from_slice(&[0, 0, 0, 2]);
        assert_eq!(p(&b), ServerMsg::UpdItem { tag: 9, changes: vec![crate::ItemChange::Nrof(2)] });
        let mut b = b"player \x00\x00\x00\x01\x00\x00\x00\x02\x00\x00\x00\x03".to_vec();
        b.push(3);
        b.extend_from_slice(b"Bob");
        assert_eq!(p(&b), ServerMsg::Player { tag: 1, weight: 2, face: 3, name: "Bob".into() });
    }

    #[test]
    fn parse_spell_messages() {
        let mut b = b"addspell ".to_vec();
        b.extend(crate::spells::tests::spell_bytes(7, 1, "light"));
        let ServerMsg::AddSpell(s) = p(&b) else { panic!() };
        assert_eq!(s[0].name, "light");
        let mut b = b"updspell \x04\x00\x00\x00\x07".to_vec();
        b.extend_from_slice(&30i16.to_be_bytes());
        assert_eq!(p(&b), ServerMsg::UpdSpell { tag: 7, mana: None, grace: None, damage: Some(30) });
        assert_eq!(p(b"delspell \x00\x00\x00\x07"), ServerMsg::DelSpell(7));
    }

    #[test]
    fn parse_sound2() {
        let mut b = b"sound2 \xff\x02\x03\x50\x01".to_vec();
        b.push(4);
        b.extend_from_slice(b"miss");
        b.push(4);
        b.extend_from_slice(b"bird");
        assert_eq!(
            p(&b),
            ServerMsg::Sound { x: -1, y: 2, dir: 3, volume: 80, kind: 1, action: "miss".into(), source: "bird".into() }
        );
    }

    #[test]
    fn parse_music_and_query() {
        assert_eq!(p(b"music scorn"), ServerMsg::Music("scorn".into()));
        assert_eq!(p(b"query 4 Play again?"), ServerMsg::Query { flags: 4, text: "Play again?".into() });
        assert_eq!(p(b"query 0"), ServerMsg::Query { flags: 0, text: "".into() });
    }

    #[test]
    fn drawextinfo_keeps_type() {
        assert_eq!(p(b"drawextinfo 0 4 1 Hello"), ServerMsg::Text { kind: 4, subtype: 1, text: "Hello".into() });
    }

    #[test]
    fn parse_unknown() {
        assert_eq!(p(b"magicmap 1 1 0 0 x"), ServerMsg::Unknown("magicmap".into()));
        assert!(parse(b"\xff\xfe").is_err());
    }

    #[test]
    fn short_binary_is_error() {
        assert!(parse(b"comc \x00").is_err());
        assert!(parse(b"face2 \x00").is_err());
    }
}
