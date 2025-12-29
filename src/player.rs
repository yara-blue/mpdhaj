use atomic_float::AtomicF32;
use color_eyre::Result;
use std::{
    fs::File,
    io::BufReader,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use camino::Utf8Path;
use rodio::{Decoder, OutputStream, Source, mixer::Mixer};

use crate::player::outputs::rodio2::{
    self, ConstSource,
    const_source::{
        adaptor,
        queue::uniform::{UniformQueue, UniformQueueHandle},
    },
};

pub mod outputs;

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

type MpdSourceInner = rodio2::const_source::periodic_access::WithData<
    44100,
    2,
    adaptor::DynamicToConstant<
        44100,
        2,
        rodio::source::Amplify<
            rodio::source::Pausable<rodio::source::Stoppable<Decoder<BufReader<File>>>>,
        >,
    >,
    (Arc<PlayerParams>, AbortHandle),
>;
type MpdSource = rodio2::const_source::periodic_access::PeriodicAccess<44100, 2, MpdSourceInner>;

pub struct Player {
    queue: UniformQueueHandle<44100, 2, MpdSource>,
    params: Arc<PlayerParams>,
    /// Signal the output stream holder thread to stop on drop
    audio_output_abort_handle: mpsc::Sender<()>,
    last_song_abort_handle: Option<AbortHandle>,
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
        let config = rodio::speakers::SpeakersBuilder::new()
            .default_device()
            .unwrap()
            .default_config()
            .unwrap();

        // The rodio Outputstream gets closed when its dropped. Therefore we
        // need to hold it. We want Player to be send but the Outputstream is
        // not. We therefore hold the stream hostage in this thread until Player
        // drops.
        let (tx, rx) = mpsc::channel();
        let (audio_output_abort_handle, abort_rx) = mpsc::channel();
        thread::Builder::new()
            .name("audio-output-stream-holder".to_string())
            .spawn(move || {
                let stream = config.open_stream().unwrap();
                let mixer = stream.mixer().clone();
                let (queue, handle) = UniformQueue::<44100, 2, MpdSource>::new();
                // TODO make the stream mixer accept ConstSource
                mixer.add(queue.adaptor_to_dynamic());
                tx.send(handle);

                let _ = abort_rx.recv();
            })
            .expect("should be able to spawn threads");
        let queue = rx
            .recv()
            .expect("audio-output-stream-holder thread should not panic");

        Self {
            queue,
            audio_output_abort_handle,
            params: Arc::new(PlayerParams {
                volume: AtomicF32::new(volume),
                paused: AtomicBool::new(paused),
            }),
            last_song_abort_handle: None,
        }
    }

    pub async fn add(&mut self, path: &Utf8Path) -> Result<()> {
        const AUDIO_THREAD_RESPONSE_LATENCY: Duration = Duration::from_millis(50);

        let file = BufReader::new(File::open(path)?);
        let params = Arc::clone(&self.params);
        let abort_handle = AbortHandle::new();

        // this drops any previous abort handle.
        // Causing any playing song to stop
        self.last_song_abort_handle = Some(abort_handle.clone());

        let source = Decoder::try_from(file)?
            // TODO move to const source
            .stoppable()
            .pausable(params.paused())
            // TODO move to queue (needs to be implemented on ConstSource first
            .amplify(1.0);
        let const_source = adaptor::DynamicToConstant::<44100, 2, _>::new(source)
            .with_data((params, abort_handle))
            .periodic_access(AUDIO_THREAD_RESPONSE_LATENCY, fun_name);

        // ensure the previous song has been stopped before the new one starts
        tokio::time::sleep(AUDIO_THREAD_RESPONSE_LATENCY).await;
        self.queue.add(const_source);
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

fn fun_name(
    source: &mut rodio2::const_source::periodic_access::WithData<
        44100,
        2,
        adaptor::DynamicToConstant<
            44100,
            2,
            rodio::source::Amplify<
                rodio::source::Pausable<rodio::source::Stoppable<Decoder<BufReader<File>>>>,
            >,
        >,
        (Arc<PlayerParams>, AbortHandle),
    >,
) {
    let (params, abort_handle) = &source.data;

    let amplify = source.inner.inner_mut();
    amplify.set_factor(params.volume());

    let pausable = amplify.inner_mut();
    pausable.set_paused(params.paused());

    let stoppable = pausable.inner_mut();
    if abort_handle.should_abort() {
        stoppable.stop();
    }
}
