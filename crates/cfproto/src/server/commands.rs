//! One parser per server command, looked up by name from `parse`.

use super::{err, first_report, CharacterInfo, ParseError, Reader, ServerMsg};
use super::{ACL_CLASS, ACL_FACE_NUM, ACL_LEVEL, ACL_NAME, ACL_RACE};
use crate::items::{parse_item2, parse_upditem};
use crate::map2::parse_map2;
use crate::spells::{parse_addspell, parse_updspell};
use crate::stats::parse_stats;

type Parser = fn(&[u8]) -> Result<ServerMsg, ParseError>;

const COMMANDS: &[(&str, Parser)] = &[
    ("version", parse_version),
    ("setup", parse_setup),
    ("failure", parse_failure),
    ("accountplayers", parse_accountplayers),
    ("replyinfo", parse_replyinfo),
    ("addme_success", parse_addme_success),
    ("addme_failed", parse_addme_failed),
    ("newmap", parse_newmap),
    ("goodbye", parse_goodbye),
    ("map2", parse_map2_msg),
    ("face2", parse_face2),
    ("image2", parse_image2),
    ("anim", parse_anim),
    ("smooth", parse_smooth),
    ("comc", parse_comc),
    ("tick", parse_tick),
    ("item2", parse_item2_msg),
    ("upditem", parse_upditem_msg),
    ("delitem", parse_delitem),
    ("delinv", parse_delinv),
    ("player", parse_player),
    ("stats", parse_stats_msg),
    ("drawinfo", parse_drawinfo),
    ("drawextinfo", parse_drawextinfo),
    ("addspell", parse_addspell_msg),
    ("updspell", parse_updspell_msg),
    ("delspell", parse_delspell),
    ("sound2", parse_sound2),
    ("music", parse_music),
    ("query", parse_query),
];

/// Run the parser for this command word; an unlisted command is `Unknown`.
pub(super) fn dispatch(cmd: &str, data: &[u8]) -> Result<ServerMsg, ParseError> {
    match COMMANDS.iter().find(|(name, _)| *name == cmd) {
        Some((_, parser)) => parser(data),
        None => Ok(ServerMsg::Unknown(cmd.into())),
    }
}

/// Protocol versions and server name: "cs sc info".
fn parse_version(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let text = r.text();
    let mut it = text.splitn(3, ' ');
    let mut num = || it.next().and_then(|v| v.parse().ok()).ok_or_else(|| err("bad version"));
    let (cs, sc) = (num()?, num()?);
    let info = text.splitn(3, ' ').nth(2).unwrap_or("").to_string();
    Ok(ServerMsg::Version { cs, sc, info })
}

/// The server's answer to our setup request: a list of "name value" words.
fn parse_setup(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let text = r.text();
    let words: Vec<&str> = text.split(' ').filter(|w| !w.is_empty()).collect();
    Ok(ServerMsg::Setup(words.chunks(2).map(|c| (c[0].into(), c.get(1).unwrap_or(&"").to_string())).collect()))
}

/// A refused command and the reason, as text.
fn parse_failure(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let text = r.text();
    let (command, reason) = text.split_once(' ').unwrap_or((&text, ""));
    Ok(ServerMsg::Failure { command: command.into(), reason: reason.into() })
}

/// The characters on an account: a count, then one record per living character.
fn parse_accountplayers(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let count = r.u8()?;
    let mut chars = Vec::new();
    for _ in 0..count {
        // The count includes dead characters, which have no record.
        if r.0.is_empty() {
            break;
        }
        chars.push(read_character(&mut r)?);
    }
    Ok(ServerMsg::AccountPlayers(chars))
}

/// One character record: length-prefixed fields until a zero length.
fn read_character(r: &mut Reader) -> Result<CharacterInfo, ParseError> {
    let mut c = CharacterInfo::default();
    loop {
        let len = r.u8()? as usize;
        if len == 0 {
            break;
        }
        let field = r.take(len)?;
        set_character_field(&mut c, field[0], field);
    }
    Ok(c)
}

/// Store one character field (its first byte is the kind) in the record.
fn set_character_field(c: &mut CharacterInfo, kind: u8, field: &[u8]) {
    let text = || String::from_utf8_lossy(&field[1..]).into_owned();
    let short = || field.get(1..3).map(|b| u16::from_be_bytes([b[0], b[1]]));
    match kind {
        ACL_NAME => c.name = text(),
        ACL_CLASS => c.class = text(),
        ACL_RACE => c.race = text(),
        ACL_LEVEL => c.level = short().unwrap_or(0),
        ACL_FACE_NUM => c.face = short(),
        _ => {}
    }
}

/// A reply to an info request: the kind, then the raw rest.
fn parse_replyinfo(data: &[u8]) -> Result<ServerMsg, ParseError> {
    // The kind ends at a space or, for some replies, a newline.
    let (kind, rest) = match data.iter().position(|&b| b == b' ' || b == b'\n') {
        Some(i) => (&data[..i], &data[i + 1..]),
        None => (data, &[][..]),
    };
    Ok(ServerMsg::ReplyInfo { kind: String::from_utf8_lossy(kind).into_owned(), data: rest.to_vec() })
}

/// The server accepted our character.
fn parse_addme_success(_: &[u8]) -> Result<ServerMsg, ParseError> {
    Ok(ServerMsg::AddmeSuccess)
}

/// The server refused our character.
fn parse_addme_failed(_: &[u8]) -> Result<ServerMsg, ParseError> {
    Ok(ServerMsg::AddmeFailed)
}

