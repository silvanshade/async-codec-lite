use ::bytes::BytesMut;

macro_rules! impl_phantom {
    ($t:ident < $($param:ident),+ >) => {
        impl<$($param),+> $t<$($param),+> {
            #[allow(missing_docs)]
            pub const fn new() -> Self {
                Self(PhantomData)
            }
        }
        impl<$($param),+> ::std::clone::Clone for $t<$($param),+> {
            fn clone(&self) -> Self { Self::new() }
        }
        impl<$($param),+> ::std::fmt::Debug for $t<$($param),+> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($t)).finish()
            }
        }
        impl<$($param),+> ::std::default::Default for $t<$($param),+> {
            fn default() -> Self { Self::new() }
        }
        impl<$($param),+> ::std::cmp::PartialEq for $t<$($param),+> {
            fn eq(&self, _other: &Self) -> bool { true }
        }
    }
}

mod bytes;
pub use self::bytes::BytesCodec;

mod length;
pub use self::length::{LengthCodec, OverflowError};

mod limit;
pub use self::limit::{DecoderWithSkipAhead, LimitCodec, LimitError, SkipAheadHandler};

#[cfg(feature = "lines")]
mod lines;
#[cfg(feature = "lines")]
pub use self::lines::LinesCodec;

#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "cbor")]
pub use self::cbor::CborCodec;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use self::json::JsonCodec;

pub trait Decoder {
    type Item;
    type Error: std::error::Error + 'static;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error>;

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}

pub trait Encoder {
    type Item;
    type Error: std::error::Error + 'static;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error>;
}
