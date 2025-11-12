use std::time::Duration;

use rodio::nz;

use crate::protocol::{
    AudioParams, Decibel, PlaylistId, PlaylistName, SongId, SongNumber, State, Status,
    response_format,
};

#[test]
fn serialize_status() {
    assert_eq!(
        response_format::to_string(&Status {
            repeat: false,
            random: true,
            single: false,
            consume: true,
            partition: "default".to_string(),
            playlist: PlaylistId(22),
            playlistlength: 0,
            mixrampdb: Decibel(0.0),
            state: State::Stop,
            lastloadedplaylist: None,
            xfade: Duration::from_secs(5),
            song: SongNumber(5),
            songid: SongId(5),
            elapsed: Duration::from_secs(2),
            bitrate: 320_000,
            duration: Duration::from_secs(320),
            audio: AudioParams {
                samplerate: nz!(44100),
                bits: 24,
                channels: nz!(2)
            },
            error: "Failed to open \"usb dac attached to pi\" (alsa); Failed to open ALSA device \"hw:CARD=UD110v2,DEV=1\": No such device".to_string(),
            nextsong: SongNumber(1),
            nextsongid: SongId(1),
        })
        .unwrap(),
        "repeat: 0
random: 1
single: 0
consume: 1
partition: default
playlist: 22
playlistlength: 0
mixrampdb: 0
state: stop
lastloadedplaylist:
xfade: 5
Failed to open \"usb dac attached to pi\" (alsa); Failed to open ALSA device \"hw:CARD=UD110v2,DEV=1\": No such device
OK
 "
    );
}
