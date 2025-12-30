use itertools::Itertools;
use rodio::Sample;

use std::sync::mpsc::Receiver;
use std::time::Duration;

use super::super::ConstSource;
use super::ConstMix;

pub struct ReceiverMixer<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> {
    active: Vec<S>,
    rx: Receiver<S>,
    /// kept between zero and `CH`
    sample_counter: u16,
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstMix<SR, CH> for Receiver<S> {
    type Mixer = ReceiverMixer<SR, CH, S>;
    fn mix(self) -> Self::Mixer
    where
        Self: Sized,
    {
        ReceiverMixer {
            active: Vec::with_capacity(10),
            rx: self,
            sample_counter: 0,
        }
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ReceiverMixer<SR, CH, S> {
    fn update_active(&mut self) {
        if self.sample_counter == 0 {
            // is kept between zero and `CH`
            // TODO perf, benchmark if checking an atomic makes sense.
            // Or only doing this once every few samples
            if let Ok(new) = self.rx.try_recv() {
                // this may allocate, I hate that. We may be able to do that on
                // the sender thread but that requires a dedicated sender struct
                // and that is not as elegant... (a perfectly fine reason)
                self.active.push(new);
            }
        }
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstSource<SR, CH>
    for ReceiverMixer<SR, CH, S>
{
    fn total_duration(&self) -> Option<Duration> {
        self.active
            .iter()
            .map(|s| s.total_duration())
            .fold_options(Duration::ZERO, |longest, dur| longest.max(dur))
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> Iterator for ReceiverMixer<SR, CH, S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.update_active();

        // accumulate into f64 to prevent overflow
        let (sum, n_mixed) = self
            .active
            .iter_mut()
            .filter_map(|s| s.next())
            .map(|s| (s as f64, 1))
            .reduce(|(sum, summed), (sample, _)| (sum + sample, summed + 1))?;

        self.sample_counter += 1;
        self.sample_counter %= CH;
        Some((sum / n_mixed as f64) as f32)
    }
}
