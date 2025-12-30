use std::time::Duration;

use rodio::Sample;

use crate::ConstSource;

pub struct TakeDuration<const SR: u32, const CH: u16, S: ConstSource<SR, CH>>(
    TakeSamples<SR, CH, S>,
);

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> TakeDuration<SR, CH, S> {
    pub(crate) fn new(source: S, duration: Duration) -> Self {
        let left = duration.as_secs_f64() * SR as f64;
        let left = left.ceil() as u64;
        Self(TakeSamples {
            inner: source,
            left,
        })
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstSource<SR, CH>
    for TakeDuration<SR, CH, S>
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.0.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> Iterator for TakeDuration<SR, CH, S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct TakeSamples<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> {
    pub(crate) inner: S,
    pub(crate) left: u64,
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstSource<SR, CH>
    for TakeSamples<SR, CH, S>
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> Iterator for TakeSamples<SR, CH, S> {
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
