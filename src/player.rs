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
use rodio::{
    Decoder, DynamicSource, FixedSource, const_source, dynamic_source,
    dynamic_source_ext::{ExtendDynamicSource, IntoFixedSource},
    fixed_source::{
        self,
        amplify::{Amplify, Factor},
        pausable::Pausable,
        periodic_access::{PeriodicAccess, WithData},
    },
    mixer, nz, speakers,
};

use rodio::{
    self, ConstSource,
    fixed_source::FixedSourceExt,
    fixed_source::queue::uniform::{UniformQueue, UniformQueueHandle},
};

pub mod outputs;
const AUDIO_THREAD_RESPONSE_LATENCY: Duration = Duration::from_millis(50);

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
    queue: UniformQueueHandle<MpdTrack>,
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
        let params = Arc::new(PlayerParams {
            volume: AtomicF32::new(volume),
            paused: AtomicBool::new(paused),
        });

        let builder = speakers::SpeakersBuilder::new()
            .default_device()
            .unwrap()
            .default_config()
            .unwrap()
            .prefer_channel_counts([nz!(2)])
            .prefer_sample_rates([nz!(44100)]);

        // The rodio Outputstream gets closed when its dropped. Therefore we
        // need to hold it. We want Player to be send but the Outputstream is
        // not. We therefore hold the stream hostage in this thread until Player
        // drops.
        let (tx, rx) = mpsc::channel();
        let (audio_output_abort_handle, abort_rx) = mpsc::channel();
        let params_clone = Arc::clone(&params);
        thread::Builder::new()
            .name("audio-output-stream-holder".to_string())
            .spawn(move || {
                let sink = builder.get_config();
                let (queue, handle) = UniformQueue::<MpdTrack>::new(nz!(2), nz!(44100));
                let queue = queue
                    .pausable(params_clone.paused())
                    .amplify(Factor::input_volume())
                    .with_data(params_clone)
                    .periodic_access(AUDIO_THREAD_RESPONSE_LATENCY, update_params);
                let needs_resample = sink.sample_rate != queue.sample_rate();
                let needs_rechannel = sink.channel_count != queue.channels();

                // TODO move all this into builder::play and friends
                let _sink = match (needs_resample, needs_rechannel) {
                    (true, true) => builder
                        .play(
                            queue
                                .with_channel_count(sink.channel_count)
                                .with_sample_rate(sink.sample_rate),
                        )
                        .unwrap(),
                    (true, false) => builder
                        .play(queue.with_sample_rate(sink.sample_rate))
                        .unwrap(),

                    (false, true) => builder
                        .play(queue.with_channel_count(sink.channel_count))
                        .unwrap(),

                    (false, false) => builder.play(queue).unwrap(),
                };

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
            params,
            last_song_abort_handle: None,
        }
    }

    pub async fn add(&mut self, path: &Utf8Path) -> Result<()> {
        let file = BufReader::new(File::open(path)?);
        let params = Arc::clone(&self.params);
        let abort_handle = AbortHandle::new();

        // this drops any previous abort handle.
        // Causing any playing song to stop
        self.last_song_abort_handle = Some(abort_handle.clone());

        let source = Decoder::try_from(file)?
            .into_fixed_source(nz!(44100), nz!(2))
            .stoppable()
            .with_data(abort_handle)
            .periodic_access(AUDIO_THREAD_RESPONSE_LATENCY, stop_on_abort);

        // ensure the previous song has been stopped before the new one starts
        tokio::time::sleep(AUDIO_THREAD_RESPONSE_LATENCY).await;
        self.queue.add(source);
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

use rodio::fixed_source::stoppable::Stoppable;

type DecodeFile = Decoder<BufReader<File>>;
type MpdTrackInner = WithData<Stoppable<IntoFixedSource<DecodeFile>>, AbortHandle>;
type MpdTrack = PeriodicAccess<MpdTrackInner>;
type MpdQueueInner = WithData<Amplify<Pausable<UniformQueue<MpdTrack>>>, Arc<PlayerParams>>;
type MpdQueue = PeriodicAccess<MpdQueueInner>;

fn stop_on_abort(source: &mut MpdTrackInner) {
    let abort_handle = &source.data;
    if abort_handle.should_abort() {
        let stoppable = source.inner_mut();
        stoppable.stop();
    }
}

fn update_params(source: &mut MpdQueueInner) {
    let params = &source.data;

    let mut amplify = &mut source.inner;
    amplify.set_factor(Factor::Normalized(params.volume()));
    let pausable = amplify.inner_mut();
    pausable.set_paused(params.paused());
}
