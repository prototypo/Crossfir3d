//! Needs a local server: scripts/server.sh run, then
//! cargo test -p crossfir3d --test live_server -- --ignored

use std::time::{Duration, Instant, SystemTime};

use cfproto::{ClientMsg, Map2Item, ServerMsg};
use crossfir3d::login::{Creds, LoginFlow};
use crossfir3d::net::{connect, Net, NetEvent};

/// Next server message, or panic after `wait`. Answers login messages on the way.
fn next(net: &Net, flow: &mut LoginFlow, wait: Duration) -> ServerMsg {
    match net.rx.recv_timeout(wait).expect("server went quiet") {
        NetEvent::Msg(m) => {
            for out in flow.on_message(&m) {
                net.tx.send(out).unwrap();
            }
            m
        }
        NetEvent::Disconnected(r) => panic!("disconnected: {r}"),
    }
}

fn scrolled(msg: &ServerMsg) -> Option<(i8, i8)> {
    let ServerMsg::Map2(items) = msg else { return None };
    items.iter().find_map(|i| match i {
        Map2Item::Scroll { dx, dy } => Some((*dx, *dy)),
        _ => None,
    })
}

#[test]
#[ignore]
fn walk_one_step() {
    let net = connect("localhost:13327").expect("is the server running? scripts/server.sh run");
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() % 1_000_000;
    let name = format!("walk{stamp}");
    let creds = Creds { account: name.clone(), password: "testpw".into(), new_account: true, quick_character: Some(name) };
    let (mut flow, first) = LoginFlow::start(creds);
    for m in first {
        net.tx.send(m).unwrap();
    }

    let start = Instant::now();
    while !matches!(next(&net, &mut flow, Duration::from_secs(5)), ServerMsg::Map2(_)) {
        assert!(start.elapsed() < Duration::from_secs(10), "no map after login");
    }

    let tries = [("north", (0, -1)), ("east", (1, 0)), ("south", (0, 1)), ("west", (-1, 0))];
    for (packet, (command, want)) in tries.into_iter().enumerate() {
        net.tx.send(ClientMsg::Ncom { packet: packet as u16 + 1, repeat: 0, command: command.into() }).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let Ok(NetEvent::Msg(m)) = net.rx.recv_timeout(Duration::from_millis(200)) else { continue };
            if let Some(got) = scrolled(&m) {
                assert_eq!(got, want, "moved {command}");
                return;
            }
        }
    }
    panic!("could not move in any direction");
}

/// Waits until `check` finds what it wants in a server message, or panics.
fn wait_for<T>(net: &Net, flow: &mut LoginFlow, what: &str, mut check: impl FnMut(&ServerMsg) -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        let msg = next(net, flow, Duration::from_secs(8));
        if let Some(t) = check(&msg) {
            return t;
        }
    }
    panic!("timed out waiting for {what}");
}

/// Logs in as a new account with a quick character and returns the connection and login flow.
fn quick_login(prefix: &str) -> (Net, LoginFlow) {
    let net = connect("localhost:13327").expect("is the server running? scripts/server.sh run");
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() % 1_000_000;
    let name = format!("{prefix}{stamp}");
    let creds = Creds { account: name.clone(), password: "testpw".into(), new_account: true, quick_character: Some(name) };
    let (flow, first) = LoginFlow::start(creds);
    for m in first {
        net.tx.send(m).unwrap();
    }
    (net, flow)
}

/// Waits until item `tag` is listed at `location`.
fn wait_item_at(net: &Net, flow: &mut LoginFlow, what: &str, location: u32, tag: u32) {
    wait_for(net, flow, what, |m| match m {
        ServerMsg::Item2 { location: l, items } if *l == location => items.iter().any(|i| i.tag == tag).then_some(()),
        _ => None,
    });
}

/// Waits for the player's tag.
fn wait_player(net: &Net, flow: &mut LoginFlow) -> u32 {
    wait_for(net, flow, "player", |m| match m {
        ServerMsg::Player { tag, .. } => Some(*tag),
        _ => None,
    })
}

/// Waits for a starting item in the player's inventory that is neither locked nor worn; returns (tag, nrof).
fn wait_droppable(net: &Net, flow: &mut LoginFlow, player: u32) -> (u32, u32) {
    wait_for(net, flow, "a droppable item", |m| match m {
        ServerMsg::Item2 { location, items } if *location == player => {
            items.iter().find(|i| !i.locked() && !i.applied()).map(|i| (i.tag, i.nrof))
        }
        _ => None,
    })
}

#[test]
#[ignore]
fn pick_up_and_drop() {
    let (net, mut flow) = quick_login("drop");
    let player = wait_player(&net, &mut flow);
    let (tag, nrof) = wait_droppable(&net, &mut flow, player);

    net.tx.send(ClientMsg::Move { to: 0, tag, count: nrof }).unwrap();
    wait_item_at(&net, &mut flow, "the dropped item on the ground", 0, tag);

    net.tx.send(ClientMsg::Move { to: player, tag, count: nrof }).unwrap();
    wait_item_at(&net, &mut flow, "the item back in the inventory", player, tag);
}

