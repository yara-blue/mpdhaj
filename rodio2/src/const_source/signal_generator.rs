// adapted from rodio. Copyright rodio contributors.
// Tests and docs removed for brevity (will be re-added once contributing to upstream)

use std::f32::consts::TAU;
use std::time::Duration;

use crate::ConstSource;

pub type GeneratorFunction = fn(f32) -> f32;

#[derive(Clone, Debug)]
pub enum Function {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

fn sine_signal(phase: f32) -> f32 {
    (TAU * phase).sin()
}

fn triangle_signal(phase: f32) -> f32 {
    4.0f32 * (phase - (phase + 0.5f32).floor()).abs() - 1f32
}

fn square_signal(phase: f32) -> f32 {
    if phase % 1.0f32 < 0.5f32 {
        1.0f32
    } else {
        -1.0f32
    }
}

fn sawtooth_signal(phase: f32) -> f32 {
    2.0f32 * (phase - (phase + 0.5f32).floor())
}

/// An infinite source that produces one of a selection of test waveforms.
#[derive(Clone, Debug)]
pub struct SignalGenerator<const SR: u32> {
    function: GeneratorFunction,
    phase_step: f32,
    phase: f32,
}

impl<const SR: u32> SignalGenerator<SR> {
    pub const fn new(frequency: f32, f: Function) -> Self {
        let function: GeneratorFunction = match f {
            Function::Sine => sine_signal,
            Function::Triangle => triangle_signal,
            Function::Square => square_signal,
            Function::Sawtooth => sawtooth_signal,
        };

        Self::with_function(frequency, function)
    }

    pub const fn with_function(frequency: f32, generator_function: GeneratorFunction) -> Self {
        assert!(frequency > 0.0, "frequency must be greater than zero");
        const { assert!(SR > 0, "Sample rate must be larger then zero") };
        let period = SR as f32 / frequency;
        let phase_step = 1.0f32 / period;

        SignalGenerator {
            function: generator_function,
            phase_step,
            phase: 0.0f32,
        }
    }
}

impl<const SR: u32> Iterator for SignalGenerator<SR> {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        let f = self.function;
        let val = Some(f(self.phase));
        self.phase = (self.phase + self.phase_step).rem_euclid(1.0f32);
        val
    }
}

impl<const SR: u32> ConstSource<SR, 1> for SignalGenerator<SR> {
    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }

    // TODO support try_seek (lets take classic impl will fix it up later)
}

macro_rules! signal_new_type {
    ($name:ident, $function:expr) => {
        #[derive(Clone, Debug)]
        pub struct $name<const SR: u32> {
            inner: SignalGenerator<SR>,
        }

        impl<const SR: u32> $name<SR> {
            /// The frequency of the sine.
            #[inline]
            pub fn new(freq: f32) -> Self {
                Self {
                    inner: SignalGenerator::new(freq, $function),
                }
            }
        }

        impl<const SR: u32> Iterator for $name<SR> {
            type Item = f32;

            #[inline]
            fn next(&mut self) -> Option<f32> {
                self.inner.next()
            }
        }

        impl<const SR: u32> ConstSource<SR, 1> for $name<SR> {
            #[inline]
            fn total_duration(&self) -> Option<Duration> {
                None
            }
        }
    };
}

signal_new_type!(SineWave, Function::Sine);
signal_new_type!(SawtoothWave, Function::Sawtooth);
signal_new_type!(SquareWave, Function::Square);
signal_new_type!(TriangleWave, Function::Triangle);
