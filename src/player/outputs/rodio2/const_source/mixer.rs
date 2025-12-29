use std::time::Duration;

use itertools::Itertools;
use rodio::Sample;

use crate::player::outputs::rodio2::const_source::list::UniformArrayList;

use super::ConstSource;

/// An optimal mixer that mixer `N` identical sources each with samplerate `SR`
/// and channel count `CH`
pub struct UniformArrayMixer<const SR: u32, const CH: u16, const N: usize, S>
where
    S: ConstSource<SR, CH>,
{
    pub(crate) sources: [S; N],
}

impl<const SR: u32, const CH: u16, const N: usize, S> ConstSource<SR, CH>
    for UniformArrayMixer<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.sources
            .iter()
            .map(ConstSource::total_duration)
            .fold_options(Duration::ZERO, |longest, new| longest.max(new))
    }
}

impl<const SR: u32, const CH: u16, const N: usize, S> Iterator for UniformArrayMixer<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.sources.iter_mut().filter_map(|s| s.next()).sum1()
    }
}
