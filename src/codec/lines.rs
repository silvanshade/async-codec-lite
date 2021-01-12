use super::{Decoder, Encoder};
use bytes::{BufMut, BytesMut};
use memchr::memchr;
use std::convert::Infallible;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinesCodec;

impl Encoder for LinesCodec {
    type Error = Infallible;
    type Item = String;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len());
        dst.put(item.as_bytes());
        Ok(())
    }
}

impl Decoder for LinesCodec {
    type Error = std::string::FromUtf8Error;
    type Item = String;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match memchr(b'\n', src) {
            Some(pos) => {
                let buf = src.split_to(pos + 1);
                String::from_utf8(buf.to_vec()).map(Some)
            },
            _ => Ok(None),
        }
    }
}
