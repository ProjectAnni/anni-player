pub mod cache;
pub mod identifier;
pub mod provider;
pub mod source;

pub use anni_playback;
pub use anni_provider::providers::TypedPriorityProvider;
use cache::CacheStore;

use std::{
    ops::Deref,
    panic::{RefUnwindSafe, UnwindSafe},
    path::PathBuf,
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver},
        Arc, RwLock,
    },
    thread,
};

use anni_playback::{types::PlayerEvent, Controls, Decoder};
use identifier::TrackIdentifier;
// use once_cell::sync::Lazy;
use provider::{AudioQuality, ProviderProxy};
use reqwest::blocking::Client;

use crate::source::CachedAnnilSource;
// use symphonia::core::io::ReadOnlySource;
// use tokio::runtime::Runtime;
// use tokio_util::io::SyncIoBridge;

// static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

#[derive(Clone)]
pub struct Player {
    pub controls: Controls,
}

impl Player {
    pub fn new() -> (Self, Receiver<PlayerEvent>) {
        let (sender, receiver) = mpsc::channel();
        let controls = Controls::new(sender);
        let thread_killer = anni_playback::create_unbound_channel();

        thread::Builder::new()
            .name("decoder".to_owned())
            .spawn({
                let controls = controls.clone();
                move || {
                    let decoder = Decoder::new(controls, thread_killer.1.clone()); // why clone?

                    decoder.start();
                }
            })
            .unwrap();

        (Self { controls }, receiver)
    }
}

impl Deref for Player {
    type Target = Controls;

    fn deref(&self) -> &Self::Target {
        &self.controls
    }
}

#[derive(Debug, Clone, Default)]
pub struct Playlist {
    pos: Option<usize>,
    tracks: Vec<TrackIdentifier>,
}

impl Playlist {
    pub fn set_item(&mut self, track: TrackIdentifier) {
        self.pos = None;
        self.tracks.clear();
        self.tracks.push(track);
    }

    pub fn next_track(&mut self) -> Option<TrackIdentifier> {
        let pos = match self.pos.as_mut() {
            Some(pos) => {
                *pos += 1;
                pos
            }
            None => self.pos.insert(0),
        };

        self.tracks.get(*pos).copied()
    }

    pub fn push(&mut self, track: TrackIdentifier) {
        self.tracks.push(track);
    }
}

pub struct AnniPlayer {
    pub player: Player,
    pub client: Client,
    provider: RwLock<TypedPriorityProvider<ProviderProxy>>,
    cache_store: CacheStore, // root of cache
}

impl AnniPlayer {
    pub fn new(
        provider: TypedPriorityProvider<ProviderProxy>,
        cache_path: PathBuf,
    ) -> (Self, Receiver<PlayerEvent>) {
        let (player, receiver) = Player::new();

        (
            Self {
                player,
                client: Client::new(),
                provider: RwLock::new(provider),
                cache_store: CacheStore::new(cache_path),
            },
            receiver,
        )
    }

    pub fn add_provider(&self, url: String, auth: String, priority: i32) {
        let mut provider = self.provider.write().unwrap();

        provider.insert(ProviderProxy::new(url, auth, self.client.clone()), priority);
    }

    pub fn clear_provider(&self) {
        let mut provider = self.provider.write().unwrap();

        *provider = TypedPriorityProvider::new(vec![]);
    }

    pub fn load(&self, track: TrackIdentifier, quality: AudioQuality) -> anyhow::Result<()> {
        log::info!("loading track: {track}");

        self.player.pause();

        let provider = self.provider.read().unwrap();

        let buffer_signal = Arc::new(AtomicBool::new(true));
        let source = CachedAnnilSource::new(
            track,
            quality,
            &self.cache_store,
            self.client.clone(),
            &provider,
            buffer_signal.clone(),
        )?;

        self.player.open(Box::new(source), buffer_signal, false);

        Ok(())
    }

    pub fn open(&self, track: TrackIdentifier, quality: AudioQuality) -> anyhow::Result<()> {
        self.load(track, quality)?;
        self.play();

        Ok(())
    }

    pub fn play(&self) {
        self.player.play();
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn open_file(&self, path: String) -> anyhow::Result<()> {
        self.player.open_file(path, false)
    }

    pub fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }

    pub fn seek(&self, position: u64) {
        self.player.seek(position);
    }
}

impl UnwindSafe for AnniPlayer {}
impl RefUnwindSafe for AnniPlayer {}

// pub struct SyncReadWrapper<T> {
//     inner: SyncIoBridge<T>,
// }

// impl<T: > Read
