use itertools::Itertools;
use rodio::Sample;

use std::time::Duration;

use super::super::ConstSource;
use super::ConstMix;
use super::ConstMixer;

impl<const SR: u32, const CH: u16, const N: usize, S: ConstSource<SR, CH>> ConstMix<SR, CH>
    for [S; N]
{
    type Mixer = ConstMixer<SR, CH, Self>;
    fn mix(self) -> ConstMixer<SR, CH, Self>
    where
        Self: Sized,
    {
        ConstMixer(self)
    }
}

impl<const SR: u32, const CH: u16, const N: usize, S: ConstSource<SR, CH>> ConstSource<SR, CH>
    for ConstMixer<SR, CH, [S; N]>
{
    fn total_duration(&self) -> Option<Duration> {
        self.0
            .iter()
            .map(|s| s.total_duration())
            .fold_options(Duration::ZERO, |longest, dur| longest.max(dur))
    }
}

impl<const SR: u32, const CH: u16, const N: usize, S: ConstSource<SR, CH>> Iterator
    for ConstMixer<SR, CH, [S; N]>
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if N < 20 {
            // Chance of overflowing the 24 bit mantissa is very small with only
            // 20 inputs. Use f32 as accumulator helps simd codegen.
            // TODO verify with benchmark
            let (sum, n_mixed) = self
                .0
                .iter_mut()
                .filter_map(|s| s.next())
                .map(|s| (s, 1))
                .reduce(|(sum, summed), (sample, _)| (sum + sample, summed + 1))?;
            Some(sum / n_mixed as f32)
        } else {
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
}
