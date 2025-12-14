mod uglyness;
use color_eyre::Result;
use std::{
    fs::File,
    io::BufReader,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use camino::Utf8Path;
use rodio::{Decoder, OutputStream, Source};

struct Player {
    stream: OutputStream,
    pls_stop: Arc<AtomicBool>,
}

impl Player {
    fn new() -> Self {
        let stream = rodio::speakers::SpeakersBuilder::new()
            .default_device()
            .unwrap()
            .default_config()
            .unwrap()
            .open_stream()
            .unwrap();

        Self {
            stream,
            pls_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    fn add(&self, path: &Utf8Path) -> Result<()> {
        let file = BufReader::new(File::open(path)?);
        let pls_stop = self.pls_stop.clone();
        let source = Decoder::try_from(file)?.stoppable().periodic_access(
            Duration::from_millis(50),
            move |source| {
                if pls_stop.load(Ordering::Relaxed) {
                    source.stop();
                }
            },
        );

        self.stream.mixer().add(source);
        Ok(())
        // .play_raw(rodio::Decoder::new(std::fs::File::open(path).unwrap()).unwrap());
    }

    fn pause(&self) {
        // self._stream.???
    }
    fn unpause(&self) {
        // self._stream.???
    }
}
