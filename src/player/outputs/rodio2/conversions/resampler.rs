use core::iter;
use std::time::Duration;

use audioadapter_buffers::direct::InterleavedSlice;
use audioadapter_buffers::owned::InterleavedOwned;
use rodio::{ChannelCount, Sample, SampleRate, Source};
use rubato::{Resampler, SincInterpolationParameters, calculate_cutoff};

pub struct VariableInputResampler<S> {
    input: S,
    next_sample: usize,
    output_buffer: Vec<Sample>,
    input_buffer: Vec<Sample>,
    target_sample_rate: SampleRate,
    resampler: rubato::Async<Sample>,
}

fn high_quality_parameters() -> SincInterpolationParameters {
    let window = rubato::WindowFunction::Blackman2;
    let sinc_len = 128;
    let f_cutoff = calculate_cutoff(sinc_len, window);
    // based on example fixedout_ramp64.rs in the rubato repo
    SincInterpolationParameters {
        sinc_len,
        f_cutoff,
        oversampling_factor: 2048,
        interpolation: rubato::SincInterpolationType::Cubic, // highest quality
        window,
    }
}

impl<S: Source> VariableInputResampler<S> {
    pub fn new(input: S, target_sample_rate: SampleRate) -> Self {
        let chunk_size_in =
            Duration::from_millis(10).as_secs_f32() * input.sample_rate().get() as f32;
        let chunk_size_in = chunk_size_in.ceil() as usize;
        let chunk_size_in = chunk_size_in.min(2048);
        let ratio = target_sample_rate.get() as f64 / input.sample_rate().get() as f64;

        let resampler = rubato::Async::new_sinc(
            ratio,
            10.0,
            high_quality_parameters(),
            chunk_size_in,
            input.channels().get() as usize,
            rubato::FixedAsync::Output,
        )
        .expect(
            "sample rates are non zero, and we are not changing it so there is no resample ratio",
        );

        let mut this = Self {
            next_sample: 0,
            output_buffer: vec![
                0.0;
                resampler.output_frames_max() * input.channels().get() as usize
            ],
            input_buffer: vec![0.0; resampler.input_frames_max() * input.channels().get() as usize],
            target_sample_rate,
            resampler,
            input,
        };
        this.bootstrap();
        this
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.input
    }

    pub fn into_inner(self) -> S {
        self.input
    }

    /// collect samples until rate changes or maximum
    fn collect_span(&mut self) -> (ChannelCount, SampleRate) {
        let channels = self.input.channels();
        let sample_rate = self.input.sample_rate();

        let input_min = self.resampler.input_frames_next();
        let input_max = self.resampler.input_frames_max().max(4069);
        match self.input.current_span_len() {
            // parameters will never change (yay)
            None => self
                .input_buffer
                .extend(self.input.by_ref().chain(iter::repeat(0.0)).take(input_min)),
            Some(span) => self.input_buffer.extend(
                self.input
                    .by_ref()
                    .take(span.min(input_max))
                    .chain(iter::repeat(0.0))
                    .take(input_min),
            ),
        }
        (channels, sample_rate)
    }

    fn bootstrap(&mut self) -> Option<()> {
        let (channels, sample_rate) = self.collect_span();

        let input = InterleavedSlice::new(
            &self.input_buffer,
            channels.get() as usize,
            self.input_buffer.len() / channels.get() as usize,
        )
        .expect("we pre allocate enough space");

        let mut output = InterleavedSlice::new_mut(
            &mut self.output_buffer,
            channels.get() as usize,
            self.resampler.output_frames_next(),
        )
        .expect("we pre allocate enough space");

        let (input_frames, output_frames) = self.resampler
            .process_into_buffer(&input, &mut output, None).expect("Input and output buffer channels are correct as they have been set by the resampler. The buffer for each channel is the same length. The buffer length is what is requested the resampler.");

        debug_assert_eq!(
            input_frames,
            self.input_buffer.len() / channels.get() as usize
        );
        debug_assert_eq!(
            output_frames,
            self.output_buffer.len() / channels.get() as usize
        );

        // https://github.com/HEnquist/rubato/blob/preview_1.0/examples/fixedout_ramp64.rs
        // extract out using audio adapter thingy

        self.next_sample = 0;
        Some(())
    }

