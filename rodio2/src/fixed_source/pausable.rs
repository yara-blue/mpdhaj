use rodio::{FixedSource, Sample};

pub struct Pausable<S: FixedSource> {
    pub(crate) inner: S,
    // TODO we need to ramp samples up/down when this changes
    // (though current rodio neglects to do this as well...)
    pub(crate) paused: bool,
}

crate::add_inner_methods!(Pausable<S>);

impl<S: FixedSource> Pausable<S> {
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
}

impl<S: FixedSource> FixedSource for Pausable<S> {
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

impl<S: FixedSource> Iterator for Pausable<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.paused {
            Some(0.0)
        } else {
            self.inner.next()
        }
    }
}
