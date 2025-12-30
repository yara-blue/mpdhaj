use itertools::Itertools;
use rodio::Sample;

use std::time::Duration;

use super::super::ConstSource;
use super::ConstMix;

/// Need to hold the vec from the user so they can not modify it once we start
/// iterating (since then they could mess up the channel inverleaving by
/// introducing a new source mid frame
pub struct VecMixer<const SR: u32, const CH: u16, S: ConstSource<SR, CH>>(Vec<S>);

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstMix<SR, CH> for Vec<S> {
    type Mixer = VecMixer<SR, CH, S>;
    fn mix(self) -> Self::Mixer
    where
        Self: Sized,
    {
        VecMixer(self)
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstSource<SR, CH>
    for VecMixer<SR, CH, S>
{
    fn total_duration(&self) -> Option<Duration> {
        self.0
            .iter()
            .map(|s| s.total_duration())
            .fold_options(Duration::ZERO, |longest, dur| longest.max(dur))
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> Iterator for VecMixer<SR, CH, S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        // accumulate into f64 to prevent overflow
        let (sum, n_mixed) = self
            .0
            .iter_mut()
            .filter_map(|s| s.next())
            .map(|s| (s as f64, 1))
            .reduce(|(sum, summed), (sample, _)| (sum + sample, summed + 1))?;
        Some((sum / n_mixed as f64) as f32)
    }
}
