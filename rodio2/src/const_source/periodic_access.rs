use std::time::Duration;

use rodio::Sample;

use crate::ConstSource;

pub struct PeriodicAccess<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> {
    inner: S,
    access: fn(&mut S),
    update_period: u32, // in samples
    samples_until_update: u32,
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> PeriodicAccess<SR, CH, S> {
    pub(crate) fn new(source: S, update_period: Duration, access: fn(&mut S)) -> Self {
        let update_period = 1.0 / update_period.as_secs_f64() * SR as f64;
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

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> ConstSource<SR, CH>
    for PeriodicAccess<SR, CH, S>
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>> Iterator for PeriodicAccess<SR, CH, S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_until_update > self.update_period {
            self.do_access(); // separate fn so we can hint this branch is cold
        }

        self.inner.next()
    }
}

pub struct WithData<const SR: u32, const CH: u16, S: ConstSource<SR, CH>, D> {
    pub inner: S,
    pub data: D,
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>, D> ConstSource<SR, CH>
    for WithData<SR, CH, S, D>
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}

impl<const SR: u32, const CH: u16, S: ConstSource<SR, CH>, D> Iterator for WithData<SR, CH, S, D> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
