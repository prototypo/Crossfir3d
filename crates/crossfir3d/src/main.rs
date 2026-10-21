use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};
use cfproto::{ClientMsg, Inventory, MapView, ServerMsg, SpellBook, Stats};
use crossfir3d::faces::FaceStore;
use crossfir3d::login::{LoginFlow, LoginState};
use crossfir3d::clock::{is_time_reply, is_time_reply_line, parse_time_reply, GameClock};
use crossfir3d::messages::Messages;
use crossfir3d::modes::Mode;
use crossfir3d::slots::{default_slots_path, load_slots, QuickSlots};
use crossfir3d::net::{Net, NetEvent};

mod audio;
mod characters3d;
mod characters_ui;
mod combat_fx;
mod graphics;
mod hud;
mod input;
mod settings_ui;
mod sky;
mod inventory_ui;
mod keys_ui;
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

/// Which part of the screen owns the keyboard.
#[derive(Resource, Default)]
pub struct KeyMode(pub Mode);

/// When the player last did something (apply, slot, inventory action, command), in
/// seconds of game time. Decides whether a sign or NPC message opens the box.
#[derive(Resource, Default)]
pub struct LastAction(pub Option<f32>);

#[derive(Resource, Default)]
pub struct Msgs(pub Messages);

/// Crossfire's game clock, synced from our own `time` request.
#[derive(Resource, Default)]
pub struct Clock(pub GameClock);

/// When we sent `time`; its reply is used for the clock and not shown.
#[derive(Resource, Default)]
pub struct TimeRequest(pub Option<f32>);

/// The server asked a question; the next command box line is the answer.
#[derive(Resource, Default)]
pub struct PendingQuery(pub bool);

#[derive(Resource, Default)]
pub struct PlayerStats(pub Stats);

#[derive(Resource, Default)]
pub struct Inv(pub Inventory);

#[derive(Resource, Default)]
pub struct Spells(pub SpellBook);

#[derive(Resource, Default)]
pub struct Slots(pub QuickSlots);

/// This character's key bindings and the all-characters ones (`keys.txt`).
#[derive(Resource, Default)]
pub struct KeyBindings(pub crossfir3d::keys::Bindings);

/// Skill names by stat code (140-189), from `replyinfo skill_info`.
#[derive(Resource, Default)]
pub struct SkillNames(pub std::collections::HashMap<u8, String>);

/// The skills this character knows, by name, sorted.
pub fn known_skills(stats: &Stats, names: &SkillNames) -> Vec<String> {
    let mut known: Vec<String> = stats.skills.keys().filter_map(|c| names.0.get(c).cloned()).collect();
    known.sort();
    known
}

#[derive(Resource, Default)]
pub struct HudState {
    pub tall_log: bool,
}

fn main() {
    if let Some(home) = std::env::var_os("HOME") {
        crossfir3d::settings::move_old_folders(std::path::Path::new(&home));
    }
    let settings = graphics::startup_settings();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin { primary_window: Some(graphics::initial_window(&settings.0)), ..default() })
                .set(AssetPlugin { file_path: world::ASSET_DIR.into(), ..default() }),
        )
        .insert_resource(settings)
        .add_plugins(graphics::GraphicsPlugin)
        .add_plugins(EguiPlugin::default())
        // egui goes on the window camera (graphics.rs), not the 3D camera that draws into an image.
        .insert_resource(bevy_egui::EguiGlobalSettings { auto_create_primary_context: false, ..default() })
        .init_state::<AppState>()
        .init_resource::<Status>()
        .init_resource::<NcomSeq>()
        .init_resource::<ui::LoginForm>()
        .init_resource::<ui::CommandLine>()
        .init_resource::<KeyMode>()
        .init_resource::<LastAction>()
        .init_resource::<Msgs>()
        .init_resource::<PendingQuery>()
        .init_resource::<Clock>()
        .init_resource::<TimeRequest>()
        .add_systems(OnEnter(AppState::InGame), (ask_time, ask_skills, dev_commands))
        .init_resource::<HudState>()
        .init_resource::<PlayerStats>()
        .init_resource::<Inv>()
        .init_resource::<Spells>()
        .init_resource::<Slots>()
        .init_resource::<SkillNames>()
        .init_resource::<KeyBindings>()
        .init_resource::<ui::PendingBind>()
        .init_resource::<inventory_ui::InvSel>()
        .init_resource::<keys_ui::KeysSel>()
        .init_resource::<input::Facing>()
        .init_resource::<input::CameraHeading>()
        .init_resource::<input::CameraView>()
        .add_systems(EguiPrimaryContextPass, (hud::hud, inventory_ui::inventory_screen, keys_ui::keys_screen, combat_fx::draw).chain().run_if(in_state(AppState::InGame)))
        .init_resource::<combat_fx::CombatFx>()
        .add_systems(Update, combat_fx::watch.run_if(in_state(AppState::InGame)))
        .add_plugins(world::WorldPlugin)
        .add_plugins(characters3d::CharactersPlugin)
        .add_plugins(audio::AudioPlugin)
        .add_systems(Startup, ui::auto_login)
        .add_systems(Update, pump_network)
        .add_systems(Update, (input::send_moves, input::world_keys, ui::capture_bind_key, input::watch_ground).chain().run_if(in_state(AppState::InGame)))
        .add_systems(Update, dev_screenshot)
        .add_systems(OnEnter(AppState::InGame), load_character_keys)
        .init_resource::<settings_ui::SettingsOpen>()
        .add_systems(
            EguiPrimaryContextPass,
            (ui::login_screen, characters_ui::characters_screen).run_if(in_state(AppState::Login)),
        )
        .add_systems(EguiPrimaryContextPass, settings_ui::settings_screen)
        .add_systems(EguiPrimaryContextPass, ui::command_line.run_if(in_state(AppState::InGame)))
        .run();
}

