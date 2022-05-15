use crate::{
    codec::{Decoder, Encoder},
    error::Error,
};
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
        let mut pinned = self.project();
        let state: &mut ReadFrame = pinned.state.borrow_mut();

        loop {
            // Return `None` if we have encountered an error from the underlying decoder
            // See: https://github.com/tokio-rs/tokio/issues/3976
            if state.has_errored {
                log::trace!("Returning None and setting paused");
                state.is_readable = false;
                state.has_errored = false;
                return Poll::Ready(None);
            }

            if state.is_readable {
                // pausing or framing
                if state.eof {
                    // pausing
                    let frame = pinned.codec.decode_eof(&mut state.buffer).map_err(|err| {
                        log::trace!("Got an error, going to errored state");
                        state.has_errored = true;
                        Error::Codec(err)
                    })?;
                    if frame.is_none() {
                        // prepare pausing -> paused
                        state.is_readable = false;
                    }
                    // implicit pausing -> pausing or pausing -> paused
                    return Poll::Ready(frame.map(Ok));
                }

                // framing
                log::trace!("attempting to decode a frame");

                if let Some(frame) = pinned.codec.decode(&mut state.buffer).map_err(|err| {
                    log::trace!("Got an error, going to errored state");
                    state.has_errored = true;
                    Error::Codec(err)
                })? {
                    log::trace!("frame decoded from buffer");
                    // implicit framing -> framing
                    return Poll::Ready(Some(Ok(frame)));
                }

                // framing -> reading
                state.is_readable = false;
            }

            // reading or paused
            state.buffer.reserve(1);
            let bytes_read = ready!(pinned.inner.as_mut().poll_read(cx, &mut state.buffer).map_err(|err| {
                log::trace!("Got an error, going to errored state");
                state.has_errored = true;
                err
            }))?;

            if bytes_read == 0 {
                if state.eof {
                    return Poll::Ready(None);
                }
                // prepare reading -> paused
                state.eof = true;
            } else {
                // prepare pasued -> framing or noop reading -> framing
                state.eof = false;
            }

            // paused -> framing or reading -> framing or reading -> pausing
            state.is_readable = true;
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
        let pinned = self.project();
        if pinned.state.borrow_mut().buffer.len() >= BACKPRESSURE_BOUNDARY {
            pinned.inner.poll_flush(cx).map_err(Into::into)
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(self: Pin<&mut Self>, item: U::Item) -> Result<(), Self::Error> {
        let pinned = self.project();
        pinned
            .codec
            .encode(item, &mut pinned.state.borrow_mut().buffer)
            .map_err(Error::Codec)?;
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        log::trace!("flushing framed transport");
        let mut pinned = self.project();

        while !pinned.state.borrow_mut().buffer.is_empty() {
            let WriteFrame { buffer } = pinned.state.borrow_mut();
            log::trace!("remaining: {}, writing;", buffer.len());

            let bytes_written = ready!(pinned.inner.as_mut().poll_write(cx, buffer))?;

            if bytes_written == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write frame to transport",
                )
                .into()));
            }
        }

        // Try flushing the underlying IO
        ready!(pinned.inner.poll_flush(cx))?;

        log::trace!("framed transport flushed");

        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        ready!(self.project().inner.poll_close(cx))?;

        Poll::Ready(Ok(()))
    }
}

pub(super) struct ReadFrame {
    pub(super) buffer: BytesMut,
    pub(super) eof: bool,
    pub(super) has_errored: bool,
    pub(super) is_readable: bool,
}

impl Default for ReadFrame {
    fn default() -> Self {
        Self {
            buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
            eof: false,
            has_errored: false,
            is_readable: false,
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
            eof: false,
            has_errored: false,
            is_readable: size > 0,
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
