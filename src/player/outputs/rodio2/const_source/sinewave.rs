use super::signal_generator::{Function, SignalGenerator};
use crate::player::outputs::rodio2::ConstSource;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SineWave<const SR: u32> {
    inner: SignalGenerator<SR>,
}

impl<const SR: u32> SineWave<SR> {
    /// The frequency of the sine.
    #[inline]
    pub fn new(freq: f32) -> SineWave<SR> {
        SineWave {
            inner: SignalGenerator::new(freq, Function::Sine),
        }
    }
}

impl<const SR: u32> Iterator for SineWave<SR> {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.inner.next()
    }
}

impl<const SR: u32> ConstSource<SR, 1> for SineWave<SR> {
    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
