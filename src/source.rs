use std::{
    fs::{create_dir_all, File},
    io::{ErrorKind, Read, Seek, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use anni_playback::types::MediaSource;
use anni_provider::providers::TypedPriorityProvider;
use anyhow::anyhow;

use reqwest::{blocking::Client, Url};

use crate::{identifier::TrackIdentifier, provider::ProviderProxy};

const BUF_SIZE: usize = 1024 * 64; // 64k

pub struct CachedHttpSource {
    cache: File,
    buf_len: Arc<AtomicUsize>,
    pos: usize,
    buffer_signal: Arc<AtomicBool>,
}

impl CachedHttpSource {
    /// `cache_path` is the path to cache file.
    pub fn new(
        url: Url,
        cache_path: &Path,
        client: Client,
        buffer_signal: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        let (cache, buf_len) = if cache_path.exists() {
            // todo: varify cache
            let cache = File::open(cache_path)?;
            let buf_len = cache.metadata()?.len() as usize;

            (cache, Arc::new(AtomicUsize::new(buf_len)))
        } else {
            create_parent(cache_path)?;

            let cache = File::options()
                .write(true)
                .read(true)
                .create(true)
                .open(cache_path)?;

            let buf_len = Arc::new(AtomicUsize::new(0));

            thread::spawn({
                let mut response = client.get(url).send()?;

                let mut cache = cache.try_clone()?;
                let buf_len = Arc::clone(&buf_len);
                let mut buf = [0; BUF_SIZE];
                let buffer_signal = Arc::clone(&buffer_signal);

                move || loop {
                    match response.read(&mut buf) {
                        Ok(0) => {
                            buffer_signal.store(false, Ordering::Relaxed);
                            break;
                        }
                        Ok(n) => {
                            if let Err(e) = cache.write_all(&buf[..n]) {
                                log::error!("{e}")
                            }
                            let _ = cache.flush();

                            log::trace!("wrote {n} bytes");

                            buf_len.fetch_add(n, Ordering::AcqRel);
                        }
                        Err(e) if e.kind() == ErrorKind::Interrupted => {}
                        Err(e) => log::error!("{e}"),
                    }
                }
            });

            (cache, buf_len)
        };

        Ok(Self {
            cache,
            buf_len,
            pos: 0,
            buffer_signal,
        })
    }
}

impl Read for CachedHttpSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // let n = self.cache.read(buf)?;
        // self.pos.fetch_add(n, Ordering::AcqRel);
        // log::trace!("read {n} bytes");
        // Ok(n)

        loop {
            let has_buf = self.buf_len.load(Ordering::Acquire) > self.pos;

            if has_buf || !self.buffer_signal.load(Ordering::Acquire) {
                let n = self.cache.read(buf)?;
                self.pos += n;
                log::trace!("read {n} bytes");
                break Ok(n);
            }
        }
    }
}

impl Seek for CachedHttpSource {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let p = self.cache.seek(pos)?;
        self.pos = p as usize;
        Ok(p)
    }
}

impl MediaSource for CachedHttpSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        let len = self.buf_len.load(Ordering::Acquire) as u64;
        log::trace!("returning buf_len {len}");
        Some(len)
    }
}

pub struct CachedAnnilSource(CachedHttpSource);

impl CachedAnnilSource {
    pub fn new(
        track: TrackIdentifier,
        cache_path: &Path,
        client: Client,
        provider: &TypedPriorityProvider<ProviderProxy>,
        buffer_signal: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        let source = provider
            .providers()
            .map(|p| p.head(track))
            .collect::<One<_>>()
            .0
            .ok_or(anyhow!("No audio"))?
            .url()
            .clone();

        CachedHttpSource::new(source, cache_path, client, buffer_signal).map(Self)
    }
}

impl Read for CachedAnnilSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Seek for CachedAnnilSource {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

impl MediaSource for CachedAnnilSource {
    fn is_seekable(&self) -> bool {
        self.0.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.0.byte_len()
    }
}

fn create_parent(p: &Path) -> std::io::Result<()> {
    let mut ancestor = p.ancestors();
    let parent = ancestor.nth(1).unwrap();

    match create_dir_all(parent) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}

struct One<T>(pub Option<T>);

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
