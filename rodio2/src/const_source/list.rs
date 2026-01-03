use std::time::Duration;

use itertools::Itertools;

use crate::ConstSource;

// TODO this should work like the mixers
pub struct UniformArrayList<const SR: u32, const CH: u16, const N: usize, S>
where
    S: ConstSource<SR, CH>,
{
    pub(crate) current: usize,
    pub(crate) sources: [S; N],
}

impl<const SR: u32, const CH: u16, const N: usize, S> ConstSource<SR, CH>
    for UniformArrayList<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.sources
            .iter()
            .map(ConstSource::total_duration)
            .fold_options(Duration::ZERO, |sum, new| sum + new)
    }
}

impl<const SR: u32, const CH: u16, const N: usize, S> Iterator for UniformArrayList<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let source = self.sources.get_mut(self.current)?;
            if let Some(sample) = source.next() {
                return Some(sample);
            } else {
                self.current += 1;
            }
        }
    }
}
