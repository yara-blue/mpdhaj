use peg::RuleResult;

use super::string;
use super::try_from_str;
use crate::mpd_protocol::Tag;
use crate::mpd_protocol::query::{Filter, Query, QueryNode};

// TODO pretty sure we can inline this. But it might be nice for tests
pub fn parse(input: &str, pos: usize) -> RuleResult<Query> {
    dbg!(&input[pos..]);
    if let Ok((e, consumed)) = query::expression(&input[pos..]) {
        RuleResult::Matched(pos + consumed, Query(e))
    } else {
        RuleResult::Failed
    }
}

peg::parser! {
grammar query() for str {
    pub rule expression() -> (QueryNode, usize)
        = "\""? "(" node:node() ")" "\""? consumed:position!() { (node, consumed) }
    rule node() -> QueryNode
        = n:(filter() / not() / and() / nested()) { n }
    rule nested() -> QueryNode
        = "(" n:node() ")" { n }
    rule not() -> QueryNode
        = "todo" { todo!() }
    rule and() -> QueryNode
        = "todo" { todo!() }

    rule filter() -> QueryNode
        = filter:(tag_equal() / tag_contains() / tag_starts_with() / tag_regex() / file_equal() / base() / modified_since() / added_since() / audioformat_equals() / audioformat_mask() / pio()) { QueryNode::Filter(filter) }
    rule tag_equal() -> Filter
        = tag:tag() _ "==" _ needle:value() { Filter::TagEqual { tag, needle} }
    rule tag_contains() -> Filter
        = "todo" { todo!() }
    rule tag_starts_with() -> Filter
        = "todo" { todo!() }
    rule tag_regex() -> Filter
        = "todo" { todo!() }
    rule file_equal() -> Filter
        = "todo" { todo!() }
    rule base() -> Filter
        = "todo" { todo!() }
    rule modified_since() -> Filter
        = "todo" { todo!() }
    rule added_since() -> Filter
        = "todo" { todo!() }
    rule audioformat_equals() -> Filter
        = "todo" { todo!() }
    rule audioformat_mask() -> Filter
        = "todo" { todo!() }
    rule pio() -> Filter
        = "todo" { todo!() }


    // UTIL
    rule _() = quiet!{[' '|'\t']}
    rule tag() -> Tag = #{ try_from_str }
    rule value() -> String = #{ value }
}
}

fn value(input: &str, pos: usize) -> RuleResult<String> {
    use RuleResult::{Failed, Matched};

    match super::possibly_quoted_string(&input[pos..], ")") {
        Matched(consumed, s) => Matched(consumed + pos, s),
        Failed => Failed,
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::Section;
    use color_eyre::eyre::Context;

    use super::*;

    pub fn parse(s: &str) -> color_eyre::Result<QueryNode> {
        use ariadne::{Label, Report, ReportKind, Source};

        let s = s.trim();
        // println!("[PEG_INPUT_START]\n{s}\n[PEG_TRACE_START]");
        let result = query::expression(s);
        // println!("[PEG_TRACE_STOP]");

        match result {
            Ok((c, _)) => Ok(c),
            Err(e) => {
                Report::build(
                    ReportKind::Error,
                    e.location.column - 1..e.location.column - 1,
                )
                .with_message("Could not parse")
                .with_label(
                    Label::new(e.location.column - 1..e.location.column - 1)
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

    #[test]
    fn tag_equals() {
        assert_eq!(
            parse("((Album == '12 Memories'))").unwrap(),
            QueryNode::Filter(Filter::TagEqual {
                tag: Tag::Album,
                needle: "12 Memories".to_string()
            })
        );

        assert_eq!(
            parse("((Artist == Abba))").unwrap(),
            QueryNode::Filter(Filter::TagEqual {
                tag: Tag::Artist,
                needle: "Abba".to_string()
            })
        );
    }
}
