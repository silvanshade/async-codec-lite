use async_codec_lite::{Framed, LinesCodec};
use futures_lite::future::block_on;
use futures_util::{io::Cursor, stream::TryStreamExt};

#[test]
fn it_works() {
    let buf = "Hello\nWorld\nError".to_owned();
    let cur = Cursor::new(buf);

    let mut framed = Framed::new(cur, LinesCodec {});
    let next = block_on(framed.try_next()).unwrap().unwrap();
    assert_eq!(next, "Hello\n");
    let next = block_on(framed.try_next()).unwrap().unwrap();
    assert_eq!(next, "World\n");

    assert!(block_on(framed.try_next()).is_err());
}
