use itertools::Itertools;
use rodio::Sample;

use super::ConstSource;

pub struct ArrayMixer<const SR: u32, const CH: u16, const N: usize, S>
where
    S: ConstSource<SR, CH>,
{
    sources: [S; N],
}

impl<const SR: u32, const CH: u16, const N: usize, S> ConstSource<SR, CH>
    for ArrayMixer<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl<const SR: u32, const CH: u16, const N: usize, S> Iterator for ArrayMixer<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.sources.iter_mut().filter_map(|s| s.next()).sum1()
    }
}

pub trait CollectConstSource<const SR: u32, const CH: u16, const N: usize, S>
where
    S: ConstSource<SR, CH>,
{
    fn collect_mixed(self) -> ArrayMixer<SR, CH, N, S>;
}

impl<const SR: u32, const CH: u16, const N: usize, S> CollectConstSource<SR, CH, N, S> for [S; N]
where
    S: ConstSource<SR, CH>,
{
    fn collect_mixed(self) -> ArrayMixer<SR, CH, N, S> {
        ArrayMixer { sources: self }
    }
}