    #[cold]
    fn resample_buffer(&mut self) -> Option<()> {
        let (channels, sample_rate) = self.collect_span();

        let input = InterleavedSlice::new(
            &self.input_buffer,
            channels.get() as usize,
            self.input_buffer.len() / channels.get() as usize,
        )
        .expect("we pre allocate enough space");

        let mut output = InterleavedSlice::new_mut(
            &mut self.output_buffer,
            channels.get() as usize,
            self.resampler.output_frames_next(),
        )
        .expect("we pre allocate enough space");

        let (input_frames, output_frames) = self.resampler
            .process_into_buffer(&input, &mut output, None).expect("Input and output buffer channels are correct as they have been set by the resampler. The buffer for each channel is the same length. The buffer length is what is requested the resampler.");

        debug_assert_eq!(
            input_frames,
            self.input_buffer.len() / channels.get() as usize
        );
        debug_assert_eq!(
            output_frames,
            self.output_buffer.len() / channels.get() as usize
        );

        self.next_sample = 0;

        Some(())
    }
}

impl<S: Source> Source for VariableInputResampler<S> {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.input.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.target_sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }
}

impl<S: Source> VariableInputResampler<S> {
    fn next_sample(&mut self) -> Option<Sample> {
        let res = self.output_buffer.get(self.next_sample);
        self.next_sample += 1;
        res.copied()
    }
}

