//! What `pump_network` does with each server message.
use crate::{
    audio, characters3d, combat_fx, end_session, input, reset_play_state, AppState, Clock, Faces, Inv, LastAction,
    LastComc, Msgs, PendingQuery, PlayerStats, Session, SkillNames, Spells, Status, TimeRequest, View, ViewSize,
};
use bevy::prelude::*;
use cfproto::ServerMsg;
use crossfir3d::accounts::{default_accounts_path, load_accounts};
use crossfir3d::clock::{is_time_reply, is_time_reply_line, parse_time_reply};
use crossfir3d::login::LoginState;
use crossfir3d::net::NetEvent;

/// The next network event, or None when there is no session or nothing is waiting.
pub(crate) fn next_event(world: &World) -> Option<NetEvent> {
    let session = world.get_non_send::<Session>()?;
    match session.net.rx.try_recv() {
        Ok(e) => Some(e),
        Err(std::sync::mpsc::TryRecvError::Empty) => None,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(NetEvent::Disconnected("connection closed".into())),
    }
}

/// On a permadeath server the character list comes back after death.
pub(crate) fn permadeath_reset(world: &mut World, msg: &ServerMsg) {
    let in_game = *world.resource::<State<AppState>>().get() == AppState::InGame;
    let died = world.resource::<Msgs>().0.died;
    if let (true, Some(reason)) = (in_game, crossfir3d::messages::in_game_reset(msg, died)) {
        world.resource_mut::<Status>().0 = reason.to_string();
        world.resource_mut::<NextState<AppState>>().set(AppState::Login);
        reset_play_state(world);
    }
}

/// Feeds the message to the login flow; Some(playing), or None after ending the session on failure.
pub(crate) fn advance_login(world: &mut World, msg: &ServerMsg) -> Option<bool> {
    let mut session = world.non_send_mut::<Session>();
    let out = session.login.on_message(msg);
    session.send(out);
    if let ServerMsg::AccountPlayers(chars) = msg {
        remember_account(world, chars);
    }
    let session = world.non_send::<Session>();
    match session.login.state() {
        LoginState::Failed(reason) => {
            let reason = reason.clone();
            end_session(world, reason);
            None
        }
        LoginState::Playing => Some(true),
        _ => Some(false),
    }
}

/// A login worked: the login screen offers this account and its characters next time.
fn remember_account(world: &mut World, chars: &[cfproto::CharacterInfo]) {
    let account = world.non_send::<Session>().login.account().to_string();
    let path = default_accounts_path();
    if let Err(e) = crossfir3d::accounts::remember_account(&path, &account, chars) {
        warn!("could not save the account list: {e}");
    }
    world.resource_mut::<crate::ui::LoginForm>().remembered(load_accounts(&path));
}

/// Sends a message to the first group handler that wants it.
pub(crate) fn dispatch(world: &mut World, msg: ServerMsg, playing: bool) {
    let Some(msg) = map_view(world, msg, playing) else { return };
    let Some(msg) = faces(world, msg) else { return };
    let Some(msg) = text(world, msg) else { return };
    let Some(msg) = inventory(world, msg) else { return };
    let Some(msg) = spells(world, msg) else { return };
    misc(world, msg);
}

/// Stores the map size from a setup reply, when it reads as WxH.
fn set_mapsize(world: &mut World, opts: &[(String, String)]) {
    if let Some((_, v)) = opts.iter().find(|(k, _)| k == "mapsize") {
        if let Some((w, h)) = v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))) {
            world.insert_resource(ViewSize(w, h));
        }
    }
}

/// Setup, map, new map, comc and tick; returns other messages.
fn map_view(world: &mut World, msg: ServerMsg, playing: bool) -> Option<ServerMsg> {
    match msg {
        ServerMsg::Setup(opts) => set_mapsize(world, &opts),
        ServerMsg::Map2(items) => {
            let &ViewSize(w, h) = world.resource::<ViewSize>();
            let mut view = world.resource_mut::<View>();
            view.0.apply(&items);
            view.0.remember_outside(w, h);
            if playing && *world.resource::<State<AppState>>().get() == AppState::Login {
                world.resource_mut::<NextState<AppState>>().set(AppState::InGame);
            }
        }
        ServerMsg::NewMap => {
            world.resource_mut::<View>().0.clear();
            world.resource_mut::<input::DoorExit>().new_map = true;
            // A new map after dying is the respawn.
            world.resource_mut::<characters3d::PlayerAnimEvents>().0.push(crossfir3d::anim::AnimEvent::Revived);
        }
        ServerMsg::Comc { packet, time_us } => world.insert_resource(LastComc { time_ms: time_us / 1000, packet }),
        ServerMsg::Tick(t) => world.resource_mut::<Clock>().0.on_tick(t),
        other => return Some(other),
    }
    None
}