/// Starts a session after a successful connect.
pub fn start_session(world: &mut World, net: Net, login: LoginFlow, first: Vec<ClientMsg>) {
    let session = Session { net, login };
    session.send(first);
    world.insert_non_send(session);
    world.insert_resource(Faces(FaceStore::new(crossfir3d::faces::default_cache_dir())));
    world.insert_resource(ViewSize(11, 11)); // until the setup reply says otherwise
    reset_play_state(world);
}

/// Everything about the character being played; reset per session and when going back
/// to the character list.
fn reset_play_state(world: &mut World) {
    world.insert_resource(View::default());
    world.insert_resource(LastComc { time_ms: 200, packet: 0 });
    world.insert_resource(NcomSeq::default());
    world.insert_resource(input::MoveWait::default());
    world.insert_resource(KeyMode::default());
    world.insert_resource(LastAction::default());
    world.insert_resource(Msgs::default());
    world.insert_resource(PendingQuery::default());
    world.insert_resource(Clock::default());
    world.insert_resource(TimeRequest::default());
    world.insert_resource(HudState::default());
    world.insert_resource(PlayerStats::default());
    world.insert_resource(Inv::default());
    world.insert_resource(Spells::default());
    world.insert_resource(Slots::default());
    world.insert_resource(SkillNames::default());
    world.insert_resource(ui::PendingBind::default());
    world.insert_resource(inventory_ui::InvSel::default());
    world.insert_resource(keys_ui::KeysSel::default());
    world.insert_resource(input::Facing::default());
    world.insert_resource(input::CameraHeading::default());
    world.insert_resource(input::CameraView::default());
    *world.resource_mut::<ui::CommandLine>() = ui::CommandLine::default();
}

pub fn end_session(world: &mut World, status: String) {
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

        // On a permadeath server the character list comes back after death.
        let in_game = *world.resource::<State<AppState>>().get() == AppState::InGame;
        if let (true, Some(reason)) = (in_game, crossfir3d::messages::in_game_reset(&msg)) {
            world.resource_mut::<Status>().0 = reason.to_string();
            world.resource_mut::<NextState<AppState>>().set(AppState::Login);
            reset_play_state(world);
        }
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
            ServerMsg::NewMap => {
                world.resource_mut::<View>().0.clear();
                world.resource_mut::<input::DoorExit>().new_map = true;
                world.resource_mut::<world::MapGen>().0 += 1;
                // A new map after dying is the respawn.
                world.resource_mut::<characters3d::PlayerAnimEvents>().0.push(crossfir3d::anim::AnimEvent::Revived);
            }
            ServerMsg::Comc { packet, time_ms } => world.insert_resource(LastComc { time_ms, packet }),
            ServerMsg::Tick(t) => world.resource_mut::<Clock>().0.on_tick(t),
            // Every line of the reply to our own `time` request is for the clock, not the player.
            ServerMsg::Text { text, .. } if time_reply_expected(world, &text) => {
                if is_time_reply(&text) {
                    match parse_time_reply(&text) {
                        Some((hour, minute)) => {
                            world.resource_mut::<Clock>().0.sync(hour, minute);
                            info!("game clock synced: hour {hour}, minute {minute}");
                        }
                        None => warn!("could not read the time: {text}"),
                    }
                }
            }
            ServerMsg::Text { kind, subtype, text } => {
                info!("server: {text}");
                let (now, last) = (world.resource::<Time>().elapsed_secs(), world.resource::<LastAction>().0);
                world.resource_mut::<Msgs>().0.push(kind, subtype, &text, now, last);
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
                world.resource_mut::<Msgs>().0.push(0, 0, &format!("Server asks: {text} (answer with Enter)"), now, None);
            }
            ServerMsg::Stats(updates) => world.resource_mut::<PlayerStats>().0.apply(&updates),
            ServerMsg::Player { tag, .. } => world.resource_mut::<Inv>().0.player = tag,
            ServerMsg::Item2 { location, items } => world.resource_mut::<Inv>().0.apply_item2(location, items),
            ServerMsg::UpdItem { tag, changes } => world.resource_mut::<Inv>().0.apply_upd(tag, &changes),
            ServerMsg::DelItem(tags) => world.resource_mut::<Inv>().0.delete(&tags),
            ServerMsg::DelInv(tag) => world.resource_mut::<Inv>().0.delete_inside(tag),
            ServerMsg::AddSpell(spells) => world.resource_mut::<Spells>().0.add(spells),
            ServerMsg::UpdSpell { tag, mana, grace, damage } => world.resource_mut::<Spells>().0.update(tag, mana, grace, damage),
            ServerMsg::DelSpell(tag) => world.resource_mut::<Spells>().0.remove(tag),
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
}

