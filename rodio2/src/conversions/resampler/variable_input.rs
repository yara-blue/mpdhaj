//! This resampler supports a `DynamicSource` as input. That comes with a lot of
//! overhead as it needs to deal with audio changing samplerate or channel count
//! *at any point*.
//!
//! This resampler may need to inject zero padding if spans get shorter then 2 *
//! 2048 frames.
//!
//! Please use a `FixedSource` or `ConstSource`. These can be efficiently
//! resampled.

// The main complication is that we need fixed sized chunks for the resampler.
// We may need to pad zeros to get to that fixed size. We strip those from the
// output. Still this may give glitches.
//
// For details see:
// https://github.com/HEnquist/rubato/issues/116#issuecomment-3707189593
//
// Example
// --------------------------------------------------------------------
// | span A                    | span B                  | span C
// --------------------------------------------------------------------
// |     A chunk       | chunk | <- this chunk is too small
// --------------------------------------------------------------------
// | A                 | A     0000000000| B               |                 |
//  ******************* ~~~~~~~~~~~~~~~~~ ***************** *****************
//  ^                      ^
//  fixed chunk size for   fixed chunk size for resampling
//  resampling A params    A params, need zero padding to get there.
//

// Debuggy notes
// - its not channel order being messed up
// - its not the resampler ratio changing

use std::iter;
use std::sync::atomic::{AtomicBool, Ordering};

use audioadapter_buffers::direct::InterleavedSlice;
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

// Parameters based on camilladsp Balanced profile:
// https://github.com/HEnquist/camilladsp/blob/master/README.md#asyncsinc-asynchronous-resampling-with-anti-aliasing
// Noise floor at -170dB. (Rather overkill..)
fn high_quality_parameters() -> SincInterpolationParameters {
    let window = rubato::WindowFunction::BlackmanHarris2;
    let sinc_len = 128;
    let f_cutoff = calculate_cutoff(sinc_len, window);
    // based on example fixedout_ramp64.rs in the rubato repo
    SincInterpolationParameters {
        sinc_len,
        f_cutoff,
        oversampling_factor: 512,
        interpolation: rubato::SincInterpolationType::Quadratic, // highest quality
        window,
    }
}

// TODO this whole thing is not done yet. Silly spans make it a bit complex
// Need a bit more info: https://github.com/HEnquist/rubato/issues/116
impl<S: Source> VariableInputResampler<S> {
    pub fn new(input: S, target_sample_rate: SampleRate) -> Self {
        let chunk_size_in = 2048;
        let ratio = target_sample_rate.get() as f64 / input.sample_rate().get() as f64;

        let resampler = rubato::Async::new_sinc(
            ratio,
            10.0,
            &high_quality_parameters(),
            chunk_size_in,
            input.channels().get() as usize,
            rubato::FixedAsync::Input,
        )
        .expect(
            "sample rates are non zero, and we are not changing it so there is no resample ratio",
        );

        // TODO redo on channel count change
        let mut output_buffer = Vec::new();
        output_buffer
            .reserve_exact(resampler.output_frames_max() * input.channels().get() as usize);

        let mut input_buffer = Vec::new();
        input_buffer.reserve_exact(resampler.input_frames_max() * input.channels().get() as usize);

        let mut this = Self {
            next_sample: 0,
            output_buffer,
            input_buffer,
            target_sample_rate,
            resampler,
            input,
        };
        this.resample_buffer();

        let output_delay = this.resampler.output_delay();
        let output_delay = output_delay * this.inner_mut().channels().get() as usize;
        let _ = this.by_ref().take(output_delay).count();
        this
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.input
    }

    pub fn inner(&self) -> &S {
        &self.input
    }

    pub fn into_inner(self) -> S {
        self.input
    }

    fn resample_ratio(&self) -> f64 {
        self.target_sample_rate.get() as f64 / self.input.sample_rate().get() as f64
    }

    /// collect samples until rate changes or maximum
    fn collect_span(&mut self) -> Option<(ChannelCount, usize)> {
        let channels = self.input.channels();
        let current_span_len = self.input.current_span_len();

        // TODO do we need a new resampler or is changing the ratio enough?
        // We always keep the resampler "empty".
        let ratio = self.resample_ratio();

        // TODO experiment remove
        static FIRST: AtomicBool = AtomicBool::new(true);
        if FIRST.load(Ordering::Relaxed) {
            FIRST.store(false, Ordering::Relaxed);
            self.resampler
                .set_resample_ratio(ratio, false)
                .expect("Could not change sample ratio");
        }
        let next_size = self.resampler.input_frames_next() * channels.get() as usize;

        let mut padding_samples = 0;
        let padding = iter::repeat(0.0).inspect(|_| padding_samples += 1);

        let mut input = self.input.by_ref().peekable();
        input.peek()?;

        self.input_buffer.clear();
        match current_span_len {
            None => self // parameters will never change (yay)
                .input_buffer
                .extend(input.chain(padding).take(next_size)),
            Some(span) => self
                .input_buffer // padding here is a worste case crutch
                .extend(input.take(span).chain(padding).take(next_size)),
        }

        Some((channels, padding_samples))
    }

    #[cold]
    fn resample_buffer(&mut self) -> Option<()> {
        let (channels, padding) = self.collect_span()?;

        let input_adapter = InterleavedSlice::new(
            &self.input_buffer,
            channels.get() as usize,
            self.input_buffer.len() / channels.get() as usize,
        )
        .expect("we pre allocate enough space");

        self.output_buffer.resize(
            self.resampler.output_frames_next() * channels.get() as usize,
            0.0,
        );
        let mut output_adapter = InterleavedSlice::new_mut(
            &mut self.output_buffer,
            channels.get() as usize,
            self.resampler.output_frames_next(),
        )
        .expect("we pre allocate enough space");

        let (input_frames, output_frames) = self
            .resampler
            .process_into_buffer(&input_adapter, &mut output_adapter, None)
            .expect("Buffers passed in are of the correct sized");

        debug_assert_eq!(
            input_frames,
            self.input_buffer.len() / channels.get() as usize,
            "We should provide exactly the samples needed by the resampler"
        );

        let padding_samples = padding as f64 * self.resampler.resample_ratio();
        let output_len = output_frames * channels.get() as usize;
        let output_len = output_len - padding_samples as usize;

        self.output_buffer.truncate(output_len);

        self.next_sample = 0;
        Some(())
    }
}

impl<S: Source> Source for VariableInputResampler<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.input
            .current_span_len()
            .map(|span| self.resample_ratio() * span as f64)
            .map(|new_span| new_span as usize)
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
    use std::time::Duration;

    use itertools::Itertools;
    use rodio::buffer::SamplesBuffer;
    use rodio::source::{Function, SignalGenerator};
    use rodio::{ChannelCount, SampleRate, Source, nz};
    use spectrum_analyzer::{FrequencyLimit, scaling::divide_by_N_sqrt};

    use super::VariableInputResampler;

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
        let ms_volume = source
            .into_iter()
            .inspect(|s| print!("{s}\n"))
            .chunks(chunk_size as usize);
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
