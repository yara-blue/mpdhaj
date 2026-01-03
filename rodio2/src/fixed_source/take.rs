use std::time::Duration;

use rodio::{FixedSource, Sample};

pub struct TakeDuration<S: FixedSource>(TakeSamples<S>);

impl<S: FixedSource> TakeDuration<S> {
    pub fn inner(&self) -> &S {
        &self.0.inner
    }
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.0.inner
    }
    pub fn into_inner(self) -> S {
        self.0.inner
    }
}

impl<S: FixedSource> TakeDuration<S> {
    pub(crate) fn new(source: S, duration: Duration) -> Self {
        let left = duration.as_secs_f64() * source.sample_rate().get() as f64;
        let left = left.ceil() as u64;
        Self(TakeSamples {
            inner: source,
            left,
        })
    }
}

impl<S: FixedSource> FixedSource for TakeDuration<S> {
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.0.total_duration()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.0.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.0.sample_rate()
    }
}

impl<S: FixedSource> Iterator for TakeDuration<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct TakeSamples<S: FixedSource> {
    pub(crate) inner: S,
    pub(crate) left: u64,
}

crate::add_inner_methods!(TakeSamples<S>);

impl<S: FixedSource> FixedSource for TakeSamples<S> {
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }
}

impl<S: FixedSource> Iterator for TakeSamples<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left > 0 {
            self.left -= 1;
            self.inner.next()
        } else {
            None
        }
    }
}
