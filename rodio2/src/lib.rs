//! We didnt have the technology, but I really wanted it. So now we do.
//!
//! (You can view this as a testbed for some of the ideas I've had for rodio)
//! - Yara

// use std::time::Duration;

// use rodio::{ChannelCount, Sample, SampleRate, Source, source::SineWave};
pub use rodio::Source as DynamicSource;
pub use rodio::source as dynamic_source;
pub use rodio::speakers;
pub use rodio::{ChannelCount, SampleRate};
pub use rodio::{Decoder, MixerOsSink, mixer, nz};

pub mod const_source;
pub mod conversions;
pub mod dynamic_source_ext;
pub mod fixed_source;

pub use const_source::ConstSource;
pub use rodio::FixedSource;

macro_rules! add_inner_methods {
    ($name:ty) => {
        impl<S: FixedSource> $name {
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
    };
}

pub(crate) use add_inner_methods;