/// Quick slots and key bindings belong to the character actually being played.
fn load_character_keys(mut slots: ResMut<Slots>, mut bindings: ResMut<KeyBindings>, session: Option<NonSend<Session>>) {
    if let Some(s) = session {
        slots.0 = load_slots(&default_slots_path(), s.login.character());
        bindings.0 = crossfir3d::keys::load_bindings(&crossfir3d::keys::default_keys_path(), s.login.character());
    }
}

/// Developer check: CROSSFIR3D_SCREENSHOT=file.png saves one screenshot after
/// CROSSFIR3D_SCREENSHOT_AT seconds after start (default 8), then quits.
fn dev_screenshot(
    mut commands: Commands,
    time: Res<Time>,
    mut since: Local<Option<f32>>,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    let Ok(path) = std::env::var("CROSSFIR3D_SCREENSHOT") else { return };
    let start = *since.get_or_insert(time.elapsed_secs());
    let t = time.elapsed_secs() - start;
    let at: f32 = std::env::var("CROSSFIR3D_SCREENSHOT_AT").ok().and_then(|v| v.parse().ok()).unwrap_or(8.0);
    if (at..at + time.delta_secs()).contains(&t) {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    } else if t > at + 2.0 {
        exit.write(AppExit::Success);
    }
}

/// A reply line to our own `time` request (within 10 s) is for the clock, not the player.
fn time_reply_expected(world: &World, text: &str) -> bool {
    let now = world.resource::<Time>().elapsed_secs();
    world.resource::<TimeRequest>().0.is_some_and(|t| now - t <= 10.0) && is_time_reply_line(text)
}

/// Asks the server for the game time once per login.
/// Developer check: CROSSFIR3D_COMMANDS="cmd1;cmd2" sends these game commands on
/// entering the game (e.g. a local-server teleport for screenshots).
/// CROSSFIR3D_SCREEN=keys opens the key bindings screen.
fn dev_commands(
    mut seq: ResMut<NcomSeq>,
    mut mode: ResMut<KeyMode>,
    mut facing: ResMut<input::Facing>,
    mut heading: ResMut<input::CameraHeading>,
    session: Option<NonSend<Session>>,
) {
    if std::env::var("CROSSFIR3D_SCREEN").is_ok_and(|s| s == "keys") {
        mode.0 = crossfir3d::modes::Mode::Keys;
    }
    let (Ok(cmds), Some(session)) = (std::env::var("CROSSFIR3D_COMMANDS"), session) else { return };
    for c in cmds.split(';').map(str::trim).filter(|c| !c.is_empty()) {
        facing.0 = crossfir3d::slots::dir_number(c).unwrap_or(facing.0);
        heading.0 = crossfir3d::camera::heading_of(facing.0);
        session.send([ClientMsg::Ncom { packet: seq.next(), repeat: 0, command: c.into() }]);
    }
}

fn ask_skills(session: Option<NonSend<Session>>) {
    if let Some(session) = session {
        session.send([ClientMsg::RequestInfo("skill_info".into())]);
    }
}

fn ask_time(time: Res<Time>, mut request: ResMut<TimeRequest>, mut seq: ResMut<NcomSeq>, session: Option<NonSend<Session>>) {
    let Some(session) = session else { return };
    session.send([ClientMsg::Ncom { packet: seq.next(), repeat: 0, command: "time".into() }]);
    request.0 = Some(time.elapsed_secs());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_skills_are_named_and_sorted() {
        let mut stats = Stats::default();
        stats.apply(&[(142, cfproto::StatValue::Int(5)), (140, cfproto::StatValue::Int(9)), (150, cfproto::StatValue::Int(1))]);
        let names = SkillNames([(140, "praying".to_string()), (142, "alchemy".to_string())].into());
        assert_eq!(known_skills(&stats, &names), ["alchemy", "praying"]);
    }

    #[test]
    fn back_to_list_keeps_server_view_size() {
        // The server sends the view size once per connection (setup reply).
        let mut world = World::new();
        world.init_resource::<ui::CommandLine>();
        world.insert_resource(ViewSize(25, 25));
        reset_play_state(&mut world);
        let size = world.resource::<ViewSize>();
        assert_eq!((size.0, size.1), (25, 25));
    }
}
