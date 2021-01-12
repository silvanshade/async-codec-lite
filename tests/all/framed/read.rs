#[cfg(feature = "lines")]
use async_codec_lite::LinesCodec;
use async_codec_lite::{BytesMut, Decoder, Framed};
use futures_lite::future::block_on;
use futures_util::{io::AsyncRead, stream::StreamExt};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

struct MockBurstySender {
    sent: bool,
}
impl AsyncRead for MockBurstySender {
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        const MESSAGES: &[u8] = b"one\ntwo\n";
        if !self.sent && buf.len() >= MESSAGES.len() {
            self.sent = true;
            buf[0 .. MESSAGES.len()].clone_from_slice(MESSAGES);
            Poll::Ready(Ok(MESSAGES.len()))
        } else {
            Poll::Pending
        }
    }
}

#[cfg(feature = "lines")]
#[test]
fn line_read_multi() {
    let io = MockBurstySender { sent: false };
    let mut framed = Framed::new(io, LinesCodec {});
    let one = block_on(framed.next()).unwrap().unwrap();
    assert_eq!(one, "one\n");
    let two = block_on(framed.next()).unwrap().unwrap();
    assert_eq!(two, "two\n");
}

struct OneByteAtATime<'a> {
    input: &'a [u8],
}
impl AsyncRead for OneByteAtATime<'_> {
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        if self.input.is_empty() {
            Poll::Ready(Ok(0))
        } else {
            buf[0] = self.input[0];
            self.input = &self.input[1 ..];
            Poll::Ready(Ok(1))
        }
    }
}

struct AllTheAs;

impl Decoder for AllTheAs {
    type Error = io::Error;
    type Item = char;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        while !src.is_empty() {
            let buf = src.split_to(1);
            let c = char::from(buf[0]);
            if c == 'a' {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }
}

#[test]
fn read_few_messages() {
    let string: &[u8] = b"aabbbabbbabbbabb";
    let input = OneByteAtATime { input: string };
    let mut framed = Framed::new(input, AllTheAs);
    for _ in 0 .. 5 {
        let item = block_on(framed.next()).unwrap().unwrap();
        assert_eq!(item, 'a');
    }
}
