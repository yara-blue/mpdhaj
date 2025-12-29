use std::time::Duration;

use rodio::ChannelCount;
use rodio::Sample;
use rodio::SampleRate;
use rodio::Source as DynamicSource; // will be renamed to this upstream

pub mod adaptor;
pub mod list;
pub mod mixer;
pub mod periodic_access;
pub mod queue;

pub mod signal_generator;
pub use signal_generator::{SawtoothWave, SineWave, SquareWave, TriangleWave};

use periodic_access::PeriodicAccess;

use crate::const_source::periodic_access::WithData;

pub trait ConstSource<const SR: u32, const CH: u16>: Iterator<Item = Sample> {
    /// This value is free to change at any time
    fn total_duration(&self) -> Option<Duration>;

    fn adaptor_to_dynamic(self) -> ConstSourceAdaptor<SR, CH, Self>
    where
        Self: Sized,
    {
        ConstSourceAdaptor { inner: self }
    }

    fn periodic_access(
        self,
        call_every: Duration,
        arg: fn(&mut Self),
    ) -> PeriodicAccess<SR, CH, Self>
    where
        Self: Sized,
    {
        periodic_access::PeriodicAccess::new(self, call_every, arg)
    }

    fn with_data<D>(self, data: D) -> WithData<SR, CH, Self, D>
    where
        Self: Sized,
    {
        periodic_access::WithData { inner: self, data }
    }
}

// we still need this. More fancy const generics will save us at some point :)
pub struct ConstSourceAdaptor<const SR: u32, const CH: u16, S>
where
    S: ConstSource<SR, CH>,
{
    inner: S,
}

impl<const SR: u32, const CH: u16, S> Iterator for ConstSourceAdaptor<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

// TODO rename Source to DynamicSource
impl<const SR: u32, const CH: u16, S> DynamicSource for ConstSourceAdaptor<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        const {
            assert!(CH != 0, "Channel count for ConstantSource may not be zero");
        }
        // checked at compile time above
        unsafe { ChannelCount::new_unchecked(CH as u16) }
    }

    fn sample_rate(&self) -> SampleRate {
        const {
            assert!(SR != 0, "SampleRate for ConstantSource may not be zero");
        }
        // checked at compile time above
        unsafe { SampleRate::new_unchecked(SR as u32) }
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

pub trait CollectConstSource<const SR: u32, const CH: u16, const N: usize, S>
where
    S: ConstSource<SR, CH>,
{
    fn collect_mixed(self) -> mixer::UniformArrayMixer<SR, CH, N, S>;
    fn collect_list(self) -> list::UniformArrayList<SR, CH, N, S>;
}

impl<const SR: u32, const CH: u16, const N: usize, S> CollectConstSource<SR, CH, N, S> for [S; N]
where
    S: ConstSource<SR, CH>,
{
    fn collect_mixed(self) -> mixer::UniformArrayMixer<SR, CH, N, S> {
        mixer::UniformArrayMixer { sources: self }
    }
    fn collect_list(self) -> list::UniformArrayList<SR, CH, N, S> {
        list::UniformArrayList {
            sources: self,
            current: 0,
        }
    }
}
