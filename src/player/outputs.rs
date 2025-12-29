use std::{thread, time::Duration};

use color_eyre::{Result, eyre::Context};
use gag::Gag;
use itertools::Itertools;
use rodio::{
    Source,
    speakers::{Output, OutputConfig},
};

pub(crate) mod rodio2;
use rodio2::const_source::{CollectConstSource, ConstSource, SineWave};

pub fn print_all() -> Result<()> {
    let (outputs, errors) = outputs()?;

    for (config, output) in outputs {
        if output.is_default() {
            println!("{output} [default output]")
        } else {
            println!("{output}")
        }
    }

    if !errors.is_empty() {
        tracing::error!(
            "Ran into a number of errors:\n\t{}",
            errors.into_iter().map(|e| format!("\t- {e}")).join("\n")
        );
    }

    Ok(())
}

fn major_a_chord() -> impl Source {
    [220.5, 138.5, 164.5]
        .map(|freq| SineWave::<44100>::new(freq))
        .collect_mixed()
        .adaptor_to_dynamic()
}

/// Go through all outputs beeping as you go
pub fn beep() -> Result<()> {
    let (outputs, _errors) = outputs()?;

    for ((config, output), freq) in outputs.into_iter().zip([220., 440.].into_iter().cycle()) {
        let mut stream = rodio::speakers::SpeakersBuilder::new()
            .device(output.clone())?
            .config(config)?
            .open_stream()?;
        stream.log_on_drop(false);
        let mixer = stream.mixer();

        println!("Playing beep on: {}", output);
        mixer.add(major_a_chord().take_duration(Duration::from_secs(4)));
        thread::sleep(Duration::from_secs(4));
    }
    Ok(())
}

fn outputs() -> Result<(Vec<(OutputConfig, Output)>, Vec<color_eyre::Report>)> {
    let outputs = {
        // alsa loves spamming to stderr
        let gag = Gag::stderr().unwrap();
        rodio::speakers::available_outputs()
    }
    .wrap_err("Could not list available inputs")?;

    let (outputs, errors): (Vec<_>, Vec<_>) = outputs
        .into_iter()
        .map(|output| {
            let config = rodio::speakers::SpeakersBuilder::new()
                .device(output.clone())
                .wrap_err("Could not set device")?
                .default_config()
                .wrap_err("Could not get default config")?
                .try_channels(rodio::nz!(2))
                .ok()
                .map(|config| {
                    config // these rates that can be trivially resampled into 44100
                        .prefer_sample_rates([rodio::nz!(44100), rodio::nz!(88200)])
                        .get_config()
                });
            Ok::<_, color_eyre::Report>((config, output))
        })
        .filter_map_ok(|(config, output)| config.map(|config| (config, output)))
        .partition_result();
    Ok((outputs, errors))
}
