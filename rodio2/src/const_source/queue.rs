use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use crate::ConstSource;

pub mod uniform;

pub struct Queue<const SR: u32, const CH: u16> {
    current: Option<Box<dyn ConstSource<SR, CH>>>,
    pending: mpsc::Receiver<(Box<dyn ConstSource<SR, CH>>, u32)>,
    current_id: Arc<AtomicU32>,
}

impl<const SR: u32, const CH: u16> Queue<SR, CH> {
    pub fn new() -> (Self, QueueHandle<SR, CH>) {
        static QUEUE_ID: AtomicU32 = AtomicU32::new(0);

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
            QueueHandle {
                queue_id,
                next_id: Arc::new(AtomicU32::new(0)),
                current_id,
                tx,
            },
        )
    }
}

pub struct QueueHandle<const SR: u32, const CH: u16> {
    queue_id: u32,
    next_id: Arc<AtomicU32>,
    current_id: Arc<AtomicU32>,
    tx: mpsc::Sender<(Box<dyn ConstSource<SR, CH>>, u32)>,
}

pub struct SourceId {
    pub queue_id: u32,
    pub source_id: u32,
}

#[derive(Debug)]
pub struct QueueDropped;

impl<const SR: u32, const CH: u16> QueueHandle<SR, CH> {
    pub fn add(&self, source: Box<dyn ConstSource<SR, CH>>) -> Result<SourceId, QueueDropped> {
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

impl<const SR: u32, const CH: u16> ConstSource<SR, CH> for Queue<SR, CH> {
    fn total_duration(&self) -> Option<std::time::Duration> {
        None // endless
    }
}

impl<const SR: u32, const CH: u16> Iterator for Queue<SR, CH> {
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
