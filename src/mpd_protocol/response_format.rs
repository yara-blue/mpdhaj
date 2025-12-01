mod error;
/// Responses send from server to client, can only serialize. Note this is a
/// completly different encoding then commands send from client to server.
mod ser;

use std::time::Duration;

use crate::mpd_protocol::{AudioParams, SubSystem};

pub use ser::to_string;

#[cfg(test)]
mod tests;

pub fn duration_seconds<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

/// MPD represents "accurate" durations as a number with three places after the decimal.
/// the mpd format (see [`response_format::ser`]) has been set up to serialize
/// f64 floats with 3 decimals only
pub fn duration_millis_precise<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64(duration.as_secs_f64())
}

pub fn audio_params<S>(
    AudioParams { samplerate, bits, channels }: &AudioParams,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{samplerate}:{bits}:{channels}"))
}

pub fn subsystem(s: SubSystem) -> String {
    let s = ser::to_string(&s).expect("Subsystem should always serialize");
    format!("changed: {s}\nOK\n")
}

pub fn unix_time<S>(ts: &jiff::Timestamp, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_i64(ts.as_second())
}
