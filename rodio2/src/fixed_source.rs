use std::time::Duration;

use rodio::ChannelCount;
use rodio::FixedSource;
use rodio::Sample;
use rodio::SampleRate;

use crate::conversions::channelcount::fixed_input::ChannelConverter;
use crate::conversions::resampler::fixed_input::Resampler;

use crate::fixed_source::amplify::Amplify;
use crate::ConstSource;
use crate::fixed_source::pausable::Pausable;
use crate::fixed_source::periodic_access::PeriodicAccess;
use crate::fixed_source::periodic_access::WithData;
use crate::fixed_source::stoppable::Stoppable;

pub mod amplify;
pub mod buffer;
pub mod pausable;
pub mod periodic_access;
pub mod queue;
pub mod stoppable;
pub mod take;

pub trait FixedSourceExt: FixedSource {
    fn take_duration(self, duration: Duration) -> take::TakeDuration<Self>
    where
        Self: Sized,
    {
        take::TakeDuration::new(self, duration)
    }

    fn periodic_access(self, call_every: Duration, arg: fn(&mut Self)) -> PeriodicAccess<Self>
    where
        Self: Sized,
    {
        periodic_access::PeriodicAccess::new(self, call_every, arg)
    }

    fn with_data<D>(self, data: D) -> WithData<Self, D>
    where
        Self: Sized,
    {
        periodic_access::WithData { inner: self, data }
    }

    fn with_sample_rate(self, sample_rate: SampleRate) -> Resampler<Self>
    where
        Self: Sized,
    {
        Resampler::new(self, sample_rate)
    }

    fn with_channel_count(self, channel_count: ChannelCount) -> ChannelConverter<Self>
    where
        Self: Sized,
    {
        ChannelConverter::new(self, channel_count)
    }

    /// Tries to convert from a fixed source to a const one assuming
    /// the parameters already match. If they do not this returns an error.
    ///
    /// If the parameters do not match you can resample using: ``
    fn try_into_const_source<const SR: u32, const CH: u16>(
        self,
    ) -> Result<IntoConstSource<SR, CH, Self>, ParameterMismatch<SR, CH>>
    where
        Self: Sized,
    {
        if self.channels().get() != CH || self.sample_rate().get() != SR {
            Err(ParameterMismatch {
                sample_rate: self.sample_rate(),
                channel_count: self.channels(),
            })
        } else {
            Ok(IntoConstSource(self))
        }
    }

    fn stoppable(self) -> Stoppable<Self>
    where
        Self: Sized,
    {
        Stoppable {
            inner: self,
            stop: false,
        }
    }

    fn pausable(self, paused: bool) -> Pausable<Self>
    where
        Self: Sized,
    {
        Pausable {
            inner: self,
            paused,
        }
    }

    fn amplify(self, amplify: amplify::Factor) -> Amplify<Self>
    where
        Self: Sized,
    {
        Amplify {
            inner: self,
            factor: amplify.as_linear(),
        }
    }
}

impl<S: FixedSource> FixedSourceExt for S {}

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

#[derive(Debug)]
pub struct ParameterMismatch<const SR: u32, const CH: u16> {
    sample_rate: SampleRate,
    channel_count: ChannelCount,
}

impl<const SR: u32, const CH: u16> std::error::Error for ParameterMismatch<SR, CH> {}

impl<const SR: u32, const CH: u16> std::fmt::Display for ParameterMismatch<SR, CH> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.sample_rate.get() == SR && self.channel_count.get() == CH {
            unreachable!("ParameterMismatch error can only occur when params mismatch");
        } else if self.sample_rate.get() == SR && self.channel_count.get() != CH {
            f.write_fmt(format_args!("Fixed source's channel count: {}, does not match target const source's channel count: {}", self.channel_count.get(), CH))
        } else if self.sample_rate.get() != SR && self.channel_count.get() != CH {
            f.write_fmt(format_args!("Fixed source's sample rate and channel count ({}, {}) do not match target const source's sample rate and channel count ({} {})", self.sample_rate.get(), self.channel_count.get(), SR, CH))
        } else {
            f.write_fmt(format_args!("Fixed source's sample rate : {}, does not match target const source's sample rate: {}", self.sample_rate.get(), SR))
        }
    }
}
