use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use cfproto::{ClientMsg, MapView, ServerMsg};
use maccrossfire::faces::FaceStore;
use maccrossfire::login::{LoginFlow, LoginState};
use maccrossfire::net::{Net, NetEvent};

mod input;
mod ui;
mod world;

#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum AppState {
    #[default]
    Login,
    InGame,
}

/// The live connection. Non-send: it holds an mpsc Receiver.
pub struct Session {
    pub net: Net,
    pub login: LoginFlow,
}

impl Session {
    pub fn send(&self, msgs: impl IntoIterator<Item = ClientMsg>) {
        for m in msgs {
            let _ = self.net.tx.send(m); // a dead writer shows up as Disconnected
        }
    }
}

#[derive(Resource)]
pub struct Faces(pub FaceStore);

#[derive(Resource, Default)]
pub struct View(pub MapView);

/// The map size the server confirmed in its setup reply.
#[derive(Resource)]
pub struct ViewSize(pub u8, pub u8);

#[derive(Resource)]
pub struct LastComc {
    pub time_ms: u32,
    pub packet: u16,
}

/// ncom packet numbering, shared by movement and the command line.
#[derive(Resource, Default)]
pub struct NcomSeq(pub u16);

impl NcomSeq {
    pub fn next(&mut self) -> u16 {
        self.0 = self.0.wrapping_add(1);
        self.0
    }
}

#[derive(Resource, Default)]
pub struct Status(pub String);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "MacCrossfire".into(), ..default() }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .init_state::<AppState>()
        .init_resource::<Status>()
        .init_resource::<NcomSeq>()
        .init_resource::<ui::LoginForm>()
        .init_resource::<ui::CommandLine>()
        .add_plugins(world::WorldPlugin)
        .add_systems(Startup, ui::auto_login)
        .add_systems(Update, pump_network)
        .add_systems(Update, (input::send_moves, input::send_key_commands).run_if(in_state(AppState::InGame)))
        .add_systems(Update, dev_screenshot.run_if(in_state(AppState::InGame)))
        .add_systems(EguiPrimaryContextPass, ui::login_screen.run_if(in_state(AppState::Login)))
        .add_systems(EguiPrimaryContextPass, ui::command_line.run_if(in_state(AppState::InGame)))
        .run();
}

/// Starts a session after a successful connect.
pub fn start_session(world: &mut World, net: Net, login: LoginFlow, first: Vec<ClientMsg>) {
    let session = Session { net, login };
    session.send(first);
    world.insert_non_send(session);
    world.insert_resource(Faces(FaceStore::new(maccrossfire::faces::default_cache_dir())));
    world.insert_resource(View::default());
    world.insert_resource(ViewSize(11, 11));
    world.insert_resource(LastComc { time_ms: 200, packet: 0 });
    world.insert_resource(NcomSeq::default());
    world.insert_resource(input::MoveWait::default());
}

fn end_session(world: &mut World, status: String) {
    info!("{status}");
    world.remove_non_send::<Session>();
    world.resource_mut::<Status>().0 = status;
    world.resource_mut::<NextState<AppState>>().set(AppState::Login);
}

fn pump_network(world: &mut World) {
    loop {
        let event = {
            let Some(session) = world.get_non_send::<Session>() else { return };
            match session.net.rx.try_recv() {
                Ok(e) => e,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => NetEvent::Disconnected("connection closed".into()),
            }
        };
        let msg = match event {
            NetEvent::Msg(m) => m,
            NetEvent::Disconnected(reason) => return end_session(world, format!("Disconnected: {reason}")),
        };

        let mut session = world.non_send_mut::<Session>();
        let out = session.login.on_message(&msg);
        session.send(out);
        let playing = match session.login.state() {
            LoginState::Failed(reason) => {
                let reason = reason.clone();
                return end_session(world, reason);
            }
            LoginState::Playing => true,
            _ => false,
        };

        match msg {
            ServerMsg::Setup(opts) => {
                if let Some((_, v)) = opts.iter().find(|(k, _)| k == "mapsize") {
                    if let Some((w, h)) = v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))) {
                        world.insert_resource(ViewSize(w, h));
                    }
                }
            }
            ServerMsg::Face2 { num, checksum, name, .. } => {
                let ask = world.resource_mut::<Faces>().0.on_face2(num, checksum, &name);
                world.non_send::<Session>().send(ask);
            }
            ServerMsg::Image2 { face, png, .. } => world.resource_mut::<Faces>().0.on_image2(face, png),
            ServerMsg::Anim { id, faces } => world.resource_mut::<Faces>().0.on_anim(id, faces),
            ServerMsg::Map2(items) => {
                world.resource_mut::<View>().0.apply(&items);
                if playing && *world.resource::<State<AppState>>().get() == AppState::Login {
                    world.resource_mut::<NextState<AppState>>().set(AppState::InGame);
                }
            }
            ServerMsg::NewMap => world.resource_mut::<View>().0.clear(),
            ServerMsg::Comc { packet, time_ms } => world.insert_resource(LastComc { time_ms, packet }),
            ServerMsg::Text(t) => info!("server: {t}"),
            _ => {}
        }
    }
}

/// Developer check: MACCROSSFIRE_SCREENSHOT=file.png saves one screenshot after
/// MACCROSSFIRE_SCREENSHOT_AT seconds in game (default 8), then quits.
fn dev_screenshot(
    mut commands: Commands,
    time: Res<Time>,
    mut since: Local<Option<f32>>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let Ok(path) = std::env::var("MACCROSSFIRE_SCREENSHOT") else { return };
    let start = *since.get_or_insert(time.elapsed_secs());
    let t = time.elapsed_secs() - start;
    let at: f32 = std::env::var("MACCROSSFIRE_SCREENSHOT_AT").ok().and_then(|v| v.parse().ok()).unwrap_or(8.0);
    if (at..at + time.delta_secs()).contains(&t) {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    } else if t > at + 2.0 {
        exit.write(AppExit::Success);
    }
}
