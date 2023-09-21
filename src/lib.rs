pub mod cache;
pub mod identifier;
pub mod provider;
pub mod source;

pub use anni_provider::providers::TypedPriorityProvider;

use std::{
    ops::Deref,
    sync::{
        atomic::AtomicBool,
        mpsc::{self, Receiver},
        Arc, RwLock,
    },
    thread,
};

use anni_playback::{types::PlayerEvent, Controls, Decoder};
use anyhow::Context;
use identifier::TrackIdentifier;
// use once_cell::sync::Lazy;
use provider::ProviderProxy;
use reqwest::blocking::{Client, Response};

use crate::source::CachedHttpSource;
// use symphonia::core::io::ReadOnlySource;
// use tokio::runtime::Runtime;
// use tokio_util::io::SyncIoBridge;

// static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

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
    player: Player,
    playlist: RwLock<Playlist>,
    // receiver: Receiver<PlayerEvent>,
    provider: TypedPriorityProvider<ProviderProxy>,
}

impl AnniPlayer {
    pub fn new(provider: TypedPriorityProvider<ProviderProxy>) -> (Self, Receiver<PlayerEvent>) {
        let (player, receiver) = Player::new();

        (
            Self {
                player,
                playlist: Default::default(),
                // receiver,
                provider,
            },
            receiver,
        )
    }

    // pub fn open(&self, track: TrackIdentifier) -> anyhow::Result<()> {
    // let mut pl = self.playlist.write().unwrap();
    // pl.set_item(track);
    //
    // let next = pl.next().context("empty playlist")?;
    // let album_id = next.album_id.to_string();
    // let audio = RUNTIME.block_on(self.provider.get_audio(&album_id, next.disc_id, next.track_id, Range::FULL))?;
    // let bridge = SyncIoBridge::new(audio.reader);
    // self.player.open(Box::new(ReadOnlySource::new(bridge)) as _, Arc::new(AtomicBool::new(true)), false);
    //
    // todo!()
    // }

    fn play_track(&self, track: TrackIdentifier) -> anyhow::Result<()> {
        log::info!("opening track: {track}");

        // self.pause();

        let source = self
            .provider
            .providers()
            .map(|p| p.head(track))
            .collect::<One<Response>>()
            .unwrap()
            .url()
            .clone();

        let buffer_signal = Arc::new(AtomicBool::new(true));
        // let source = HttpStream::new(source.to_string(), Arc::clone(&buffer_signal))?;
        let source = CachedHttpSource::new(
            source,
            format!("D:/temp/{track}").as_ref(),
            Client::new(),
            Arc::clone(&buffer_signal),
        )?;

        self.player.open(Box::new(source), buffer_signal, false);

        Ok(())
    }

    pub fn open(&self, track: TrackIdentifier) -> anyhow::Result<()> {
        let mut pl = self.playlist.write().unwrap();
        pl.set_item(track);

        self.play_track(track)?;
        self.play();

        Ok(())
    }

    pub fn play_next(&self) -> anyhow::Result<()> {
        let mut pl = self.playlist.write().unwrap();

        let track = pl.next_track().context("end of playlist")?;
        self.play_track(track)?;
        self.play();

        Ok(())
    }

    pub fn push_track(&self, track: TrackIdentifier) {
        log::info!("adding track {track} to playlist");

        let mut pl = self.playlist.write().unwrap();
        pl.push(track);
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

struct One<T>(pub Option<T>);

impl<T> One<T> {
    fn unwrap(self) -> T {
        self.0.unwrap()
    }
}

impl<T, E: std::error::Error> FromIterator<Result<T, E>> for One<T> {
    fn from_iter<I: IntoIterator<Item = Result<T, E>>>(iter: I) -> Self {
        for item in iter {
            match item {
                Ok(r) => return Self(Some(r)),
                Err(e) => log::warn!("{e}"),
            }
        }

        Self(None)
    }
}

// pub struct SyncReadWrapper<T> {
//     inner: SyncIoBridge<T>,
// }

// impl<T: > Read
