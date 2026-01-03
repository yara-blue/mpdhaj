use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use rodio::FixedSource;
use rodio::{ChannelCount, SampleRate};

use super::AddError;

pub struct UniformQueue<S: FixedSource> {
    channels: ChannelCount,
    sample_rate: SampleRate,
    current: Option<S>,
    pending: mpsc::Receiver<(S, u32)>,
    // zero means silence is 'playing'
    current_id: Arc<AtomicU32>,
}

impl<S: FixedSource> UniformQueue<S> {
    pub fn new(channels: ChannelCount, sample_rate: SampleRate) -> (Self, UniformQueueHandle<S>) {
        static QUEUE_ID: AtomicU32 = AtomicU32::new(1);

        let queue_id = QUEUE_ID.fetch_add(1, Ordering::Relaxed);
        assert!(queue_id < u32::MAX, "Can not create 4 billion queues");
        let current_id = Arc::new(AtomicU32::new(0));

        let (tx, rx) = mpsc::channel();

        (
            Self {
                channels,
                sample_rate,
                current: None,
                pending: rx,
                current_id: Arc::clone(&current_id),
            },
            UniformQueueHandle {
                channels,
                sample_rate,
                queue_id,
                next_id: Arc::new(AtomicU32::new(0)),
                current_id,
                tx,
            },
        )
    }
}

pub struct UniformQueueHandle<S: FixedSource> {
    channels: ChannelCount,
    sample_rate: SampleRate,
    queue_id: u32,
    next_id: Arc<AtomicU32>,
    current_id: Arc<AtomicU32>,
    tx: mpsc::Sender<(S, u32)>,
}

pub struct SourceId {
    pub queue_id: u32,
    pub source_id: u32,
}

impl<S: FixedSource> UniformQueueHandle<S> {
    pub fn add(&self, source: S) -> Result<SourceId, AddError> {
        if source.channels() != self.channels {
            return Err(AddError::WrongChannelCount {
                got: source.channels(),
                expected: self.channels,
            });
        }
        if source.sample_rate() != self.sample_rate {
            return Err(AddError::WrongSampleRate {
                got: source.sample_rate(),
                expected: self.sample_rate,
            });
        }

        // wraps on overflow, should be okay as long as there are < 4 million
        // sources in the list.
        let source_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.tx
            .send((source, source_id))
            .map_err(|_| AddError::QueueDropped)?;

        Ok(SourceId {
            queue_id: self.queue_id,
            source_id,
        })
    }

    pub fn current(&self) -> SourceId {
        SourceId {
            queue_id: self.queue_id,
            source_id: self.current_id.load(Ordering::Relaxed),
        }
    }
}

impl<S: FixedSource> FixedSource for UniformQueue<S> {
    fn total_duration(&self) -> Option<std::time::Duration> {
        None // endless
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }
}

impl<S: FixedSource> Iterator for UniformQueue<S> {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(curr) = &mut self.current
                && let Some(sample) = curr.next()
            {
                return Some(sample);
            }

            // No need to end the audio source when the queue handle drops
            // that should be handled with a `Stoppable` wrapper instead.
            let next = self.pending.try_recv().ok();

            if let Some((source, id)) = next {
                self.current = Some(source);
                self.current_id.store(id, Ordering::Relaxed);
            } else {
                return Some(0.0);
            }
        }
    }
}
