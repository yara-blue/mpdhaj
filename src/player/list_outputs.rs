fn list_outputs() {
    let outputs = rodio::speakers::available_outputs().unwrap();
    for config in outputs.into_iter().map(|output| {
        Ok::<_, rodio::speakers::builder::Error>(
            rodio::speakers::SpeakersBuilder::new()
                .device(output)?
                .try_channels(rodio::nz!(2))?
                // those rates that can be trivially resampled into 44100
                .prefer_sample_rates([rodio::nz!(44100), rodio::nz!(88200)])
                .get_config(),
        )
    }) {
        let config = config.unwrap(); // handle errors later

        // if config. // TODO move is_default device into output config
        //     println!("{name} [default output]")
        // } else {
        //     println!("{name}")
        // }
    }
}

// use std::{ops::Range, thread, time::Duration};

// use itertools::Itertools;
// use rodio::{
//     ChannelCount, DeviceTrait, SampleRate, Source,
//     cpal::{self, ALL_HOSTS, SupportedStreamConfigRange, host_from_id, traits::HostTrait},
//     source::SineWave,
//     stream::supported_output_configs,
// };

// struct OutputDevice {
//     device: rodio::Device,
//     name: String,
//     is_default: bool,
//     supported_configs: Vec<SupportedStreamConfigRange>,
// }

// #[derive(Debug, PartialEq, Eq)]
// enum Error {
//     HostUnavailable(cpal::HostUnavailable),
//     Devices(cpal::DevicesError),
//     DeviceName(cpal::DeviceNameError),
//     SupportedConfigs(cpal::SupportedStreamConfigsError),
// }

// struct Player {}

// impl Player {
//     fn list_outputs() -> (Vec<OutputDevice>, Vec<Error>) {
//         let (mut outputs, mut errors): (Vec<_>, Vec<_>) = ALL_HOSTS
//             .iter()
//             .map(|id| {
//                 // todo something with id?
//                 let host = host_from_id(*id).map_err(Error::HostUnavailable)?;

//                 let default_output = host.default_output_device().map(|device| device.name());
//                 let outputs = host
//                     .devices()
//                     .map_err(Error::Devices)?
//                     .filter(|device| device.supports_output())
//                     .map(move |device| {
//                         Ok::<_, Error>(OutputDevice {
//                             name: device.name().map_err(Error::DeviceName)?,
//                             is_default: Some(device.name()) == default_output,
//                             supported_configs: device
//                                 .supported_output_configs()
//                                 .map_err(Error::SupportedConfigs)?
//                                 .collect(),
//                             device,
//                         })
//                     });
//                 Ok::<_, Error>(outputs)
//             })
//             .flatten_ok()
//             .flatten()
//             .partition_result();

//         outputs.dedup_by_key(|output| output.name.clone());
//         errors.dedup();

//         (outputs, errors)
//     }
// }

// /// Go through all outputs beeping as you go
// pub fn beep_outputs() {
//     // let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
//     // dbg!(&stream_handle);
//     // let mixer = stream_handle.mixer();
//     // mixer.add(SineWave::new(440.0).take_duration(Duration::from_secs(4)));
//     // thread::sleep(Duration::from_secs(2));

//     let (outputs, _errors) = Player::list_outputs();

//     for (output, freq) in outputs.into_iter().zip([220., 440.].into_iter().cycle()) {
//         dbg!();

//         let suitable_configs = output.supported_configs.iter().filter(|c| {
//             (c.min_sample_rate()..c.max_sample_rate()).contains(&cpal::SampleRate(44100))
//         }).collect();

//         let with_optimal_sample_format = configs.by

//         // let builder = rodio::OutputStreamBuilder::from_device(output.device)
//         //     .unwrap()
//         //     .with_channels(2)
//         // let mut stream = rodio::OutputStreamBuilder::from_device(output.device)
//         //     .unwrap()
//         //     .with_channels(rodio::nz!(1))
//         //     .with_sample_rate(rodio::nz!(44100))
//         //     .open_stream()
//         //     .unwrap();
//         // stream.log_on_drop(false);
//         // let mixer = stream.mixer();

//         println!("Playing beep on: {}", output.name);
//         // mixer.add(SineWave::new(freq).take_duration(Duration::from_secs(4)));
//         // thread::sleep(Duration::from_secs(4));
//         dbg!();
//     }
// }

// pub fn print_outputs() {
//     let (outputs, errors) = Player::list_outputs();

//     if !outputs.is_empty() {
//         println!("Stereo outputs:");
//         for OutputDevice {
//             name,
//             is_default,
//             supported_configs,
//             ..
//         } in outputs
//         {
//             if is_default {
//                 println!("{name} [default output]")
//             } else {
//                 println!("{name}")
//             }

//             for config in supported_configs
//                 .iter()
//                 .filter(|c| c.channels() == 2)
//                 .dedup()
//             {
//                 println!(
//                     "\t{}hz - {}hz",
//                     config.min_sample_rate().0,
//                     config.max_sample_rate().0
//                 );
//             }
//         }
//     } else {
//         println!("No audio outputs found");
//     }

//     if errors.is_empty() {
//         return;
//     }

//     println!("\nErrors:");
//     for error in errors {
//         println!("{error:?}");
//     }
// }
