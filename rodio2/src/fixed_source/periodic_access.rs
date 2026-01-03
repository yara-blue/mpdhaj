use std::time::Duration;

use rodio::Sample;

use crate::FixedSource;

pub struct PeriodicAccess<S: FixedSource> {
    inner: S,
    access: fn(&mut S),
    update_period: u32, // in samples
    samples_until_update: u32,
}

crate::add_inner_methods!(PeriodicAccess<S>);

impl<S: FixedSource> PeriodicAccess<S> {
    pub(crate) fn new(source: S, update_period: Duration, access: fn(&mut S)) -> Self {
        let update_period = 1.0 / update_period.as_secs_f64() * source.sample_rate().get() as f64;
        Self {
            inner: source,
            access,
            update_period: update_period as u32,
            samples_until_update: 0,
        }
    }

    #[cold]
    fn do_access(&mut self) {
        (self.access)(&mut self.inner);
        self.samples_until_update = self.update_period;
    }
}

impl<S: FixedSource> FixedSource for PeriodicAccess<S> {
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

impl<S: FixedSource> Iterator for PeriodicAccess<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_until_update > self.update_period {
            self.do_access(); // separate fn so we can hint this branch is cold
        }

        self.inner.next()
    }
}

pub struct WithData<S: FixedSource, D> {
    pub inner: S,
    pub data: D,
}

impl<D, S: FixedSource> WithData<S, D> {
    pub fn inner(&self) -> &S {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: FixedSource, D> FixedSource for WithData<S, D> {
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

impl<S: FixedSource, D> Iterator for WithData<S, D> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
