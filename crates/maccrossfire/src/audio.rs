//! Plays the server's sound effects and music from the crossfire-sounds checkout.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use bevy::audio::Volume;
use bevy::prelude::*;
use maccrossfire::sound::{sounds_dir, SoundTable};

use crate::ui::CommandLine;

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
        })
        .init_resource::<AudioQueue>()
        .add_systems(Update, (toggle_mute, play_queued).chain());
    }
}

impl Sounds {
    /// Loads a file once; None (and one warning per name) if it is missing.
    fn handle(&mut self, path: PathBuf, sources: &mut Assets<AudioSource>) -> Option<Handle<AudioSource>> {
        if let Some(h) = self.loaded.get(&path) {
            return Some(h.clone());
        }
        match std::fs::read(&path) {
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
    line: Res<CommandLine>,
    mut sounds: ResMut<Sounds>,
    music: Query<&AudioSink, With<Music>>,
) {
    if line.open || !keys.just_pressed(KeyCode::KeyM) {
        return;
    }
    sounds.muted = !sounds.muted;
    for sink in &music {
        if sounds.muted { sink.pause() } else { sink.play() }
    }
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
        for e in &playing {
            commands.entity(e).despawn();
        }
        if name == "NONE" {
            return;
        }
        let found = ["ogg", "mp3"].iter().map(|ext| sounds.dir.join("music").join(format!("{name}.{ext}"))).find(|p| p.exists());
        let Some(path) = found else {
            info!("no music file for '{name}'");
            return;
        };
        let Some(h) = sounds.handle(path, &mut sources) else { return };
        let settings = PlaybackSettings { paused: sounds.muted, volume: Volume::Linear(0.5), ..PlaybackSettings::LOOP };
        commands.spawn((Music, AudioPlayer(h), settings));
    }
}
