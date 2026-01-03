use std::num::NonZeroU32;
use std::time::Duration;

use rodio::{ChannelCount, FixedSource, SampleRate, Source as DynamicSource};

use super::ConstSource;
use crate::conversions::resampler::variable_input::VariableInputResampler;

pub struct DynamicToConstant<const SR: u32, const CH: u16, S: DynamicSource> {
    inner: VariableInputResampler<S>,
}

impl<const SR: u32, const CH: u16, S: DynamicSource> DynamicToConstant<SR, CH, S> {
    pub fn new(source: S) -> Self {
        Self {
            inner: VariableInputResampler::new(
                source,
                const { NonZeroU32::new(SR).expect("Samplerate must be nonzero") },
            ),
        }
    }

    pub fn inner_mut(&mut self) -> &mut S {
        self.inner.inner_mut()
    }
}

impl<const SR: u32, const CH: u16, S: DynamicSource> ConstSource<SR, CH>
    for DynamicToConstant<SR, CH, S>
{
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S: DynamicSource> Iterator for DynamicToConstant<SR, CH, S> {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub struct DynamicToFixed<S: DynamicSource> {
    inner: VariableInputResampler<S>,
    channels: ChannelCount,
    sample_rate: SampleRate,
}

impl<S: DynamicSource> DynamicToFixed<S> {
    pub fn new(source: S) -> Self {
        Self {
            inner: VariableInputResampler::new(
                source,
                const { NonZeroU32::new(SR).expect("Samplerate must be nonzero") },
            ),
        }
    }

    pub fn inner_mut(&mut self) -> &mut S {
        self.inner.inner_mut()
    }
}

impl<S: DynamicSource> FixedSource for DynamicToFixed<S> {
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }
}

impl<S: DynamicSource> Iterator for DynamicToFixed<S> {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
