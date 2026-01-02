use std::time::Duration;

use rodio::ChannelCount;
use rodio::Sample;
use rodio::SampleRate;

use crate::ConstSource;

pub mod queue;
pub mod take;

pub struct ParameterMismatch<const SR: u32, const CH: u16> {
    got_sample_rate: SampleRate,
    got_channel_count: ChannelCount,
}

pub trait FixedSource: Iterator<Item = Sample> {
    /// May NEVER return something else once its returned a value
    fn channels(&self) -> ChannelCount;
    /// May NEVER return something else once its returned a value
    fn sample_rate(&self) -> SampleRate;
    fn total_duration(&self) -> Option<Duration>;

    fn take_duration(self, duration: Duration) -> take::TakeDuration<Self>
    where
        Self: Sized,
    {
        take::TakeDuration::new(self, duration)
    }

    fn try_into_const_source<const SR: u32, const CH: u16>(
        self,
    ) -> Result<IntoConstSource<SR, CH, Self>, ParameterMismatch<SR, CH>>
    where
        Self: Sized,
    {
        if self.channels().get() != CH || self.sample_rate().get() != SR {
            Err(ParameterMismatch {
                got_sample_rate: self.sample_rate(),
                got_channel_count: self.channels(),
            })
        } else {
            Ok(IntoConstSource(self))
        }
    }
}

pub struct IntoConstSource<const SR: u32, const CH: u16, S: FixedSource>(S);

impl<const SR: u32, const CH: u16, S: FixedSource> ConstSource<SR, CH>
    for IntoConstSource<SR, CH, S>
{
    fn total_duration(&self) -> Option<Duration> {
        self.0.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S: FixedSource> Iterator for IntoConstSource<SR, CH, S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
