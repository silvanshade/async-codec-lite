use super::inner::{FramedInner, WriteFrame};
use crate::{codec::Encoder, error::Error};
pub use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use futures_io::AsyncWrite;
use futures_sink::Sink;
use pin_project_lite::pin_project;
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    pub struct FramedWrite<T, E> {
        #[pin]
        inner: FramedInner<T, E, WriteFrame>,
    }
}

impl<T, E> FramedWrite<T, E>
where
    T: AsyncWrite,
{
    pub fn new(inner: T, encoder: E) -> FramedWrite<T, E> {
        FramedWrite {
            inner: FramedInner {
                inner,
                codec: encoder,
                state: WriteFrame::default(),
            },
        }
    }
}

impl<T, E> FramedWrite<T, E> {
    pub fn get_ref(&self) -> &T {
        &self.inner.inner
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner.inner
    }

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.project().inner.project().inner
    }

    pub fn into_inner(self) -> T {
        self.inner.inner
    }

    pub fn encoder(&self) -> &E {
        &self.inner.codec
    }

    pub fn encoder_mut(&mut self) -> &mut E {
        &mut self.inner.codec
    }
}

impl<T, E> Sink<E::Item> for FramedWrite<T, E>
where
    T: AsyncWrite,
    E: Encoder,
{
    type Error = Error<E::Error>;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: E::Item) -> Result<(), Self::Error> {
        self.project().inner.start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_close(cx)
    }
}

impl<T, D> Stream for FramedWrite<T, D>
where
    T: Stream,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.project().inner.poll_next(cx)
    }
}

impl<T, U> fmt::Debug for FramedWrite<T, U>
where
    T: fmt::Debug,
    U: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FramedWrite")
            .field("inner", &self.get_ref())
            .field("encoder", &self.encoder())
            .field("buffer", &self.inner.state.buffer)
            .finish()
    }
}
