use camino::Utf8PathBuf;
use color_eyre::{
    Result, Section,
    eyre::{Context, OptionExt},
};
use rodio::{ChannelCount, SampleRate};

use crate::mpd_protocol::{Tag, command_format};

#[derive(Debug, PartialEq, Eq)]
pub enum Filter {
    /// (TAG == 'VALUE'): match a tag value; if there are multiple values of the
    /// given type, at least one must match.
    ///
    /// The special tag `any` checks all tag types.
    ///
    /// AlbumArtist looks for VALUE in AlbumArtist and falls back to Artist tags
    /// if AlbumArtist does not exist. VALUE is what to find. An empty value
    /// string means: match only if the given tag type does not exist at all
    TagEqual { tag: Tag, needle: String },
    /// (TAG != 'VALUE'): mismatch a tag value; if there are multiple values of
    /// the given type, none of them must match.
    ///
    /// The special tag `any` checks all tag types.
    ///
    /// AlbumArtist looks for VALUE in AlbumArtist and falls back to Artist tags
    /// if AlbumArtist does not exist. With an empty value checks for the
    /// existence of the given tag type.
    TagNotEqual { tag: Tag, needle: String },
    /// (TAG contains 'VALUE') checks if the given value is a substring of the tag value.
    TagContains { tag: Tag, needle: String },
    /// (TAG starts_with 'VALUE') checks if the tag value starts with the given value.
    TagStartsWith { tag: Tag, needle: String },
    /// (TAG =~ 'VALUE') and (TAG !~ 'VALUE') use a Perl-compatible regular
    /// expression instead of doing a simple string comparison.
    TagRegex { tag: Tag, regex: String },
    /// (file == 'VALUE'): match the full song URI (relative to the music directory).
    PathEqual(Utf8PathBuf),
    /// (base 'VALUE'): restrict the search to songs in the given directory (relative to the music directory).
    ParentPathEquals(Utf8PathBuf),
    /// (modified-since 'VALUE'): compares the file’s time stamp with the given value (ISO 8601 or UNIX time stamp).
    ModifiedSince { time: jiff::Timestamp },
    /// (added-since 'VALUE'): compares time stamp when the file was added with the given value (ISO 8601 or UNIX time stamp).
    AddedSince { time: jiff::Timestamp },
    /// (AudioFormat == 'SAMPLERATE:BITS:CHANNELS'): compares the audio format with the given value. See Global Audio Format for a detailed explanation.
    /// (AudioFormat =~ 'SAMPLERATE:BITS:CHANNELS'): matches the audio format with the given mask (i.e. one or more attributes may be *).
    AudioFormatEquals {
        sample_rate: Option<SampleRate>,
        bit_depth: Option<u8>,
        channel_count: Option<ChannelCount>,
    },
    /// (prio >= 42): compares the priority of queued songs.
    QueuePriority(usize),
}

// strum needs this
impl Default for Filter {
    fn default() -> Self {
        Self::QueuePriority(0)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryNode {
    Filter(Filter),
    NegatedFilter(Filter),
    And(Vec<QueryNode>),
}

impl Default for QueryNode {
    fn default() -> Self {
        Self::Filter(Filter::default())
    }
}

// TODO should be a tree of operations
/// One or more [`Filters`](Filter) combined or negated.
///
/// Note that each expression must be enclosed in parentheses, e.g. (!(artist ==
/// 'VALUE')) (which is equivalent to (artist != 'VALUE'))
///
/// (EXPRESSION1 AND EXPRESSION2 ...): combine two or more expressions with
/// logical “and”. Note that each expression must be enclosed in parentheses,
/// e.g. ((artist == 'FOO') AND (album == 'BAR'))
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Query(pub QueryNode);

// TODO replace this with a PEG parser
pub fn parse(line: &str) -> Result<Query> {
    let tag_equals = line.trim().trim_matches('"').trim_start_matches("((").trim_end_matches("))");
    let (tag, needle) = tag_equals
        .split_once("==")
        .ok_or_eyre("Parsing any query except tag == thing is not yet supported")?;
    let tag: Tag = command_format::from_str(tag.trim())
        .wrap_err("Could not deserialize tag")
        .with_note(|| format!("tag was: {tag}"))?;

    todo!()
    // Ok(Query(Filter::TagEqual {
    //     tag,
    //     needle: needle.trim().trim_matches('\'').to_string(),
    // }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_equals() {
        assert_eq!(
            parse("((Album == 'todo'))").unwrap(),
            Query(QueryNode::Filter(Filter::TagEqual {
                tag: Tag::Album,
                needle: "todo".to_string()
            }))
        )
    }
}
