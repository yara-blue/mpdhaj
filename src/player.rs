use atomic_float::AtomicF32;
use color_eyre::Result;
use std::{
    fs::File,
    io::BufReader,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use camino::Utf8Path;
use rodio::{Decoder, OutputStream, Source};

mod list_outputs;

struct PlayerParams {
    // range: 0..=1.0, weight such that 10%
    // louder sounds 10% louder
    volume: AtomicF32,
    paused: AtomicBool,
}

struct PlayingHandle {
    abort: AtomicBool,
}

impl PlayerParams {
    fn volume(&self) -> f32 {
        self.volume.load(Ordering::Relaxed)
    }
    fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}

pub struct Player {
    stream: OutputStream,
    params: Arc<PlayerParams>,
    abort_handle: Option<AbortHandle>,
}

/// Aborts the Source this is connected to when it is dropped
#[derive(Clone)]
pub struct AbortHandle(Arc<AtomicBool>);

impl AbortHandle {
    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    fn should_abort(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Drop for AbortHandle {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

impl Player {
    pub fn new(volume: f32, paused: bool) -> Self {
        let stream = rodio::speakers::SpeakersBuilder::new()
            .default_device()
            .unwrap()
            .default_config()
            .unwrap()
            .open_stream()
            .unwrap();

        Self {
            stream,
            params: Arc::new(PlayerParams {
                volume: AtomicF32::new(volume),
                paused: AtomicBool::new(paused),
            }),
            abort_handle: None,
        }
    }

    pub async fn add(&mut self, path: &Utf8Path) -> Result<()> {
        const AUDIO_THREAD_RESPONSE_LATENCY: Duration = Duration::from_millis(50);

        let file = BufReader::new(File::open(path)?);
        let params = Arc::clone(&self.params);
        let abort_handle = AbortHandle::new();

        // this drops any previous abort handle.
        // Causing any playing song to stop
        self.abort_handle = Some(abort_handle.clone());

        let source = Decoder::try_from(file)?
            .stoppable()
            .pausable(params.paused())
            .amplify(1.0)
            .periodic_access(AUDIO_THREAD_RESPONSE_LATENCY, move |source| {
                let amplify = source;
                amplify.set_factor(params.volume());

                let pausable = amplify.inner_mut();
                pausable.set_paused(params.paused());

                let stoppable = pausable.inner_mut();
                if abort_handle.should_abort() {
                    stoppable.stop();
                }
            });

        // ensure the previous song has been stopped before the new one starts
        tokio::time::sleep(AUDIO_THREAD_RESPONSE_LATENCY).await;
        self.stream.mixer().add(source);
        Ok(())
    }

    pub fn pause(&self) {
        self.params.paused.store(true, Ordering::Relaxed);
    }
    pub fn unpause(&self) {
        self.params.paused.store(false, Ordering::Relaxed);
    }
    pub fn set_volume(&self, volume: f32) {
        self.params.volume.store(volume, Ordering::Relaxed);
    }
}
