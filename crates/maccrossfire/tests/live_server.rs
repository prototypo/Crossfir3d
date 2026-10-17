//! Needs a local server: scripts/server.sh run, then
//! cargo test -p maccrossfire --test live_server -- --ignored

use std::time::{Duration, Instant, SystemTime};

use cfproto::{ClientMsg, Map2Item, ServerMsg};
use maccrossfire::login::{Creds, LoginFlow};
use maccrossfire::net::{connect, Net, NetEvent};

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
    let creds = Creds { account: name.clone(), password: "testpw".into(), character: name, new_account: true };
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

#[test]
#[ignore]
fn pick_up_and_drop() {
    let net = connect("localhost:13327").expect("is the server running? scripts/server.sh run");
    let stamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() % 1_000_000;
    let name = format!("drop{stamp}");
    let creds = Creds { account: name.clone(), password: "testpw".into(), character: name, new_account: true };
    let (mut flow, first) = LoginFlow::start(creds);
    for m in first {
        net.tx.send(m).unwrap();
    }
    let player = wait_for(&net, &mut flow, "player", |m| match m {
        ServerMsg::Player { tag, .. } => Some(*tag),
        _ => None,
    });
    // A starting item that is neither locked nor worn.
    let (tag, nrof) = wait_for(&net, &mut flow, "a droppable item", |m| match m {
        ServerMsg::Item2 { location, items } if *location == player => {
            items.iter().find(|i| !i.locked() && !i.applied()).map(|i| (i.tag, i.nrof))
        }
        _ => None,
    });

    net.tx.send(ClientMsg::Move { to: 0, tag, count: nrof }).unwrap();
    wait_for(&net, &mut flow, "the dropped item on the ground", |m| match m {
        ServerMsg::Item2 { location: 0, items } => items.iter().any(|i| i.tag == tag).then_some(()),
        _ => None,
    });

    net.tx.send(ClientMsg::Move { to: player, tag, count: nrof }).unwrap();
    wait_for(&net, &mut flow, "the item back in the inventory", |m| match m {
        ServerMsg::Item2 { location, items } if *location == player => items.iter().any(|i| i.tag == tag).then_some(()),
        _ => None,
    });
}
