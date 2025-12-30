//! A mixers input could be:
//!     - fixed size, n inputs that need mixing.
//!         - known at compile time or runtime
//!     - grow dynamically
//!     - have identical sources
//!     - have differently typed sources
//!
//! These all come with performance to usability tradeoffs. Lets see if we can come up with
//! something smart to minimize those.
//!
//! Ideas:
//! - compile time fixed size + same type: [Source].mix()
//! - compile time fixed size + different types: (Source).mix()
//! - runtime fixed size + same type: vec![Source].mix()
//! - runtime fixed size + different types: vec![Box<dyn Source>].mix()
//! - grow dynamically same type: Mpsc::Receiver[Source].mix()
//! - grow dynamically different types: Mpsc::Receiver[Box<dyn Source>].mix()
//!
//! Do we need the `.mix()`: yes as we can use the same idea for queue with `.queue()`
//!
//!
//! Impl:
//! Generic wrapper and implement source trait for that? Might need specialization in which case we
//! will just have to spam a ton of wrappers...

use rodio2::ConstSource;
use rodio2::const_source::signal_generator::{Function, SignalGenerator};
use std::hint::black_box;
use std::time::Duration;

fn main() {
    divan::main();
}

const SINGLE_DURATION: Duration = Duration::from_secs(2);
const SINES: &[usize] = &[1, 2, 5, 10, 20, 30];

fn sine() -> impl ConstSource<44100, 2> {
    SignalGenerator::new(400.0, Function::Sine)
        .with_channel_count::<2>()
        .take_duration(Duration::from_secs(10))
}

mod const_source {
    use std::sync::mpsc;

    use super::*;
    use rodio2::ConstSource;
    use rodio2::const_source::mixer::ConstMix;

    fn consume_mixer(queue: impl ConstSource<44100, 2>, num: usize) -> usize {
        queue
            .take_duration(SINGLE_DURATION.mul_f64(num as f64))
            .count()
    }

    #[divan::bench(consts = SINES)]
    fn uniform_array<const N: usize>() {
        let sources: [_; N] = core::array::from_fn(|_| sine());
        let source = sources.mix();
        black_box(consume_mixer(black_box(source), N));
    }

    #[divan::bench] // TODO larger tuples
    fn tuple() {
        let source = (sine(), sine()).mix();
        black_box(consume_mixer(black_box(source), 2));
    }

    #[divan::bench(consts = SINES)]
    fn uniform_vec<const N: usize>() {
        let sources: Vec<_> = (0..N).into_iter().map(|_| sine()).collect();
        let source = sources.mix();
        black_box(consume_mixer(black_box(source), N));
    }

    #[divan::bench(consts = SINES)]
    fn vec<const N: usize>() {
        let sources: Vec<_> = (0..N)
            .into_iter()
            .map(|_| Box::new(sine()) as Box<dyn ConstSource<44100, 2>>)
            .collect();
        let source = sources.mix();
        black_box(consume_mixer(black_box(source), N));
    }

    #[divan::bench(consts = SINES)]
    fn uniform_mpsc<const N: usize>() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..N {
            tx.send(sine()).unwrap();
        }
        let source = rx.mix();
        black_box(consume_mixer(black_box(source), N));
    }

    #[divan::bench(consts = SINES)]
    fn mpsc<const N: usize>() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..N {
            tx.send(Box::new(sine()) as Box<dyn ConstSource<44100, 2>>)
                .unwrap();
        }
        let source = rx.mix();
        black_box(consume_mixer(black_box(source), N));
    }
}
//
// mod fixed_source {
//     use super::*;
//     use rodio::nz;
//     use rodio2::FixedSource;
//     use rodio2::fixed_source::queue::Queue;
//     use rodio2::fixed_source::queue::uniform::UniformQueue;
//
//     fn consume_queue(queue: Queue, num: usize) -> usize {
//         queue
//             .take_duration(SINGLE_DURATION.mul_f64(num as f64))
//             .count()
//     }
//
//     #[divan::bench(args = SINES)]
//     fn normal(num: usize) {
//         let (source, handle) = Queue::new(nz!(2), nz!(44100));
//         for _ in 0..num {
//             handle.add(Box::new(sine().adaptor_to_dynamic())).unwrap();
//         }
//         black_box(consume_queue(black_box(source), num));
//     }
//
//     #[divan::bench(args = SINES)]
//     fn uniform(num: usize) {
//         let (source, handle) = UniformQueue::new(nz!(2), nz!(44100));
//         for _ in 0..num {
//             handle.add(sine().adaptor_to_dynamic()).unwrap();
//         }
//         black_box(consume_uniform_queue(black_box(source), num));
//     }
//
//     fn consume_uniform_queue<S: FixedSource>(queue: UniformQueue<S>, num: usize) -> usize {
//         queue
//             .take_duration(SINGLE_DURATION.mul_f64(num as f64))
//             .count()
//     }
// }
//
// mod dynamic_source {
//     use super::*;
//     use rodio::queue::{SourcesQueueOutput, queue};
//     use rodio2::DynamicSource;
//
//     fn consume_queue(queue: SourcesQueueOutput, num: usize) -> usize {
//         queue
//             .take_duration(SINGLE_DURATION.mul_f64(num as f64))
//             .count()
//     }
//
//     #[divan::bench(args = SINES)]
//     fn normal(num: usize) {
//         let (handle, source) = queue(true);
//         for _ in 0..num {
//             handle.append(sine().adaptor_to_dynamic());
//         }
//         black_box(consume_queue(black_box(source), num));
//     }
// }
//