#[test]
#[ignore]
fn creation_data_parses() {
    use cfproto::newchar::{parse_arch_info, parse_newcharinfo, parse_starting_maps};
    let net = connect("localhost:13327").expect("is the server running? scripts/server.sh run");
    net.tx.send(ClientMsg::Version).unwrap();
    net.tx.send(ClientMsg::Setup(vec![("loginmethod".into(), "2".into())])).unwrap();
    for q in ["newcharinfo", "race_list", "startingmap", "race_info dwarf_player", "class_info wizard_class"] {
        net.tx.send(ClientMsg::RequestInfo(q.into())).unwrap();
    }
    let mut seen = 0;
    while let Ok(NetEvent::Msg(m)) = net.rx.recv_timeout(Duration::from_secs(3)) {
        let ServerMsg::ReplyInfo { kind, data } = m else { continue };
        seen += 1;
        match kind.as_str() {
            "newcharinfo" => assert!(parse_newcharinfo(&data).points >= 60),
            "race_list" => assert!(String::from_utf8_lossy(&data).contains("dwarf_player")),
            "startingmap" => assert!(parse_starting_maps(&data).unwrap().iter().any(|m| m.arch == "map_scorn" && m.name == "Scorn")),
            "race_info" | "class_info" => {
                let info = parse_arch_info(&data).unwrap();
                assert!(!info.name.is_empty() && !info.message.is_empty(), "{kind}: {info:?}");
            }
            other => panic!("unexpected reply {other}"),
        }
    }
    assert_eq!(seen, 5);
}

#[test]
#[ignore]
fn create_chosen_character() {
    let net = connect("localhost:13327").expect("is the server running? scripts/server.sh run");
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() % 1_000_000;
    let name = format!("make{stamp}");
    let creds = Creds { account: name.clone(), password: "testpw".into(), new_account: true, quick_character: None };
    let (mut flow, first) = LoginFlow::start(creds);
    for m in first {
        net.tx.send(m).unwrap();
    }
    wait_for(&net, &mut flow, "character list", |m| matches!(m, ServerMsg::AccountPlayers(_)).then_some(()));
    for m in flow.start_creating() {
        net.tx.send(m).unwrap();
    }
    // Wait until the lists, the second race's and class's descriptions, and the maps arrived.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "creation data incomplete");
        next(&net, &mut flow, Duration::from_secs(5));
        let n = &flow.newchar;
        let ready = n.races.len() > 1
            && n.classes.len() > 1
            && n.infos.contains_key(&n.races[1])
            && n.infos.contains_key(&n.classes[1])
            && n.map.is_some();
        if ready {
            break;
        }
    }
    let n = &mut flow.newchar;
    assert_eq!(n.map.as_deref(), Some("map_scorn"));
    n.race = Some(n.races[1].clone());
    n.class = Some(n.classes[1].clone());
    n.refresh_choices();
    n.name = name.clone();
    // Roll until the race and class leave no stat below 1.
    for seed in 1.. {
        n.roll(seed);
        if n.problem().is_none() {
            break;
        }
    }
    let create = flow.create();
    assert!(matches!(&create[..], [ClientMsg::CreatePlayer { .. }]), "{create:?}");
    for m in create {
        net.tx.send(m).unwrap();
    }
    wait_for(&net, &mut flow, "the game to start", |m| matches!(m, ServerMsg::Map2(_)).then_some(()));
    assert_eq!(flow.character(), name);
}

/// A new character of `class` starting on `map` (arch names, e.g. "alchemist_class", "map_scorn"), in the game.
fn play_as(prefix: &str, class: &str, map: &str) -> (Net, LoginFlow) {
    let net = connect("localhost:13327").expect("is the server running? scripts/server.sh run");
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() % 1_000_000;
    let name = format!("{prefix}{stamp}");
    let creds = Creds { account: name.clone(), password: "testpw".into(), new_account: true, quick_character: None };
    let (mut flow, first) = LoginFlow::start(creds);
    for m in first {
        net.tx.send(m).unwrap();
    }
    wait_for(&net, &mut flow, "character list", |m| matches!(m, ServerMsg::AccountPlayers(_)).then_some(()));
    for m in flow.start_creating() {
        net.tx.send(m).unwrap();
    }
    while !(flow.newchar.classes.iter().any(|c| c == class) && flow.newchar.map.is_some()) {
        next(&net, &mut flow, Duration::from_secs(5));
    }
    let n = &mut flow.newchar;
    n.race = Some(n.races[0].clone());
    n.class = Some(class.into());
    n.map = Some(map.into());
    n.refresh_choices();
    n.name = name;
    for seed in 1.. {
        n.roll(seed);
        if n.problem().is_none() {
            break;
        }
    }
    for m in flow.create() {
        net.tx.send(m).unwrap();
    }
    wait_for(&net, &mut flow, "the game to start", |m| matches!(m, ServerMsg::Map2(_)).then_some(()));
    (net, flow)
}

fn say(net: &Net, packet: u16, command: &str) {
    net.tx.send(ClientMsg::Ncom { packet, repeat: 0, command: command.into() }).unwrap();
}

#[test]
#[ignore]
fn read_a_sign_opens_the_box() {
    use crossfir3d::messages::Messages;
    // The Starting House puts you at 9,4; a welcome sign is one step south.
    let (net, mut flow) = play_as("sign", "alchemist_class", "map_tutorial_free_form");
    say(&net, 1, "south");
    say(&net, 2, "apply");
    let mut msgs = Messages::default();
    wait_for(&net, &mut flow, "the sign's text", |m| match m {
        ServerMsg::Text { kind, subtype, text } if text.contains("Welcome to Crossfire!") => {
            msgs.push(*kind, *subtype, text, 1.0, Some(0.5));
            Some(())
        }
        _ => None,
    });
    assert!(msgs.boxed.is_some_and(|b| b.contains("say hello")));
}

#[test]
#[ignore]
fn cast_a_known_spell() {
    // Alchemists start knowing the spell transmutation (arch spell_alchemy).
    let (net, mut flow) = play_as("cast", "alchemist_class", "map_scorn");
    say(&net, 1, &crossfir3d::keys::spell_command("transmutation"));
    wait_for(&net, &mut flow, "the spell readied", |m| match m {
        ServerMsg::Text { text, .. } => text.contains("You ready the spell transmutation").then_some(()),
        _ => None,
    });
}
