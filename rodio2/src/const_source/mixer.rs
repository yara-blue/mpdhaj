use std::time::Duration;

use itertools::Itertools;
use rodio::Sample;

use super::ConstSource;

mod array;
mod mpsc;
mod tuple;
mod vec;

/// An optimal mixer that mixer `N` identical sources each with samplerate `SR`
/// and channel count `CH`
pub struct UniformArrayMixer<const SR: u32, const CH: u16, const N: usize, S>
where
    S: ConstSource<SR, CH>,
{
    pub(crate) sources: [S; N],
}

impl<const SR: u32, const CH: u16, const N: usize, S> ConstSource<SR, CH>
    for UniformArrayMixer<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.sources
            .iter()
            .map(ConstSource::total_duration)
            .fold_options(Duration::ZERO, |longest, new| longest.max(new))
    }
}

impl<const SR: u32, const CH: u16, const N: usize, S> Iterator for UniformArrayMixer<SR, CH, N, S>
where
    S: ConstSource<SR, CH>,
{
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.sources.iter_mut().filter_map(|s| s.next()).sum1()
    }
}

// ------------------------ ^ Old boring stuff ----------------------------------
// Cool new fanciness:

// - compile time fixed size + same type: [Source].mix()
// - compile time fixed size + different types: (Source).mix()
// - runtime fixed size + same type: vec![Source].mix()
// - runtime fixed size + different types: vec![Box<dyn Source>].mix()
// - grow dynamically same type: Mpsc::Receiver[Source].mix()
// - grow dynamically different types: Mpsc::Receiver[Box<dyn Source>].mix()

// The generic impl for ConstSource must use the SR and CH generics in the trait
// or it wont compile. In the future this need will go away and we can just use the
// Mix trait.
pub trait ConstMix<const SR: u32, const CH: u16>
where
    Self: Sized,
{
    type Mixer;
    fn mix(self) -> Self::Mixer
    where
        Self: Sized;
}

// Same restriction as with ConstMix requires us to have two different Mixers
pub struct ConstMixer<const SR: u32, const CH: u16, T>(T);
