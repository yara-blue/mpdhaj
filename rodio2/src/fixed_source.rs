use std::time::Duration;

use rodio::ChannelCount;
use rodio::Sample;
use rodio::SampleRate;
use rodio::Source as DynamicSource; // will be renamed to this upstream

pub mod queue;
pub mod take;

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
}

// we need this only because of the silly orphan rule, will go away once upstreamed
struct FixedSourceAdaptor<S: FixedSource> {
    inner: S,
}

impl<S: FixedSource> Iterator for FixedSourceAdaptor<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<S: FixedSource> DynamicSource for FixedSourceAdaptor<S> {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}
