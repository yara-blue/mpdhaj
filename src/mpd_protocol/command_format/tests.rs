use crate::mpd_protocol::{
    Command, List, PlaylistName, SubSystem,
    query::{Filter, Query, QueryNode},
};

#[test]
fn parse_commands() {
    assert_eq!(Command::parse("commands").unwrap(), Command::Commands);
}

#[test]
fn parse_binary_limit() {
    color_eyre::install().unwrap();
    assert_eq!(Command::parse("binarylimit 42").unwrap(), Command::BinaryLimit(42));
}

#[test]
fn parse_idle_with_args() {
    assert_eq!(
        Command::parse("idle database message mixer").unwrap(),
        Command::Idle(vec![SubSystem::Database, SubSystem::Message, SubSystem::Mixer])
    );
}

#[test]
fn parse_list_playlist_info() {
    assert_eq!(
        Command::parse(r#"listplaylistinfo "foo\"bar""#).unwrap(),
        Command::ListPlaylistInfo(PlaylistName("foo\"bar".to_string()), None)
    );
}

#[test]
fn parse_list_with_group() {
    assert_eq!(
        Command::parse("list Album group AlbumArtist").unwrap(),
        Command::List(List {
            tag_to_list: crate::mpd_protocol::Tag::Album,
            query: Query::default(),
            group_by: vec![crate::mpd_protocol::Tag::AlbumArtist],
        })
    );
}

#[test]
fn parse_findadd() {
    use crate::mpd_protocol::Tag;
    assert_eq!(Command::parse(
        "findadd \"((Artist == 'ABBA') AND (Album == '') AND (File == 'ABBA/The Singles. The First Fifty Years/34. I Still Have Faith In You.mp3'))\"").unwrap(),
        Command::FindAdd(Query(QueryNode::And(vec![
            QueryNode::Filter(Filter::TagEqual { tag: Tag::Artist, needle: "ABBA".to_string() }),
            QueryNode::Filter(Filter::TagEqual { tag: Tag::Album, needle: "".to_string() }), QueryNode::Filter(Filter::PathEqual("ABBA/The Singles. The First Fifty Years/34. I Still Have Faith In You.mp3'".into()))
        ])), None, None, None)
    )
}