/// A new map is starting; no data.
fn parse_newmap(_: &[u8]) -> Result<ServerMsg, ParseError> {
    Ok(ServerMsg::NewMap)
}

/// The server is closing the connection; no data.
fn parse_goodbye(_: &[u8]) -> Result<ServerMsg, ParseError> {
    Ok(ServerMsg::Goodbye)
}

/// A map update, parsed by the map2 module.
fn parse_map2_msg(data: &[u8]) -> Result<ServerMsg, ParseError> {
    Ok(ServerMsg::Map2(parse_map2(data)?))
}

/// A face announcement: number, set, checksum, name.
fn parse_face2(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let (num, set, checksum) = (r.u16()?, r.u8()?, r.u32()?);
    Ok(ServerMsg::Face2 { num, set, checksum, name: r.text() })
}

/// A face image: face, set, then a length-prefixed PNG.
fn parse_image2(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let (face, set, len) = (r.u32()?, r.u8()?, r.u32()?);
    Ok(ServerMsg::Image2 { face, set, png: r.take(len as usize)?.to_vec() })
}

/// An animation: id, unused flags, then face numbers to the end.
fn parse_anim(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let id = r.u16()?;
    r.u16()?; // flags, unused
    let mut faces = Vec::new();
    while !r.0.is_empty() {
        faces.push(r.u16()?);
    }
    Ok(ServerMsg::Anim { id, faces })
}

/// A smoothing level for a face: face, smooth.
fn parse_smooth(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::Smooth { face: r.u16()?, smooth: r.u16()? })
}

/// Command completion: packet number and time in ms.
fn parse_comc(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::Comc { packet: r.u16()?, time_us: r.u32()? })
}

/// The server tick number.
fn parse_tick(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::Tick(r.u32()?))
}

/// Items at a location, parsed by the items module.
fn parse_item2_msg(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let (location, items) = parse_item2(data)?;
    Ok(ServerMsg::Item2 { location, items })
}

/// Changes to one item, parsed by the items module.
fn parse_upditem_msg(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let (tag, changes) = parse_upditem(data)?;
    Ok(ServerMsg::UpdItem { tag, changes })
}

/// Item tags to delete, four bytes each to the end.
fn parse_delitem(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let mut tags = Vec::new();
    while !r.0.is_empty() {
        tags.push(r.u32()?);
    }
    Ok(ServerMsg::DelItem(tags))
}

/// Delete everything inside one tag, given as decimal text.
fn parse_delinv(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::DelInv(r.text().trim().parse().map_err(|_| err("bad delinv tag"))?))
}

/// Our own player: tag, weight, face, length-prefixed name.
fn parse_player(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let (tag, weight, face) = (r.u32()?, r.u32()?, r.u32()?);
    let len = r.u8()? as usize;
    Ok(ServerMsg::Player { tag, weight, face, name: String::from_utf8_lossy(r.take(len)?).into() })
}

/// Stat updates; a problem past the good ones is logged once, not returned.
fn parse_stats_msg(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let (updates, err) = parse_stats(data);
    if let Some(e) = err.filter(|e| first_report(&e.0)) {
        eprintln!("{}", e.0);
    }
    Ok(ServerMsg::Stats(updates))
}

/// Old-style text message: a color word, then the text.
fn parse_drawinfo(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::Text { kind: 0, subtype: 0, text: r.text().splitn(2, ' ').nth(1).unwrap_or("").into() })
}

/// Text message: color, kind, subtype, then the text.
fn parse_drawextinfo(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let text = r.text();
    let mut parts = text.splitn(4, ' ');
    let _color = parts.next();
    let kind = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let subtype = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    Ok(ServerMsg::Text { kind, subtype, text: parts.next().unwrap_or("").into() })
}

/// Spells learned, parsed by the spells module.
fn parse_addspell_msg(data: &[u8]) -> Result<ServerMsg, ParseError> {
    Ok(ServerMsg::AddSpell(parse_addspell(data)?))
}

/// Changes to one spell, parsed by the spells module.
fn parse_updspell_msg(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let (tag, mana, grace, damage) = parse_updspell(data)?;
    Ok(ServerMsg::UpdSpell { tag, mana, grace, damage })
}

/// A spell to forget, by tag.
fn parse_delspell(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::DelSpell(r.u32()?))
}

/// A sound: position, direction, volume, kind, then action and source names.
fn parse_sound2(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let (x, y, dir, volume, kind) = (r.u8()? as i8, r.u8()? as i8, r.u8()?, r.u8()?, r.u8()?);
    let len = r.u8()? as usize;
    let action = String::from_utf8_lossy(r.take(len)?).into_owned();
    let len = r.u8()? as usize;
    let source = String::from_utf8_lossy(r.take(len)?).into_owned();
    Ok(ServerMsg::Sound { x, y, dir, volume, kind, action, source })
}

/// The music to play, as a name.
fn parse_music(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    Ok(ServerMsg::Music(r.text().trim().to_string()))
}

/// A question for the player: flags, then the prompt text.
fn parse_query(data: &[u8]) -> Result<ServerMsg, ParseError> {
    let mut r = Reader(data);
    let text = r.text();
    let (flags, rest) = text.split_once(' ').unwrap_or((&text, ""));
    Ok(ServerMsg::Query { flags: flags.trim().parse().unwrap_or(0), text: rest.to_string() })
}
