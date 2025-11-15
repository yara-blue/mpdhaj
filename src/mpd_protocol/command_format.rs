/// Send from client to server, can only deserialize.

mod de;
mod error;

pub use de::from_str;

#[cfg(test)]
mod tests;
