use crate::{
    codec::{Decoder, Encoder},
    error::Error,
};
use bytes::Buf;
pub use bytes::{Bytes, BytesMut};
use futures_core::{ready, Stream};
use futures_io::{AsyncRead, AsyncWrite};
use futures_sink::Sink;
use pin_project_lite::pin_project;
use std::{
    borrow::{Borrow, BorrowMut},
    io,
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};

const INITIAL_CAPACITY: usize = 8 * 1024;
const BACKPRESSURE_BOUNDARY: usize = INITIAL_CAPACITY;

pin_project! {
    #[derive(Debug)]
    pub(super) struct FramedInner<T, U, State> {
        #[pin]
        pub(super) inner: T,
        pub(super) state: State,
        pub(super) codec: U,
    }
}

impl<T, U, State> Deref for FramedInner<T, U, State> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T, U, R> Stream for FramedInner<T, U, R>
where
    T: AsyncRead,
    U: Decoder,
    R: BorrowMut<ReadFrame>,
{
    type Item = Result<U::Item, Error<U::Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = [0u8; INITIAL_CAPACITY];
        let mut ended = false;
        let mut this = self.project();
        let state = this.state.borrow_mut();
        if state.has_errored {
            return Poll::Ready(None);
        }
        loop {
            match this.codec.decode(&mut state.buffer).map_err(Error::Codec) {
                Ok(ok) => match ok {
                    Some(item) => return Poll::Ready(Some(Ok(item))),
                    None if ended => {
                        return if state.buffer.is_empty() {
                            Poll::Ready(None)
                        } else {
                            match this.codec.decode_eof(&mut state.buffer).map_err(Error::Codec)? {
                                Some(item) => Poll::Ready(Some(Ok(item))),
                                None if state.buffer.is_empty() => Poll::Ready(None),
                                None => Poll::Ready(Some(Err(io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "bytes remaining in stream",
                                )
                                .into()))),
                            }
                        };
                    },
                    _ => {
                        let n = ready!(this.inner.as_mut().poll_read(cx, &mut buf))?;
                        state.buffer.extend_from_slice(&buf[..n]);
                        ended = n == 0;
                        continue;
                    },
                },
                Err(err) => {
                    state.has_errored = true;
                    return Poll::Ready(Some(Err(err)));
                },
            }
        }
    }
}

impl<T, U, W> FramedInner<T, U, W>
where
    T: AsyncWrite,
    U: Encoder,
    W: BorrowMut<WriteFrame>,
{
    fn poll_flush_until(self: Pin<&mut Self>, cx: &mut Context<'_>, limit: usize) -> Poll<Result<(), io::Error>> {
        let mut this = self.project();
        let state = this.state.borrow_mut();
        let orig_len = state.buffer.len();

        while state.buffer.len() > limit {
            let num_write = ready!(this.inner.as_mut().poll_write(cx, &state.buffer))?;

            if num_write == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "FramedWrite: end of input",
                )));
            }

            state.buffer.advance(num_write);
        }

        if orig_len != state.buffer.len() {
            this.inner.poll_flush(cx)
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

impl<T, U, W> Sink<U::Item> for FramedInner<T, U, W>
where
    T: AsyncWrite,
    U: Encoder,
    W: BorrowMut<WriteFrame>,
{
    type Error = Error<U::Error>;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush_until(cx, BACKPRESSURE_BOUNDARY - 1).map_err(Into::into)
    }

    fn start_send(self: Pin<&mut Self>, item: U::Item) -> Result<(), Self::Error> {
        let this = self.project();
        this.codec
            .encode(item, &mut this.state.borrow_mut().buffer)
            .map_err(Error::Codec)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush_until(cx, 0).map_err(Into::into)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        self.project().inner.poll_close(cx).map_err(Into::into)
    }
}

pub(super) struct ReadFrame {
    pub(super) buffer: BytesMut,
    pub(super) has_errored: bool,
}

impl Default for ReadFrame {
    fn default() -> Self {
        Self {
            buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
            has_errored: false,
        }
    }
}

impl From<BytesMut> for ReadFrame {
    fn from(mut buffer: BytesMut) -> Self {
        let size = buffer.capacity();
        if size < INITIAL_CAPACITY {
            buffer.reserve(INITIAL_CAPACITY - size);
        }

        Self {
            buffer,
            has_errored: false,
        }
    }
}

pub(super) struct WriteFrame {
    pub(super) buffer: BytesMut,
}

impl Default for WriteFrame {
    fn default() -> Self {
        Self {
            buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
        }
    }
}

impl From<BytesMut> for WriteFrame {
    fn from(mut buffer: BytesMut) -> Self {
        let size = buffer.capacity();
        if size < INITIAL_CAPACITY {
            buffer.reserve(INITIAL_CAPACITY - size);
        }

        Self { buffer }
    }
}

#[derive(Default)]
pub(super) struct RWFrames {
    pub(super) read: ReadFrame,
    pub(super) write: WriteFrame,
}

impl Borrow<ReadFrame> for RWFrames {
    fn borrow(&self) -> &ReadFrame {
        &self.read
    }
}
impl BorrowMut<ReadFrame> for RWFrames {
    fn borrow_mut(&mut self) -> &mut ReadFrame {
        &mut self.read
    }
}
impl Borrow<WriteFrame> for RWFrames {
    fn borrow(&self) -> &WriteFrame {
        &self.write
    }
}
impl BorrowMut<WriteFrame> for RWFrames {
    fn borrow_mut(&mut self) -> &mut WriteFrame {
        &mut self.write
    }
}
