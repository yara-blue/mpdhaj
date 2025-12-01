//! Parses mpd commands which are always a single line of text.

use camino::Utf8PathBuf;
use color_eyre::{Section, eyre::Context};
use itertools::Itertools;
use peg::{RuleResult, RuleResult::*};
use std::str::FromStr;

use crate::mpd_protocol::{Command, Position, SubSystem};

peg::parser! {
grammar command() for str {
    pub rule line() -> Command
        = v:command() {v}
    rule command() -> Command
        = query_state() / playback_options() / control_playback() / manipulate_queue() / manipulate_playlist() / interact_with_database() / mounts_and_neighbors() / stickers() / connection_settings() / partitions() / audio_outputs() / client_to_client() / command_without_arguments()

    rule query_state() -> Command
    = "idle" s:list(<subsystem()>) { Command::Idle(s) }

    rule playback_options() -> Command
    = "todo" { todo!() }
    rule control_playback() -> Command
    = "todo" { todo!() }
    rule manipulate_queue() -> Command
    = add()
    rule manipulate_playlist() -> Command
    = "todo" { todo!() }
    rule interact_with_database() -> Command
    = "todo" { todo!() }
    rule mounts_and_neighbors() -> Command
    = "todo" { todo!() }
    rule stickers() -> Command
    = "todo" { todo!() }
    rule connection_settings() -> Command
    = "binarylimit " n:number() { Command::BinaryLimit(n) }
    rule partitions() -> Command
    = "todo" { todo!() }
    rule audio_outputs() -> Command
    = "todo" { todo!() }
    rule client_to_client() -> Command
    = "todo" { todo!() }
    rule command_without_arguments() -> Command
        = c:$(['a'..='z' | 'A'..='Z']+) {? Command::from_str(c).or(Err("invalid command character"))  }


    // manipulate queue
    rule add() -> Command
    = "add" _ uri:uri() _? pos:position()? { Command::Add(uri, pos) }


    // util
    rule list<T>(x: rule<T>) -> Vec<T>
    = v:(x() ** " ") {v}

    rule number<T: std::str::FromStr>() -> T
    = s:$(['0'..='9']+) {? s.parse().or(Err("number")) }

    rule position() -> Position
    =     n:number() { Position::Absolute(n) } /
      "+" n:number::<i32>() { Position::Relative(n + 1 ) } /
      "-" n:number::<i32>() { Position::Relative(-n) }

    rule uri() -> Utf8PathBuf = #{ uri }
    rule _() = quiet!{[' '|'\t']}

    rule subsystem() -> SubSystem
        = #{|input, pos| subsystem(input)}
}
}

fn subsystem(input: &str) -> RuleResult<SubSystem> {
    if let Ok(v) = SubSystem::from_str(input) { Matched(input.len(), v) } else { Failed }
}

fn uri(input: &str, pos: usize) -> RuleResult<Utf8PathBuf> {
    match possibly_quoted_string(&input[pos..]) {
        Matched(consumed, s) => {
            Matched(consumed + pos, Utf8PathBuf::from_str(&s).expect("utf8 string"))
        }
        Failed => Failed,
    }
}

// TODO: make \ escaping work correctly on windows...
fn possibly_quoted_string(input: &str) -> RuleResult<String> {
    if !input.starts_with('"') {
        return if let Some(len) = input.find(' ') {
            Matched(len, input[..len].to_owned())
        } else {
            Matched(input.len(), input.to_owned())
        };
    }
    let mut output = String::new();
    let padded = input.chars();
    for w @ (_, _) in padded.tuple_windows() {
        match w {
            ('\\', c @ ('\\' | '"')) => output.push(c),
            (_, '\\') => {}
            (_, '"') => return Matched(output.len() + 2, output),
            (_, c) => output.push(c),
        }
    }
    // unclosed string
    Failed
}

pub fn parse(s: &str) -> color_eyre::Result<Command> {
    use ariadne::{Label, Report, ReportKind, Source};

    let s = s.trim();
    // println!("[PEG_INPUT_START]\n{s}\n[PEG_TRACE_START]");
    let result = command::line(s);
    // println!("[PEG_TRACE_STOP]");

    match result {
        Ok(c) => Ok(c),
        Err(e) => {
            Report::build(ReportKind::Error, e.location.column - 1..e.location.column - 1)
                .with_message("Could not parse")
                .with_label(
                    Label::new(dbg!(e.location.column - 1)..e.location.column - 1)
                        .with_message(format!("Expected one of {}", e.expected)),
                )
                .finish()
                .print(Source::from(s))
                .unwrap();

            Err(e).wrap_err("Could not parse line").with_note(|| format!("line was: {s}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait ExtendRuleResult<T> {
        fn unwrap(self) -> T;
    }

    impl<T> ExtendRuleResult<T> for RuleResult<T> {
        fn unwrap(self) -> T {
            match self {
                Matched(_, v) => v,
                Failed => panic!(),
            }
        }
    }

    #[test]
    fn test_parse_string() {
        let s = "Non-Album/Necry-Talkie/北上のススメ";
        assert_eq!(s, possibly_quoted_string(s).unwrap());
        let s = r#""Daft Punk/Discovery/02 Aerodynamic.mp3""#;
        assert_eq!(s[1..s.len() - 1], possibly_quoted_string(s).unwrap());
        let s = r#""asdf\"asdf""#;
        assert_eq!("asdf\"asdf", possibly_quoted_string(s).unwrap());
        let s = r#""asdf\\asdf""#;
        assert_eq!("asdf\\asdf", possibly_quoted_string(s).unwrap());
    }
}
