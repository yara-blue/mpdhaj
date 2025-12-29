use rodio::{ChannelCount, Sample};

use rodio::Source as DynamicSource;

pub struct VariableInputChannelConvertor<S> {
    input: S,
    target: ChannelCount,
    sample_repeat: Option<Sample>,
    next_output_sample_pos: u16,
}

impl<S: DynamicSource> VariableInputChannelConvertor<S> {
    pub fn new(input: S, target: ChannelCount) -> Self {
        Self {
            input,
            target,
            sample_repeat: None,
            next_output_sample_pos: 0,
        }
    }

    pub fn into_inner(self) -> S {
        self.input
    }
}

impl<S: DynamicSource> DynamicSource for VariableInputChannelConvertor<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.target
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }
}

impl<S: DynamicSource> Iterator for VariableInputChannelConvertor<S> {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.next_output_sample_pos {
            0 => {
                // save first sample for mono -> stereo conversion
                let value = self.input.next();
                self.sample_repeat = value;
                value
            }
            x if x < self.input.channels().get() => {
                // make sure we always end on a frame boundary
                let value = self.input.next();
                assert!(value.is_some(), "Sources may not emit half frames");
                value
            }
            1 => self.sample_repeat,
            _ => Some(0.0), // all other added channels are empty
        };

        if result.is_some() {
            self.next_output_sample_pos += 1;
        }

        if self.next_output_sample_pos == self.target.get() {
            self.next_output_sample_pos = 0;

            if self.input.channels() > self.target {
                for _ in self.target.get()..self.input.channels().get() {
                    self.input.next(); // discarding extra input
                }
            }
        }
        result
    }
}
