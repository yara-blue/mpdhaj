use rodio::Sample;
use std::time::Duration;

use super::super::ConstSource;
use super::ConstMix;
use super::ConstMixer;

// TODO generate more with a macro, take care to move accumulator type to f64
// when appropriate. Also benchmark to proof that makes sense.
impl<const SR: u32, const CH: u16, S1: ConstSource<SR, CH>, S2: ConstSource<SR, CH>>
    ConstMix<SR, CH> for (S1, S2)
{
    type Mixer = ConstMixer<SR, CH, Self>;
    fn mix(self) -> ConstMixer<SR, CH, Self>
    where
        Self: Sized,
    {
        ConstMixer(self)
    }
}

impl<const SR: u32, const CH: u16, S1: ConstSource<SR, CH>, S2: ConstSource<SR, CH>>
    ConstSource<SR, CH> for ConstMixer<SR, CH, (S1, S2)>
{
    fn total_duration(&self) -> Option<Duration> {
        self.0
            .0
            .total_duration()
            .and_then(|a| self.0.1.total_duration().map(|b| a.max(b)))
    }
}

impl<const SR: u32, const CH: u16, S1: ConstSource<SR, CH>, S2: ConstSource<SR, CH>> Iterator
    for ConstMixer<SR, CH, (S1, S2)>
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let (sum, counted) = [self.0.0.next(), self.0.1.next()]
            .into_iter()
            .flatten()
            .map(|s| (s, 1))
            .reduce(|(sum, summed), (sample, _)| (sum + sample, summed + 1))?;
        Some(sum / counted as f32)
    }
}
