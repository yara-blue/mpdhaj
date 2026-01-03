use rodio::math::db_to_linear;
use rodio::{FixedSource, Sample};

use crate::add_inner_methods;

fn normalized_to_linear(normalized: f32) -> f32 {
    const NORMALIZATION_MIN: f32 = 0.0;
    const NORMALIZATION_MAX: f32 = 1.0;
    const LOG_VOLUME_GROWTH_RATE: f32 = 6.907_755_4;
    const LOG_VOLUME_SCALE_FACTOR: f32 = 1000.0;

    let normalized = normalized.clamp(NORMALIZATION_MIN, NORMALIZATION_MAX);

    let mut amplitude = f32::exp(LOG_VOLUME_GROWTH_RATE * normalized) / LOG_VOLUME_SCALE_FACTOR;
    if normalized < 0.1 {
        amplitude *= normalized * 10.0;
    }
    amplitude
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Factor {
    Linear(f32),
    /// Amplifies the sound logarithmically by the given value.
    ///   - 0 dB = linear value of 1.0 (no change)
    ///   - Positive dB values represent amplification (> 1.0)
    ///   - Negative dB values represent attenuation (< 1.0)
    ///   - -60 dB ≈ 0.001 (barely audible)
    ///   - +20 dB = 10.0 (10x amplification)
    ///
    Decibel(f32),
    /// Normalized amplification in `[0.0, 1.0]` range. This method better
    /// matches the perceived loudness of sounds in human hearing and is
    /// recommended to use when you want to change volume in `[0.0, 1.0]` range.
    /// based on article: <https://www.dr-lex.be/info-stuff/volumecontrols.html>
    ///
    /// **note: it clamps values outside this range.**
    Normalized(f32),
}

impl Factor {
    pub fn input_volume() -> Self {
        Self::Linear(1.0)
    }
    pub fn as_linear(&self) -> f32 {
        match self {
            Factor::Linear(v) => *v,
            Factor::Decibel(db) => db_to_linear(*db),
            Factor::Normalized(normalized) => normalized_to_linear(*normalized),
        }
    }
}

pub struct Amplify<S: FixedSource> {
    pub(crate) inner: S,
    pub(crate) factor: f32,
}

add_inner_methods! {Amplify<S>}

impl<S: FixedSource> Amplify<S> {
    pub fn set_factor(&mut self, factor: Factor) {
        self.factor = factor.as_linear();
    }
}

impl<S: FixedSource> FixedSource for Amplify<S> {
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

impl<S: FixedSource> Iterator for Amplify<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|value| value * self.factor)
    }
}
