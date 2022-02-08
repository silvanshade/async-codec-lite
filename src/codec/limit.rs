use super::{Decoder, Encoder};
use bytes::{Buf, BytesMut};

#[allow(missing_docs)]
pub trait SkipAheadHandler: Sized + std::fmt::Debug {
    fn continue_skipping(self, src: &[u8]) -> anyhow::Result<(usize, Option<Self>)>;
}

impl SkipAheadHandler for () {
    fn continue_skipping(self, _: &[u8]) -> anyhow::Result<(usize, Option<Self>)> {
        Ok((0, None))
    }
}

#[allow(missing_docs)]
pub trait DecoderWithSkipAhead: Decoder {
    type Handler: SkipAheadHandler;

    fn prepare_skip_ahead(&mut self, src: &mut BytesMut) -> Self::Handler;
}

#[derive(Debug)]
pub struct LimitCodec<C: DecoderWithSkipAhead> {
    inner: C,
    max_frame_size: usize,
    skip_ahead_state: Option<<C as DecoderWithSkipAhead>::Handler>,
    decoder_defunct: bool,
}

impl<C> LimitCodec<C>
where
    C: DecoderWithSkipAhead,
{
    #[allow(missing_docs)]
    pub fn new(inner: C, max_frame_size: usize) -> Self {
        Self {
            inner,
            max_frame_size,
            skip_ahead_state: None,
            decoder_defunct: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LimitError<E: std::error::Error + 'static> {
    #[error("frame size limit exceeded (detected at {0} bytes)")]
    LimitExceeded(usize),
    #[error("codec couldn't recover from invalid or too big frame")]
    Defunct,
    #[error(transparent)]
    Inner(#[from] E),
}

impl<C> Encoder for LimitCodec<C>
where
    C: Encoder + DecoderWithSkipAhead,
{
    type Error = LimitError<<C as Encoder>::Error>;
    type Item = <C as Encoder>::Item;

    fn encode(&mut self, src: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut tmp_dst = dst.split_off(dst.len());
        self.inner.encode(src, &mut tmp_dst)?;

        if tmp_dst.len() > self.max_frame_size {
            return Err(LimitError::LimitExceeded(tmp_dst.len()));
        }

        dst.unsplit(tmp_dst);
        Ok(())
    }
}

impl<C> Decoder for LimitCodec<C>
where
    C: DecoderWithSkipAhead,
{
    type Error = LimitError<<C as Decoder>::Error>;
    type Item = <C as Decoder>::Item;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        while let Some(sas) = self.skip_ahead_state.take() {
            match sas.continue_skipping(src) {
                Ok((amount, next)) => {
                    self.skip_ahead_state = next;
                    debug_assert!(amount <= src.len());
                    src.advance(amount);
                    debug_assert!(amount != 0 || self.skip_ahead_state.is_none());
                    if src.is_empty() {
                        return Ok(None);
                    }
                },
                Err(_err) => {
                    self.decoder_defunct = true;
                },
            }
        }

        if self.decoder_defunct {
            src.clear();
            return Err(LimitError::Defunct);
        }
        match self.inner.decode(src) {
            Ok(None) if src.len() > self.max_frame_size => {
                self.skip_ahead_state = Some(self.inner.prepare_skip_ahead(src));
                Err(LimitError::LimitExceeded(src.len()))
            },
            Ok(x) => Ok(x),
            Err(x) => Err(LimitError::Inner(x)),
        }
    }
}
