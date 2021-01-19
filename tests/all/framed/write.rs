use async_codec_lite::Bytes;
use core::iter::Iterator;
use futures_util::io::AsyncWrite;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

struct ZeroBytes {
    pub count: usize,
    pub limit: usize,
}
impl Iterator for ZeroBytes {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.limit {
            None
        } else {
            self.count += 1;
            Some(Bytes::from_static(b"\0"))
        }
    }
}

struct AsyncWriteNull {
    pub num_poll_write: usize,
    pub last_write_size: usize,
}
impl AsyncWrite for AsyncWriteNull {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        self.num_poll_write += 1;
        self.last_write_size = buf.len();
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "lines")]
mod line {
    use async_codec_lite::{Framed, LinesCodec};
    use futures_lite::future::block_on;
    use futures_util::{io::Cursor, sink::SinkExt};

    #[test]
    fn write() {
        let curs = Cursor::new(vec![0u8; 16]);
        let mut framer = Framed::new(curs, LinesCodec {});
        block_on(framer.send("Hello\n".to_owned())).unwrap();
        block_on(framer.send("World\n".to_owned())).unwrap();
        let parts = framer.into_parts();
        assert_eq!(&parts.io.get_ref()[0 .. 12], b"Hello\nWorld\n");
        assert_eq!(parts.io.position(), 12);
    }

    #[cfg(feature = "lines")]
    #[test]
    fn write_to_eof() {
        let mut buf = [0u8; 16];
        let curs = Cursor::new(&mut buf[..]);
        let mut framer = Framed::new(curs, LinesCodec {});
        let _err = block_on(framer.send("This will fill up the buffer\n".to_owned())).unwrap_err();
        let parts = framer.into_parts();
        assert_eq!(parts.io.position(), 16);
        assert_eq!(&parts.io.get_ref()[0 .. 16], b"This will fill u");
    }
}
