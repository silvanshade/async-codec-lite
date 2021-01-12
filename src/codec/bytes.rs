use super::{Decoder, Encoder};
use bytes::{Bytes, BytesMut};
use std::convert::Infallible;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BytesCodec;

impl Encoder for BytesCodec {
    type Error = Infallible;
    type Item = Bytes;

    fn encode(&mut self, src: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&src);
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Error = Infallible;
    type Item = Bytes;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        Ok(if len > 0 {
            Some(src.split_to(len).freeze())
        } else {
            None
        })
    }
}
