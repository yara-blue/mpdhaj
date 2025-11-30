/// Send from client to server, can only deserialize.
mod de;
mod error;

pub use de::from_str;
use serde::{Deserialize, de::Visitor};

use crate::mpd_protocol::{
    List, Tag,
    query::{self, Query},
};

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

        Ok(List {
            tag_to_list,
            query: Default::default(),
            group_by,
        })
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Mpd List arguments")
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

impl<'de> Deserialize<'de> for Query {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(QueryVisitor {})
    }
}

struct QueryVisitor {}

impl<'de> Visitor<'de> for QueryVisitor {
    type Value = Query;

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut seq = seq;
        // hackish and ugly I know :(
        let mut query = String::new();
        while let Some(next) = seq.next_element::<String>().unwrap() {
            query.push_str(&next);
        }

        use serde::de::Error;
        query::parse(&query).map_err(A::Error::custom)
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Mpd query")
    }
}
