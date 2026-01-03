use rodio::{ChannelCount, Sample, SampleRate, FixedSource};

use crate::conversions::channelcount::VariableInputChannelConvertor;
use crate::conversions::resampler::variable_input::VariableInputResampler;
use crate::{DynamicSource};

/// Just here for the experimental phase, since we cant add anything
/// to Source/DynamicSource during it.
pub trait ExtendDynamicSource {
    fn into_fixed_source(
        self,
        sample_rate: SampleRate,
        channel_count: ChannelCount,
    ) -> IntoFixedSource<Self>
    where
        Self: DynamicSource + Sized;
}

pub struct IntoFixedSource<S: DynamicSource>(
    VariableInputResampler<VariableInputChannelConvertor<S>>,
);

impl<S: DynamicSource> FixedSource for IntoFixedSource<S> {
    fn channels(&self) -> ChannelCount {
        self.0.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.0.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.0.total_duration()
    }
}

impl<S: DynamicSource> Iterator for IntoFixedSource<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<S: DynamicSource> ExtendDynamicSource for S {
    fn into_fixed_source(
        self,
        sample_rate: SampleRate,
        channel_count: ChannelCount,
    ) -> IntoFixedSource<Self> {
        let source = VariableInputChannelConvertor::new(self, channel_count);
        let source = VariableInputResampler::new(source, sample_rate);
        IntoFixedSource(source)
    }
}
