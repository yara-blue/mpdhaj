//! We didnt have the technology, but I really wanted it. So now we do.
//!
//! (You can view this as a testbed for some of the ideas I've had for rodio)
//! - Yara

// use std::time::Duration;

// use rodio::{ChannelCount, Sample, SampleRate, Source, source::SineWave};
pub use rodio::Source as DynamicSource;
pub use rodio::source as dynamic_source;
pub use rodio::{Decoder, OutputStream, mixer, speakers, nz};
pub use rodio::{ChannelCount, SampleRate};

pub mod const_source;
pub mod fixed_source;
pub mod conversions;

pub use const_source::ConstSource;
pub use fixed_source::FixedSource;
