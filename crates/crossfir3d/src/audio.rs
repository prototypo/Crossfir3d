//! Plays the server's sound effects and music from the crossfire-sounds checkout.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use bevy::audio::Volume;
use bevy::prelude::*;
use crossfir3d::sound::{music_changes, music_file, sounds_dir, SoundTable};

use crossfir3d::modes::mute_key_allowed;

use crate::graphics::CurrentSettings;
use crate::{AppState, KeyMode};

/// Sounds and music the network pump received this frame.
#[derive(Resource, Default)]
pub struct AudioQueue {
    /// (action, volume 0-100, dx, dy)
    pub sounds: Vec<(String, u8, i8, i8)>,
    pub music: Option<String>,
}

#[derive(Resource)]
struct Sounds {
    table: SoundTable,
    dir: PathBuf,
    loaded: HashMap<PathBuf, Handle<AudioSource>>,
    warned: HashSet<String>,
    muted: bool,
    /// The song playing now, as the server named it.
    current: Option<String>,
}

#[derive(Component)]
struct Music;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        let dir = sounds_dir();
        app.insert_resource(Sounds {
            table: SoundTable::load(&dir),
            dir,
            loaded: HashMap::new(),
            warned: HashSet::new(),
            muted: false,
            current: None,
        })
        .init_resource::<AudioQueue>()
        .add_systems(Update, (toggle_mute, play_queued, pause_music).chain())
        .add_systems(OnExit(AppState::InGame), stop_music);
    }
}

impl Sounds {
    /// Loads a file once; None (and one warning per name) if it is missing.
    fn handle(&mut self, path: PathBuf, sources: &mut Assets<AudioSource>) -> Option<Handle<AudioSource>> {
        if let Some(h) = self.loaded.get(&path) {
            return Some(h.clone());
        }
        match std::fs::read(&path) {
            Ok(bytes) if !decodes(&bytes) => {
                if self.warned.insert(path.display().to_string()) {
                    warn!("sound {}: not a sound file this game can play", path.display());
                }
                None
            }
            Ok(bytes) => {
                let h = sources.add(AudioSource { bytes: Arc::from(bytes) });
                self.loaded.insert(path, h.clone());
                Some(h)
            }
            Err(e) => {
                if self.warned.insert(path.display().to_string()) {
                    warn!("sound {}: {e}", path.display());
                }
                None
            }
        }
    }
}

fn toggle_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<KeyMode>,
    state: Res<State<AppState>>,
    mut sounds: ResMut<Sounds>,
    (bindings, pending): (Res<crate::KeyBindings>, Res<crate::ui::PendingBind>),
) {
    // A bound M (or option+M, ...) runs its binding instead; a typed `bind` may be taking it.
    let taken = crate::input::key_taken(&bindings, &keys, KeyCode::KeyM) || pending.waiting_key();
    if mute_key_allowed(*state.get() == AppState::InGame, mode.0) && keys.just_pressed(KeyCode::KeyM) && !taken {
        sounds.muted = !sounds.muted;
    }
}

/// Music plays unless everything is muted (M) or the Music setting is off.
fn pause_music(sounds: Res<Sounds>, settings: Res<CurrentSettings>, music: Query<&AudioSink, With<Music>>) {
    let quiet = sounds.muted || !settings.0.music;
    for sink in &music {
        if quiet && !sink.is_paused() {
            sink.pause();
        } else if !quiet && sink.is_paused() {
            sink.play();
        }
    }
}

/// Whether Bevy can play these bytes: its decoder panics on a damaged file.
/// ponytail: swaps the global panic hook for a moment (once per file); a rodio dependency would avoid it.
fn decodes(bytes: &[u8]) -> bool {
    use bevy::audio::Decodable;
    let source = AudioSource { bytes: Arc::from(bytes) };
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(source.decoder()))).is_ok();
    std::panic::set_hook(quiet);
    ok
}

/// Logging out or dying ends the music; the next login starts its own.
fn stop_music(mut commands: Commands, mut sounds: ResMut<Sounds>, playing: Query<Entity, With<Music>>) {
    for e in &playing {
        commands.entity(e).despawn();
    }
    sounds.current = None;
}

fn play_queued(
    mut commands: Commands,
    mut queue: ResMut<AudioQueue>,
    mut sounds: ResMut<Sounds>,
    mut sources: ResMut<Assets<AudioSource>>,
    playing: Query<Entity, With<Music>>,
) {
    for (action, volume, dx, dy) in std::mem::take(&mut queue.sounds) {
        if sounds.muted {
            continue;
        }
        let Some((file, conf_volume)) = sounds.table.lookup(&action).map(|(f, v)| (f.to_string(), v)) else {
            if sounds.warned.insert(action.clone()) {
                info!("no sound for '{action}'");
            }
            continue;
        };
        let path = sounds.dir.join(file);
        let Some(h) = sounds.handle(path, &mut sources) else { continue };
        let distance = 1.0 + (dx as f32).powi(2) + (dy as f32).powi(2);
        let v = (volume as f32 / 100.0) * (conf_volume.min(100) as f32 / 100.0) / distance;
        commands.spawn((AudioPlayer(h), PlaybackSettings { volume: Volume::Linear(v), ..PlaybackSettings::DESPAWN }));
    }
    if let Some(name) = queue.music.take() {
        if !music_changes(sounds.current.as_deref(), &name) {
            return;
        }
        for e in &playing {
            commands.entity(e).despawn();
        }
        sounds.current = (name != "NONE").then(|| name.clone());
        if name == "NONE" {
            return;
        }
        let Some(path) = music_file(&sounds.dir, &name) else {
            info!("no music file for '{name}'");
            return;
        };
        let Some(h) = sounds.handle(path, &mut sources) else { return };
        // Starts paused; `pause_music` plays it if music is wanted.
        let settings = PlaybackSettings { paused: true, volume: Volume::Linear(0.5), ..PlaybackSettings::LOOP };
        commands.spawn((Music, AudioPlayer(h), settings));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// A tiny valid WAV: 16-bit mono PCM, 4 silent samples.
    fn wav() -> Vec<u8> {
        let data = [0u8; 8];
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&1u16.to_le_bytes()); // mono
        b.extend_from_slice(&8000u32.to_le_bytes());
        b.extend_from_slice(&16000u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        b
    }

    #[test]
    fn damaged_sound_is_refused() {
        assert!(decodes(&wav()));
        assert!(!decodes(b"this is not a sound file at all"));
    }

    #[test]
    fn leaving_the_game_stops_the_music() {
        let mut app = App::new();
        app.insert_resource(Sounds {
            table: SoundTable::default(),
            dir: PathBuf::new(),
            loaded: HashMap::new(),
            warned: HashSet::new(),
            muted: false,
            current: Some("town".into()),
        });
        let music = app.world_mut().spawn(Music).id();
        app.world_mut().run_system_once(stop_music).unwrap();
        assert!(app.world().get_entity(music).is_err());
        assert_eq!(app.world().resource::<Sounds>().current, None);
    }
}