/// Face, image and animation data; returns other messages.
fn faces(world: &mut World, msg: ServerMsg) -> Option<ServerMsg> {
    match msg {
        ServerMsg::Face2 { num, checksum, name, .. } => {
            let ask = world.resource_mut::<Faces>().0.on_face2(num, checksum, &name);
            world.non_send::<Session>().send(ask);
        }
        ServerMsg::Image2 { face, png, .. } => world.resource_mut::<Faces>().0.on_image2(face, png),
        ServerMsg::Anim { id, faces } => world.resource_mut::<Faces>().0.on_anim(id, faces),
        other => return Some(other),
    }
    None
}

/// Uses a line of the reply to our `time` request for the clock.
fn time_reply(world: &mut World, text: &str) {
    if crossfir3d::clock::ends_time_reply(text) {
        // A DM's timing lines follow the season at once: hide them for 2 s more.
        let now = world.resource::<Time>().elapsed_secs();
        let rest = text.starts_with("Time of Year: ").then_some(now - 8.0);
        world.resource_mut::<TimeRequest>().0 = rest;
    }
    if is_time_reply(text) {
        match parse_time_reply(text) {
            Some((hour, minute)) => {
                world.resource_mut::<Clock>().0.sync(hour, minute);
                info!("game clock synced: hour {hour}, minute {minute}");
            }
            None => warn!("could not read the time: {text}"),
        }
    }
}

/// A reply line to our own `time` request (within 10 s) is for the clock, not the player.
fn time_reply_expected(world: &World, text: &str) -> bool {
    let now = world.resource::<Time>().elapsed_secs();
    world.resource::<TimeRequest>().0.is_some_and(|t| now - t <= 10.0) && is_time_reply_line(text)
}

/// Text and query messages; returns other messages.
fn text(world: &mut World, msg: ServerMsg) -> Option<ServerMsg> {
    match msg {
        // Every line of the reply to our own `time` request is for the clock, not the player.
        ServerMsg::Text { text, .. } if time_reply_expected(world, &text) => time_reply(world, &text),
        ServerMsg::Text { kind, subtype, text } => {
            info!("server: {text}");
            let (now, last) = (world.resource::<Time>().elapsed_secs(), world.resource::<LastAction>().0);
            world.resource_mut::<Msgs>().0.push((kind, subtype), &text, now, last);
            if kind == combat_fx::MSG_ATTACK {
                world.resource_mut::<combat_fx::CombatFx>().attacks.push(subtype);
            }
            if let Some(ev) = crossfir3d::anim::event_for_message(kind, subtype) {
                world.resource_mut::<characters3d::PlayerAnimEvents>().0.push(ev);
            }
        }
        ServerMsg::Query { text, .. } => {
            world.resource_mut::<PendingQuery>().0 = true;
            let now = world.resource::<Time>().elapsed_secs();
            world.resource_mut::<Msgs>().0.push((0, 0), &format!("Server asks: {text} (answer with Enter)"), now, None);
        }
        other => return Some(other),
    }
    None
}

/// Stats, player and item messages; returns other messages.
fn inventory(world: &mut World, msg: ServerMsg) -> Option<ServerMsg> {
    match msg {
        ServerMsg::Stats(updates) => world.resource_mut::<PlayerStats>().0.apply(&updates),
        ServerMsg::Player { tag, .. } => world.resource_mut::<Inv>().0.player = tag,
        ServerMsg::Item2 { location, items } => world.resource_mut::<Inv>().0.apply_item2(location, items),
        ServerMsg::UpdItem { tag, changes } => world.resource_mut::<Inv>().0.apply_upd(tag, &changes),
        ServerMsg::DelItem(tags) => world.resource_mut::<Inv>().0.delete(&tags),
        ServerMsg::DelInv(tag) => world.resource_mut::<Inv>().0.delete_inside(tag),
        other => return Some(other),
    }
    None
}

/// Spell book messages; returns other messages.
fn spells(world: &mut World, msg: ServerMsg) -> Option<ServerMsg> {
    match msg {
        ServerMsg::AddSpell(spells) => world.resource_mut::<Spells>().0.add(spells),
        ServerMsg::UpdSpell { tag, mana, grace, damage } => world.resource_mut::<Spells>().0.update(tag, mana, grace, damage),
        ServerMsg::DelSpell(tag) => world.resource_mut::<Spells>().0.remove(tag),
        other => return Some(other),
    }
    None
}

/// Skill names, sound, music and unknown commands; anything else is ignored.
fn misc(world: &mut World, msg: ServerMsg) {
    match msg {
        ServerMsg::ReplyInfo { kind, data } if kind == "skill_info" => {
            world.insert_resource(SkillNames(cfproto::parse_skill_info(&data)))
        }
        ServerMsg::Sound { action, volume, x, y, .. } => {
            world.resource_mut::<audio::AudioQueue>().sounds.push((action, volume, x, y))
        }
        ServerMsg::Music(name) => world.resource_mut::<audio::AudioQueue>().music = Some(name),
        ServerMsg::Unknown(name) => debug!("ignored server command {name}"),
        _ => {}
    }
}
