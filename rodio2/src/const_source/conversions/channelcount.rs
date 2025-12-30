use crate::ConstSource;
use rodio::Sample;

pub struct ChannelConvertor<const SR: u32, const CH_IN: u16, const CH_OUT: u16, S> {
    input: S,
    sample_repeat: Option<Sample>,
    next_output_sample_pos: u16,
}

impl<const SR: u32, const CH_IN: u16, const CH_OUT: u16, S: ConstSource<SR, CH_IN>>
    ChannelConvertor<SR, CH_IN, CH_OUT, S>
{
    pub fn new(input: S) -> Self {
        Self {
            input,
            sample_repeat: None,
            next_output_sample_pos: 0,
        }
    }

    pub fn into_inner(self) -> S {
        self.input
    }
}

impl<const SR: u32, const CH_IN: u16, const CH_OUT: u16, S: ConstSource<SR, CH_IN>>
    ConstSource<SR, CH_OUT> for ChannelConvertor<SR, CH_IN, CH_OUT, S>
{
    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }
}

impl<const SR: u32, const CH_IN: u16, const CH_OUT: u16, S: ConstSource<SR, CH_IN>> Iterator
    for ChannelConvertor<SR, CH_IN, CH_OUT, S>
{
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.next_output_sample_pos {
            0 => {
                // save first sample for mono -> stereo conversion
                let value = self.input.next();
                self.sample_repeat = value;
                value
            }
            x if x < CH_IN => {
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

        if self.next_output_sample_pos == CH_OUT {
            self.next_output_sample_pos = 0;

            if CH_IN > CH_OUT {
                for _ in CH_OUT..CH_IN {
                    self.input.next(); // discarding extra input
                }
            }
        }
        result
    }
}
