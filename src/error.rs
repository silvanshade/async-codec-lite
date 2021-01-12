#[derive(Debug, thiserror::Error)]
pub enum Error<C: std::error::Error + 'static> {
    #[error("codec error: {0}")]
    Codec(#[source] C),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
