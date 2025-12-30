//! A set of benchmarks to compare the UniformQueue with the normal Queue. The
//! difference between them is whether dynamic dispatch is used. The
//! UniformQueue can be seen as a specialization of Queue for when all sources
//! are the same type. Something non trivial to achieve. Making queue far more
//! easy to use.
//!
//! Is it worth having the Uniform queue?
//! - is there a real performance tradeoff?
//! - at what workloads do we notice this
//!
//! We only test this for the ConstSource. These benchmarks back the
//! documentation for queue and steer the decision whether its worth the time
//! investment to try more "Uniform" sources.

use rodio2::ConstSource;
use rodio2::const_source::signal_generator::{Function, SignalGenerator};
use std::hint::black_box;
use std::time::Duration;

fn main() {
    divan::main();
}

const SINGLE_DURATION: Duration = Duration::from_secs(2);
const SINES: &[usize] = &[1, 5, 10, 20, 30, 40];

fn sine() -> impl ConstSource<44100, 2> {
    SignalGenerator::new(400.0, Function::Sine)
        .with_channel_count::<2>()
        .take_duration(Duration::from_secs(10))
}

mod const_source {
    use super::*;
    use rodio2::ConstSource;
    use rodio2::const_source::queue::Queue;
    use rodio2::const_source::queue::uniform::UniformQueue;

    fn consume_uniform_queue<S: ConstSource<44100, 2>>(
        queue: UniformQueue<44100, 2, S>,
        num: usize,
    ) -> usize {
        queue
            .take_duration(SINGLE_DURATION.mul_f64(num as f64))
            .count()
    }

    #[divan::bench(args = SINES)]
    fn uniform(num: usize) {
        let (source, handle) = UniformQueue::<44100, 2, _>::new();
        for _ in 0..num {
            handle.add(sine()).unwrap();
        }
        black_box(consume_uniform_queue(black_box(source), num));
    }

    fn consume_queue(queue: Queue<44100, 2>, num: usize) -> usize {
        queue
            .take_duration(SINGLE_DURATION.mul_f64(num as f64))
            .count()
    }

    #[divan::bench(args = SINES)]
    fn normal(num: usize) {
        let (source, handle) = Queue::<44100, 2>::new();
        for _ in 0..num {
            handle.add(Box::new(sine())).unwrap();
        }
        black_box(consume_queue(black_box(source), num));
    }
}

mod fixed_source {
    use super::*;
    use rodio::nz;
    use rodio2::FixedSource;
    use rodio2::fixed_source::queue::Queue;
    use rodio2::fixed_source::queue::uniform::UniformQueue;

    fn consume_queue(queue: Queue, num: usize) -> usize {
        queue
            .take_duration(SINGLE_DURATION.mul_f64(num as f64))
            .count()
    }

    #[divan::bench(args = SINES)]
    fn normal(num: usize) {
        let (source, handle) = Queue::new(nz!(2), nz!(44100));
        for _ in 0..num {
            handle.add(Box::new(sine().adaptor_to_dynamic())).unwrap();
        }
        black_box(consume_queue(black_box(source), num));
    }

    #[divan::bench(args = SINES)]
    fn uniform(num: usize) {
        let (source, handle) = UniformQueue::new(nz!(2), nz!(44100));
        for _ in 0..num {
            handle.add(sine().adaptor_to_dynamic()).unwrap();
        }
        black_box(consume_uniform_queue(black_box(source), num));
    }

    fn consume_uniform_queue<S: FixedSource>(queue: UniformQueue<S>, num: usize) -> usize {
        queue
            .take_duration(SINGLE_DURATION.mul_f64(num as f64))
            .count()
    }
}

mod dynamic_source {
    use super::*;
    use rodio::queue::{SourcesQueueOutput, queue};
    use rodio2::DynamicSource;

    fn consume_queue(queue: SourcesQueueOutput, num: usize) -> usize {
        queue
            .take_duration(SINGLE_DURATION.mul_f64(num as f64))
            .count()
    }

    #[divan::bench(args = SINES)]
    fn normal(num: usize) {
        let (handle, source) = queue(true);
        for _ in 0..num {
            handle.append(sine().adaptor_to_dynamic());
        }
        black_box(consume_queue(black_box(source), num));
    }
}
