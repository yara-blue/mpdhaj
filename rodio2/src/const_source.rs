use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::time::Duration;

use rodio::ChannelCount;
use rodio::FixedSource;
use rodio::Sample;
use rodio::SampleRate;
use rodio::Source as DynamicSource; // will be renamed to this upstream

// pub mod adaptor; replaced with into_fixed_source and into_const_source
pub mod conversions;
pub mod list;
pub mod mixer;
pub mod periodic_access;
pub mod queue;
pub mod take;

pub mod signal_generator;
pub use signal_generator::{SawtoothWave, SineWave, SquareWave, TriangleWave};

use periodic_access::PeriodicAccess;

use crate::const_source::conversions::channelcount::ChannelConvertor;
use crate::const_source::periodic_access::WithData;
use crate::const_source::take::TakeDuration;
use crate::const_source::take::TakeSamples;

pub trait ConstSource<const SR: u32, const CH: u16>: Iterator<Item = Sample> {
    fn sample_rate(&self) -> SampleRate {
        const { NonZeroU32::new(SR).expect("SampleRate must be > 0") }
    }
    fn channels(&self) -> ChannelCount {
        const { NonZeroU16::new(CH).expect("Channel count must be > 0") }
    }

    /// This value is free to change at any time
    fn total_duration(&self) -> Option<Duration>;

    fn into_dynamic_source(self) -> ConstSourceAdaptor<SR, CH, Self>
    where
        Self: Sized,
    {
        ConstSourceAdaptor { inner: self }
    }
    fn into_fixed_source(self) -> ConstSourceAdaptor<SR, CH, Self>
    where
        Self: Sized,
    {
        ConstSourceAdaptor { inner: self }
    }

    fn with_channel_count<const CH_OUT: u16>(self) -> ChannelConvertor<SR, CH, CH_OUT, Self>
    where
        Self: Sized,
    {
        ChannelConvertor::new(self)
    }

    fn take_samples(self, n: u64) -> TakeSamples<SR, CH, Self>
    where
        Self: Sized,
    {
        TakeSamples {
            inner: self,
            left: n,
        }
    }

    fn take_duration(self, duration: Duration) -> TakeDuration<SR, CH, Self>
    where
        Self: Sized,
    {
        TakeDuration::new(self, duration)
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

impl<const SR: u32, const CH: u16, S> FixedSource for ConstSourceAdaptor<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    fn channels(&self) -> ChannelCount {
        const { NonZeroU16::new(CH).expect("channel count must be larger then zero") }
    }

    fn sample_rate(&self) -> SampleRate {
        const { NonZeroU32::new(SR).expect("sample rate must be larger then zero") }
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S> DynamicSource for ConstSourceAdaptor<SR, CH, S>
where
    S: ConstSource<SR, CH>,
{
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        const { NonZeroU16::new(CH).expect("channel count must be larger then zero") }
    }

    fn sample_rate(&self) -> SampleRate {
        const { NonZeroU32::new(SR).expect("sample rate must be larger then zero") }
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

impl<const SR: u32, const CH: u16> ConstSource<SR, CH> for Box<dyn ConstSource<SR, CH>> {
    fn total_duration(&self) -> Option<Duration> {
        self.as_ref().total_duration()
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
