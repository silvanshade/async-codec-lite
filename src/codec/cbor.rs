use super::{Decoder, Encoder};
use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct CborCodec<Enc, Dec>(PhantomData<(Enc, Dec)>);
impl_phantom!(CborCodec<Enc, Dec>);

impl<Enc, Dec> Decoder for CborCodec<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
{
    type Error = serde_cbor::Error;
    type Item = Dec;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut de = serde_cbor::Deserializer::from_slice(&buf);

        let res: Result<Dec, _> = serde::de::Deserialize::deserialize(&mut de);

        let res = match res {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.is_eof() => Ok(None),
            Err(e) => Err(e),
        };

        let offset = de.byte_offset();

        buf.advance(offset);

        res
    }
}

impl<Enc, Dec> Encoder for CborCodec<Enc, Dec>
where
    Enc: Serialize + 'static,
{
    type Error = serde_cbor::Error;
    type Item = Enc;

    fn encode(&mut self, data: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let j = serde_cbor::to_vec(&data)?;

        buf.reserve(j.len());
        buf.put_slice(&j);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use serde::{Deserialize, Serialize};

    use super::{CborCodec, Decoder, Encoder};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        pub name: String,
        pub data: u16,
    }

    #[test]
    fn cbor_codec_encode_decode() {
        let mut codec = CborCodec::<TestStruct, TestStruct>::new();
        let mut buff = BytesMut::new();

        let item1 = TestStruct {
            name: "Test name".to_owned(),
            data: 16,
        };
        codec.encode(item1.clone(), &mut buff).unwrap();

        let item2 = codec.decode(&mut buff).unwrap().unwrap();
        assert_eq!(item1, item2);

        assert_eq!(codec.decode(&mut buff).unwrap(), None);

        assert_eq!(buff.len(), 0);
    }

    #[test]
    fn cbor_codec_partial_decode() {
        let mut codec = CborCodec::<TestStruct, TestStruct>::new();
        let mut buff = BytesMut::new();

        let item1 = TestStruct {
            name: "Test name".to_owned(),
            data: 34,
        };
        codec.encode(item1, &mut buff).unwrap();

        let mut start = buff.clone().split_to(4);
        assert_eq!(codec.decode(&mut start).unwrap(), None);

        codec.decode(&mut buff).unwrap().unwrap();

        assert_eq!(buff.len(), 0);
    }
}
