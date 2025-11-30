use std::time::Duration;

use color_eyre::{Result, eyre::Context};
use itertools::Itertools;
use rodio::nz;
use tracing::debug;

use crate::{
    mpd_protocol::{
        self, AudioParams, FindResult, Tag,
        query::{Filter, Query, QueryNode},
    },
    system::Song,
};

// TODO: try translating query to sql WHERE statement(s)
pub(crate) fn handle_find(system: &super::System, query: &Query) -> Result<Vec<FindResult>> {
    let query_root = &query.0;

    let mut stmt = system
        .db
        .prepare("SELECT path, title, artist, album FROM songs")?;
    stmt.query_and_then([], |row| {
        Result::Ok(Song {
            path: row.get::<_, String>(0)?.into(),
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
        })
    })?
    .filter_ok(|song| apply_query(song, query_root))
    .map_ok(|song| FindResult {
        path: song.path,
        last_modified: jiff::Timestamp::constant(0, 0),
        added: jiff::Timestamp::constant(0, 0),
        format: AudioParams {
            samplerate: nz!(42),
            channels: nz!(1),
            bits: 16,
        },
        duration: Duration::from_secs(69),
    })
    .collect::<Result<Vec<_>, _>>()
}

impl Song {
    fn filter(&self, filter: &Filter) -> bool {
        use mpd_protocol::query::Filter as F;
        match filter {
            F::TagEqual { tag, needle } => self.tag_equals(*tag, needle),
            other => {
                debug!("filter: {other:?} not yet supported, return false");
                false
            }
        }
    }
    fn tag_equals(&self, tag: Tag, needle: &str) -> bool {
        match tag {
            Tag::Album => false,
            Tag::AlbumArtist => false,
            Tag::Artist => self.artist == needle,
            _ => todo!(),
        }
    }
}

fn apply_query(song: &Song, node: &QueryNode) -> bool {
    use mpd_protocol::query::QueryNode as Q;
    match node {
        Q::Filter(filter) => song.filter(filter),
        Q::NegatedFilter(filter) => !song.filter(filter),
        Q::And(query_nodes) => query_nodes.iter().all(|node| apply_query(song, node)),
    }
}
