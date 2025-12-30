use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use crate::ConstSource;

pub struct UniformQueue<const SR: u32, const CH: u16, S>
where
    S: ConstSource<SR, CH>,
{
    current: Option<S>,
    pending: mpsc::Receiver<(S, u32)>,
    // zero means silence is 'playing'
    current_id: Arc<AtomicU32>,
}

impl<const SR: u32, const CH: u16, S> UniformQueue<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    pub fn new() -> (Self, UniformQueueHandle<SR, CH, S>) {
        static QUEUE_ID: AtomicU32 = AtomicU32::new(1);

        let queue_id = QUEUE_ID.fetch_add(1, Ordering::Relaxed);
        assert!(queue_id < u32::MAX, "Can not create 4 billion queues");
        let current_id = Arc::new(AtomicU32::new(0));

        let (tx, rx) = mpsc::channel();

        (
            Self {
                current: None,
                pending: rx,
                current_id: Arc::clone(&current_id),
            },
            UniformQueueHandle {
                queue_id,
                next_id: Arc::new(AtomicU32::new(0)),
                current_id,
                tx,
            },
        )
    }
}

pub struct UniformQueueHandle<const SR: u32, const CH: u16, S>
where
    S: ConstSource<SR, CH>,
{
    queue_id: u32,
    next_id: Arc<AtomicU32>,
    current_id: Arc<AtomicU32>,
    tx: mpsc::Sender<(S, u32)>,
}

pub struct SourceId {
    pub queue_id: u32,
    pub source_id: u32,
}

#[derive(Debug)]
pub struct QueueDropped;

impl<const SR: u32, const CH: u16, S> UniformQueueHandle<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    pub fn add(&self, source: S) -> Result<SourceId, QueueDropped> {
        // wraps on overflow, should be okay as long as there are < 4 million
        // sources in the list.
        let source_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.tx
            .send((source, source_id))
            .map_err(|_| QueueDropped)?;

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

impl<const SR: u32, const CH: u16, S> ConstSource<SR, CH> for UniformQueue<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        None // endless
    }
}

impl<const SR: u32, const CH: u16, S> Iterator for UniformQueue<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
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
