use rodio::{FixedSource, Sample};

pub struct Stoppable<S: FixedSource> {
    pub(crate) inner: S,
    pub(crate) stop: bool,
}

crate::add_inner_methods!(Stoppable<S>);

impl<S: FixedSource> Stoppable<S> {
    pub fn stop(&mut self) {
        self.stop = true;
    }
}

impl<S: FixedSource> FixedSource for Stoppable<S> {
    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

impl<S: FixedSource> Iterator for Stoppable<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop {
            None
        } else {
            self.inner.next()
        }
    }
}
