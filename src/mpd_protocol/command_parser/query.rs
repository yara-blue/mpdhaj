use peg::RuleResult;

use super::try_from_str;
use crate::mpd_protocol::Tag;
use crate::mpd_protocol::query::{Filter, Query, QueryNode};

pub fn parse(input: &str, pos: usize) -> RuleResult<Query> {
    if let Ok(e) = query::expression(&input[pos..]) {
        RuleResult::Matched(pos, Query(e))
    } else {
        RuleResult::Failed
    }
}

peg::parser! {
grammar query() for str {
    pub rule expression() -> QueryNode
        = "(" node:node() ")" { node }
    rule node() -> QueryNode
        = filter() / not() / and() { todo!() }
    rule not() -> QueryNode
        = "todo" { todo!() }
    rule and() -> QueryNode
        = "todo" { todo!() }

    rule filter() -> QueryNode
        = filter:(tag_equal() / tag_contains() / tag_starts_with() / tag_regex() / file_equal() / base() / modified_since() / added_since() / audioformat_equals() / audioformat_mask() / pio()) { QueryNode::Filter(filter) }
    rule tag_equal() -> Filter
        = tag:tag() "==" "'" needle:value() "'" { Filter::TagEqual { tag, needle} }
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
    rule tag() -> Tag = #{ try_from_str }
    rule value() -> String
    = s:([^'\'']*) { s.into_iter().collect() }
}
}