impl<S: Source> Iterator for VariableInputResampler<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.next_sample() {
            return Some(sample);
        }

        self.resample_buffer()?;
        self.next_sample()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::Duration;

    use itertools::Itertools;
    use rodio::buffer::SamplesBuffer;
    use rodio::source::{Function, SignalGenerator};
    use rodio::{ChannelCount, SampleRate, Source, nz};
    use spectrum_analyzer::{FrequencyLimit, scaling::divide_by_N_sqrt};

    use crate::player::outputs::rodio2::conversions::resampler::VariableInputResampler;

    pub(crate) fn sine(channels: ChannelCount, sample_rate: SampleRate) -> impl Source + Clone {
        let sine = SignalGenerator::new(sample_rate, 400.0, Function::Sine)
            .take(sample_rate.get() as usize)
            .map(|s| core::iter::repeat_n(s, channels.get() as usize))
            .flatten();

        SamplesBuffer::new(channels, sample_rate, sine.collect_vec())
    }

    #[derive(Debug)]
    struct PeakPitch {
        pub median: f32,
        pub error: f32,
    }

    fn assert_non_zero_volume_fuzzy(source: impl Source) {
        let sample_rate = source.sample_rate();
        let chunk_size = sample_rate.get() / 1000;
        let ms_volume = source.into_iter().chunks(chunk_size as usize);
        let ms_volume = ms_volume
            .into_iter()
            .map(|chunk| chunk.into_iter().map(|s| s.abs()).sum::<f32>() / chunk_size as f32);

        for (millis, volume) in ms_volume.enumerate() {
            assert!(
                volume > 0.01,
                "Volume about zero around {:?}",
                Duration::from_millis(millis as u64)
            )
        }
    }

    fn median_peak_pitch(source: impl Source) -> PeakPitch {
        use spectrum_analyzer::{samples_fft_to_spectrum, windows::hann_window};

        let channels = source.channels().get();
        let sample_rate = source.sample_rate().get();
        let nyquist_freq = (sample_rate / 2) as f32;
        let hundred_millis: usize = usize::try_from(sample_rate / 10)
            .unwrap()
            .next_power_of_two();

        // de-interleave (take channel 0)
        let samples: Vec<_> = source.step_by(channels as usize).collect();
        let mut resolution = 0f32;
        let mut peaks = samples
            .chunks_exact(hundred_millis)
            .map(|chunk| {
                let spectrum = samples_fft_to_spectrum(
                    &hann_window(chunk),
                    sample_rate,
                    // only care about the human audible range (sorry bats)
                    // (resamplers can include artifacts outside this range
                    // we do not care about since we wont hear them anyway)
                    FrequencyLimit::Range(20f32, 20_000f32.min(nyquist_freq)),
                    Some(&divide_by_N_sqrt),
                )
                .unwrap();

                resolution = resolution.max(spectrum.frequency_resolution());
                spectrum.max().0
            })
            .collect_vec();

        peaks.sort();
        let median = peaks[peaks.len() / 2].val();
        PeakPitch {
            median,
            error: resolution,
        }
    }

    #[test]
    fn constant_samplerate_preserves_length() {
        let test_signal = sine(nz!(3), nz!(48_000));
        let resampled = VariableInputResampler::new(test_signal.clone(), nz!(16_000));

        let diff_in_length = test_signal
            .total_duration()
            .unwrap()
            .abs_diff(resampled.total_duration().unwrap());
        assert!(diff_in_length.as_secs_f32() < 0.1)
    }

    #[test]
    fn stereo_gets_preserved() {
        use rodio::{
            buffer::SamplesBuffer,
            source::{Function, SignalGenerator},
        };

        let sample_rate = nz!(48_000);
        let sample_rate_resampled = nz!(16_000);
        let frequency_0 = 550f32;
        let frequency_1 = 330f32;

        let channel0 = SignalGenerator::new(sample_rate, frequency_0, Function::Sine)
            .take_duration(Duration::from_secs(1));
        let channel1 = SignalGenerator::new(sample_rate, frequency_1, Function::Sine)
            .take_duration(Duration::from_secs(1));

        let source = channel0.interleave(channel1).collect_vec();
        let source = SamplesBuffer::new(nz!(2), sample_rate, source);
        let resampled =
            VariableInputResampler::new(source.clone(), sample_rate_resampled).collect_vec();

        let (channel0_resampled, channel1_resampled): (Vec<_>, Vec<_>) = resampled
            .chunks_exact(2)
            .map(|s| TryInto::<[_; 2]>::try_into(s).unwrap())
            .map(|[channel0, channel1]| (channel0, channel1))
            .unzip();

        for (resampled, frequency) in [
            (channel0_resampled, frequency_0),
            (channel1_resampled, frequency_1),
        ] {
            let resampled = SamplesBuffer::new(nz!(1), sample_rate_resampled, resampled);
            let peak_pitch = median_peak_pitch(resampled);
            assert!(
                (peak_pitch.median - frequency).abs() < peak_pitch.error,
                "pitch should be {frequency} but was {peak_pitch:?}"
            )
        }
    }

    #[test]
    fn resampler_does_not_add_any_latency() {
        let resampled = VariableInputResampler::new(sine(nz!(1), nz!(48_000)), nz!(16_000));
        assert_non_zero_volume_fuzzy(resampled);
    }

    #[cfg(test)]
    mod constant_samplerate_preserves_pitch {
        use super::*;

        #[test]
        fn one_channel() {
            let test_signal = sine(nz!(1), nz!(48_000));
            let resampled = VariableInputResampler::new(test_signal.clone(), nz!(16_000));

            let peak_pitch_before = median_peak_pitch(test_signal);
            let peak_pitch_after = median_peak_pitch(resampled);

            assert!(
                (peak_pitch_before.median - peak_pitch_after.median).abs()
                    < peak_pitch_before.error.max(peak_pitch_after.error),
                "peak pitch_before: {peak_pitch_before:?}, peak pitch_after: {peak_pitch_after:?}"
            );
        }

        #[test]
        fn two_channel() {
            let test_signal = sine(nz!(2), nz!(48_000));
            let resampled = VariableInputResampler::new(test_signal.clone(), nz!(16_000));

            let peak_pitch_before = median_peak_pitch(test_signal);
            let peak_pitch_after = median_peak_pitch(resampled);
            assert!(
                (peak_pitch_before.median - peak_pitch_after.median).abs()
                    < peak_pitch_before.error.max(peak_pitch_after.error),
                "peak pitch_before: {peak_pitch_before:?}, peak pitch_after: {peak_pitch_after:?}"
            );
        }
    }
}
