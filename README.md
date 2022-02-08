<div align="center">
  <h1><code>async-codec-lite</code></h1>
  <p>
    <strong>Adaptors from AsyncRead/AsyncWrite to Stream/Sink using futures.</strong>
  </p>
  <p style="margin-bottom: 0.5ex;">
    <a href="https://silvanshade.github.io/async-codec-lite/async_codec_lite"><img
        src="https://img.shields.io/badge/docs-latest-blueviolet?logo=Read-the-docs&logoColor=white"
        /></a>
    <a href="https://github.com/silvanshade/async-codec-lite/actions"><img
        src="https://github.com/silvanshade/async-codec-lite/workflows/main/badge.svg"
        /></a>
    <a href="https://codecov.io/gh/silvanshade/async-codec-lite"><img
        src="https://codecov.io/gh/silvanshade/async-codec-lite/branches/main/graph/badge.svg"
        /></a>
  </p>
</div>

# async-codec-lite

Adaptors from AsyncRead/AsyncWrite to Stream/Sink using futures.

## Description

This crate is similar to existing crates that also provide `FramedWrite`
adapters. The difference between this crate and other non-tokio alternatives is
that it does not require `T: Unpin` in the `Sink` implementation for
`FramedWrite<T, E>`. This unnecessarily strict requirement made using
`FramedWrite` with `tower-lsp` problematic, as discussed in the issue
[here](https://github.com/matthunz/futures-codec/issues/46).

## Acknowledgements

This crate is based on code and ideas from the following crates:

* [matthunz/futures-codec](https://github.com/matthunz/futures-codec)
* [tokio-rs/tokio](https://github.com/tokio-rs/tokio)
