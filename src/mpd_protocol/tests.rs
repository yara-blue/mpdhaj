use crate::protocol::{Command, PlaylistName, SubSystem};

#[test]
fn parse_commands() {
    assert_eq!(Command::parse("commands").unwrap(), Command::Commands);
}

#[test]
fn parse_binary_limit() {
    assert_eq!(
        Command::parse("binarylimit 42").unwrap(),
        Command::BinaryLimit(42)
    );
}

#[test]
fn parse_idle_with_args() {
    assert_eq!(
        Command::parse("idle database message mixer").unwrap(),
        Command::Idle(vec![
            SubSystem::Database,
            SubSystem::Message,
            SubSystem::Mixer
        ])
    );
}

#[test]
fn parse_list_playlist_info() {
    assert_eq!(
        Command::parse(r#"listplaylistinfo "foo\"bar""#).unwrap(),
        Command::ListPlaylistInfo(PlaylistName("foo\"bar".to_string()))
    );
}
