use std::{
    fs::{self, File},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use symphonia::{
    core::{
        codecs::{CodecRegistry, DecoderOptions},
        formats::FormatOptions,
        io::MediaSourceStream,
        meta::MetadataOptions,
        probe::Hint,
    },
    default::{get_probe, register_enabled_codecs},
};

use crate::identifier::TrackIdentifier;

// todo: enable opus support (which is private code in anni-playback)
static CODEC_REGISTRY: Lazy<CodecRegistry> = Lazy::new(|| {
    let mut registry = CodecRegistry::new();
    register_enabled_codecs(&mut registry);
    registry
});

#[derive(Debug, Clone)]
pub struct CacheStore {
    base: PathBuf,
}

impl CacheStore {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    // Returns the path to given `track`
    pub fn loaction_of(&self, track: TrackIdentifier) -> PathBuf {
        let mut tmp = self.base.clone();
        tmp.extend([
            track.album_id.to_string(),
            track.disc_id.to_string(),
            track.track_id.to_string(),
        ]);
        tmp
    }

    pub fn acquire(&self, track: TrackIdentifier) -> io::Result<Result<File, File>> {
        let path = self.loaction_of(track);

        if path.exists() {
            if validate_audio(&path).unwrap_or(false) {
                return File::open(path).map(|f| Ok(f));
            }

            log::warn!("cache of {track} exists but is invalid");
        }

        create_dir_all(path.parent().unwrap())?; // parent of `path` exists

        File::options()
            .read(true)
            .append(true)
            .create(true)
            .open(path)
            .map(|f| Ok(f))
    }

    pub fn add(&self, path: &Path, track: TrackIdentifier) -> io::Result<()> {
        let location = self.loaction_of(track);

        if location.exists() {
            Err(ErrorKind::AlreadyExists.into())
        } else if validate_audio(path).unwrap_or(false) {
            fs::copy(path, location).map(|_| {})
        } else {
            Err(io::Error::new(ErrorKind::Other, "invalid cache"))
        }
    }
}

pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    match fs::create_dir_all(path.as_ref()) {
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
        r => r,
    }
}

pub fn validate_audio(p: &Path) -> symphonia::core::errors::Result<bool> {
    let source = MediaSourceStream::new(Box::new(File::open(p)?), Default::default());

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = get_probe().format(&Hint::new(), source, &format_opts, &metadata_opts)?;

    let mut format_reader = probed.format;
    let track = match format_reader.default_track() {
        Some(track) => track,
        None => return Ok(false),
    };

    let mut decoder = CODEC_REGISTRY.make(&track.codec_params, &DecoderOptions { verify: true })?;

    while let Ok(packet) = format_reader.next_packet() {
        let _ = decoder.decode(&packet)?;
    }

    Ok(decoder.finalize().verify_ok.unwrap_or(false))
}
