use std::{
    path::Path,
    time::{Duration, SystemTime},
};

use rodio::nz;

use crate::mpd_protocol::{
    AudioParams, IdInPlaylist, PlaylistEntry, PlaylistId, PlaylistInfo, PosInPlaylist, SongId,
    SongNumber, PlaybackState, Status, Volume, response_format,
};

#[test]
fn serialize_status() {
    pretty_assertions::assert_eq!(
        response_format::to_string(&Status {
            repeat: false,
            random: true,
            single: false,
            consume: true,
            partition: "default".to_string(),
            volume: Volume::new(50),
            playlist: PlaylistId(22),
            playlistlength: 0,
            state: PlaybackState::Stop,
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
volume: 50
playlist: 22
playlistlength: 0
state: stop
xfade: 5
song: 5
songid: 5
elapsed: 2.000
bitrate: 320000
duration: 320.000
audio: 44100:24:2
error: Failed to open \"usb dac attached to pi\" (alsa); Failed to open ALSA device \"hw:CARD=UD110v2,DEV=1\": No such device
nextsong: 1
nextsongid: 1
"
    );
}

#[test]
fn serialize_playlistinfo() {
    pretty_assertions::assert_eq!(
        response_format::to_string(&PlaylistInfo(vec![
            PlaylistEntry {
                file: Path::new("Lukas Graham/7 Years.mp3").into(),
                last_modified: "2025-06-15T22:08:17Z".parse().unwrap(),
                added: "2025-11-07T15:33:17Z".parse().unwrap(),
                format: AudioParams {
                    samplerate: nz!(44100),
                    bits: 16,
                    channels: nz!(2)
                },
                disc: None,
                date: "2023".to_string(),
                album_artist: "Various Artists".to_string(),
                track: 15,
                label: "Warner Music Group - X5 Music Group".to_string(),
                genre: None,
                album: "do you ever think about dying".to_string(),
                title: "7 Years".to_string(),
                artist: "Lukas Graham".to_string(),
                duration: Duration::from_secs_f64(237.3),
                pos: PosInPlaylist(0),
                id: IdInPlaylist(294),
            },
            PlaylistEntry {
                file: Path::new("Taylor Swift/1989/01 Welcome To New York.mp3").into(),
                last_modified: "2025-06-15T22:06:26Z".parse().unwrap(),
                added: "2025-11-07T15:33:05Z".parse().unwrap(),
                format: AudioParams {
                    samplerate: nz!(44100),
                    bits: 16,
                    channels: nz!(2)
                },
                artist: "Taylor Swift".to_string(),
                album_artist: "Taylor Swift".to_string(),
                title: "Welcome To New York".to_string(),
                album: "1989 (Deluxe)".to_string(),
                track: 19,
                date: "2014".to_string(),
                genre: Some("Country & Folk".to_string()),
                disc: Some(1),
                label: "Taylor Swift".to_string(),
                duration: Duration::from_secs_f64(212.6),
                pos: PosInPlaylist(1),
                id: IdInPlaylist(295),
            },
            PlaylistEntry {
                file: Path::new("Chappell Roan/EPs/Chappell Roan - School Nights (2017) [24B-44.1kHz]/03. Meantime.flac").into(),
                last_modified: "2025-06-15T22:14:00Z".parse().unwrap(),
                added: "2025-11-07T15:36:03Z".parse().unwrap(),
                format: AudioParams {
                    samplerate: nz!(44100),
                    bits: 24,
                    channels: nz!(2)
                },
                album_artist: "Chappell Roan".to_string(),
                label: "Atlantic Records".to_string(),
                artist: "Chappell Roan".to_string(),
                title: "Meantime".to_string(),
                album: "School Nights".to_string(),
                date: "2017-09-22".to_string(),
                genre: "Pop, Rock, Alternatif et Indé".to_string().into(),
                track: 3,
                disc: None,
                duration: Duration::from_secs_f64(183.448),
                pos: PosInPlaylist(2),
                id: IdInPlaylist(296),

            }
        ]))
        .unwrap(),
        "file: Lukas Graham/7 Years.mp3
Last-Modified: 2025-06-15T22:08:17Z
Added: 2025-11-07T15:33:17Z
Format: 44100:16:2
Artist: Lukas Graham
AlbumArtist: Various Artists
Title: 7 Years
Album: do you ever think about dying
Track: 15
Date: 2023
Label: Warner Music Group - X5 Music Group
duration: 237.300
Pos: 0
Id: 294
file: Taylor Swift/1989/01 Welcome To New York.mp3
Last-Modified: 2025-06-15T22:06:26Z
Added: 2025-11-07T15:33:05Z
Format: 44100:16:2
Artist: Taylor Swift
AlbumArtist: Taylor Swift
Title: Welcome To New York
Album: 1989 (Deluxe)
Track: 19
Date: 2014
Genre: Country & Folk
Label: Taylor Swift
Disc: 1
duration: 212.600
Pos: 1
Id: 295
file: Chappell Roan/EPs/Chappell Roan - School Nights (2017) [24B-44.1kHz]/03. Meantime.flac
Last-Modified: 2025-06-15T22:14:00Z
Added: 2025-11-07T15:36:03Z
Format: 44100:24:2
Artist: Chappell Roan
AlbumArtist: Chappell Roan
Title: Meantime
Album: School Nights
Track: 3
Date: 2017-09-22
Genre: Pop, Rock, Alternatif et Indé
Label: Atlantic Records
duration: 183.448
Pos: 2
Id: 296
"
    );
}
