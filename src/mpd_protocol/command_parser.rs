//! Parses mpd commands which are always a single line of text.

use color_eyre::{Section, eyre::Context};
use peg::RuleResult;
use std::str::FromStr;

use crate::mpd_protocol::{Command, SubSystem};

peg::parser! {
    grammar command() for str {
        pub rule line() -> Command
          = v:command() {v}
        // rule subsystem() -> SubSystem
        //   = "test" { SubSystem::Database }
        rule subsystem() -> SubSystem
            = #{|input, pos| subsystem(input)}
        rule command() -> Command
          = query_state() / playback_options() / control_playback() / manipulate_queue() / manipulate_playlist() / interact_with_database() / mounts_and_neighbors() / stickers() / connection_settings() / partitions() / audio_outputs() / client_to_client() / command_without_arguments()

        rule query_state() -> Command
        = "idle" s:list(<subsystem()>) { Command::Idle(s) }

        rule playback_options() -> Command
        = "todo" { todo!() }
        rule control_playback() -> Command
        = "todo" { todo!() }
        rule manipulate_queue() -> Command
        = "todo" { todo!() }
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
        = #{|input, pos| command_without_args(input) }

        rule list<T>(x: rule<T>) -> Vec<T>
        = v:(x() ** " ") {v}
        rule number<T: std::str::FromStr>() -> T
        = s:$(['0'..='9']+) {? s.parse().or(Err("number")) }
    }
}

fn subsystem(input: &str) -> RuleResult<SubSystem> {
    if let Ok(v) = SubSystem::from_str(input) {
        RuleResult::Matched(input.len(), v)
    } else {
        RuleResult::Failed
    }
}

fn command_without_args(input: &str) -> RuleResult<Command> {
    if let Ok(v) = Command::from_str(input) {
        RuleResult::Matched(input.len(), v)
    } else {
        RuleResult::Failed
    }
}

pub fn parse(s: &str) -> color_eyre::Result<Command> {
    use ariadne::{Label, Report, ReportKind, Source};

    let s = s.trim();
    let result = command::line(dbg!(s));

    match result {
        Ok(c) => Ok(c),
        Err(e) => {
            Report::build(
                ReportKind::Error,
                e.location.column - 1..e.location.column - 1,
            )
            .with_message("Could not parse")
            .with_label(
                Label::new(dbg!(e.location.column - 1)..e.location.column - 1)
                    .with_message(format!("Expected one of {}", e.expected)),
            )
            .finish()
            .print(Source::from(s))
            .unwrap();

            Err(e)
                .wrap_err("Could not parse line")
                .with_note(|| format!("line was: {s}"))
        }
    }
}
