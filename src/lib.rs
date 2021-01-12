#![forbid(unsafe_code)]
#![warn(missing_debug_implementations, rust_2018_idioms)]
#![warn(clippy::all)]

mod codec;
mod error;
mod framed;

pub use self::{
    codec::*,
    framed::{Framed, FramedParts, FramedRead, FramedWrite},
};
pub use bytes::{Bytes, BytesMut};
