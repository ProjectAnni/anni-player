use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use anni_playback::types::MediaSource;
use reqwest::{blocking::Client, Url};

pub struct CachedHttpSource {
    cache: File,
    buf_len: Arc<AtomicUsize>,
    pos: usize,
    buffer_signal: Arc<AtomicBool>,
}

impl CachedHttpSource {
    pub fn new(
        url: Url,
        cache_path: &Path,
        client: Client,
        buffer_signal: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        if cache_path.exists() {
            let cache = File::open(cache_path)?;
            let buf_len = Arc::new(AtomicUsize::new(cache.metadata()?.len() as usize));

            Ok(Self {
                cache,
                buf_len,
                pos: 0,
                buffer_signal,
            })
        } else {
            let buf_len = Arc::new(AtomicUsize::new(0));

            thread::spawn({
                let mut response = client.get(url).send()?;

                let mut cache = File::options().append(true).create(true).open(cache_path)?;
                let buf_len = Arc::clone(&buf_len);
                let mut buf = [0; 1024 * 64];
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

            // sleep 1s to buffer data for decoder.
            // thread::sleep(Duration::from_secs(1));

            let cache = File::options()
                .write(true)
                .read(true)
                .create(true)
                .open(cache_path)?;

            Ok(Self {
                cache,
                buf_len,
                pos: 0,
                buffer_signal,
            })
        }
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
