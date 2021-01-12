use super::{Decoder, Encoder};
use bytes::{Buf, Bytes, BytesMut};
use std::{convert::TryFrom, marker::PhantomData};

pub struct LengthCodec<L>(PhantomData<L>);
impl_phantom!(LengthCodec<L>);

#[derive(Debug, thiserror::Error)]
#[error("length overflow")]
pub struct OverflowError;

impl<L> LengthCodec<L> {
    const HEADER_LEN: usize = std::mem::size_of::<L>();
}

pub trait Length {
    fn encode(x: usize, dst: &mut BytesMut) -> Result<(), OverflowError>;
    fn start_decode(src: &[u8]) -> Result<usize, OverflowError>;
}

macro_rules! impl_length {
    ($($x:ty => $y:expr),+ $(,)?) => {
        $(
        impl Length for $x {
            fn encode(x: usize, dst: &mut BytesMut) -> Result<(), OverflowError> {
                let this = Self::try_from(x).map_err(|_| OverflowError)?;
                dst.extend_from_slice(&Self::to_be_bytes(this));
                Ok(())
            }

            fn start_decode(src: &[u8]) -> Result<usize, OverflowError> {
                let mut len_bytes = [0u8; $y];
                len_bytes.copy_from_slice(&src[..$y]);
                usize::try_from(Self::from_be_bytes(len_bytes)).map_err(|_| OverflowError)
            }
        }
        )+
    }
}

impl_length!(u8 => 1, u16 => 2, u32 => 4, u64 => 8);

impl<L: Length> Encoder for LengthCodec<L> {
    type Error = OverflowError;
    type Item = Bytes;

    fn encode(&mut self, src: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(Self::HEADER_LEN + src.len());
        L::encode(src.len(), dst)?;
        dst.extend_from_slice(&src);
        Ok(())
    }
}

impl<L: Length> Decoder for LengthCodec<L> {
    type Error = OverflowError;
    type Item = Bytes;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(if src.len() < std::mem::size_of::<L>() {
            None
        } else {
            let len = L::start_decode(&src)?;
            if src.len() - Self::HEADER_LEN >= len {
                // Skip the length header we already read.
                src.advance(Self::HEADER_LEN);
                Some(src.split_to(len).freeze())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decode {
        use super::*;

        #[test]
        fn it_returns_bytes_withouth_length_header() {
            use bytes::BufMut;
            let mut codec = LengthCodec::<u64>::new();

            let mut src = BytesMut::with_capacity(5);
            src.put(&[0, 0, 0, 0, 0, 0, 0, 3u8, 1, 2, 3, 4][..]);
            let item = codec.decode(&mut src).unwrap();

            assert!(item == Some(Bytes::from(&[1u8, 2, 3][..])));
        }
    }
}
