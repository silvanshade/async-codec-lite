use super::{Decoder, Encoder};
use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::marker::PhantomData;

pub struct JsonCodec<Enc, Dec>(PhantomData<(Enc, Dec)>);
impl_phantom!(JsonCodec<Enc, Dec>);

impl<Enc, Dec> Decoder for JsonCodec<Enc, Dec>
where
    for<'de> Dec: Deserialize<'de> + 'static,
{
    type Error = Error;
    type Item = Dec;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let de = serde_json::Deserializer::from_slice(&buf);
        let mut iter = de.into_iter::<Dec>();

        let res = match iter.next() {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(ref e)) if e.is_eof() => Ok(None),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        };

        let offset = iter.byte_offset();

        buf.advance(offset);

        res
    }
}

impl<Enc, Dec> Encoder for JsonCodec<Enc, Dec>
where
    Enc: Serialize + 'static,
{
    type Error = Error;
    type Item = Enc;

    fn encode(&mut self, data: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let j = serde_json::to_string(&data)?;

        buf.reserve(j.len());
        buf.put_slice(&j.as_bytes());

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use serde::{Deserialize, Serialize};

    use super::{Decoder, Encoder, JsonCodec};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        pub name: String,
        pub data: u16,
    }

    #[test]
    fn json_codec_encode_decode() {
        let mut codec = JsonCodec::<TestStruct, TestStruct>::new();
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
    fn json_codec_partial_decode() {
        let mut codec = JsonCodec::<TestStruct, TestStruct>::new();
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
