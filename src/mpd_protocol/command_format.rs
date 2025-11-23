/// Send from client to server, can only deserialize.
mod de;
mod error;

pub use de::from_str;
use serde::{Deserialize, de::Visitor};

use crate::mpd_protocol::{List, Tag};

#[cfg(test)]
mod tests;

struct ListVisitor {}

impl<'de> Visitor<'de> for ListVisitor {
    type Value = List;

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut group_by = Vec::new();

        let mut seq = seq;
        let tag_to_list: Tag = seq.next_element().unwrap().unwrap();

        while let Some(next) = seq.next_element::<String>().unwrap() {
            if next == "group" {
                let tag: Tag = seq.next_element().unwrap().unwrap();
                group_by.push(tag);
            } else {
                todo!("parse mpd filter");
            }
        }

        return Ok(List {
            tag_to_list,
            group_by,
        });
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("could not deserialize mpd list argument group")
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ListVisitor {})
    }
}
