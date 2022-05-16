use async_codec_lite::{Framed, LinesCodec};
use futures_lite::future::block_on;
use futures_util::{io::Cursor, stream::TryStreamExt};

#[test]
fn it_works() {
    let buf = "Hello\nWorld\nError".to_owned();
    let cur = Cursor::new(buf);
    let mut framed = Framed::new(cur, LinesCodec {});
    let next = block_on(framed.try_next()).unwrap();
    assert_eq!(next, Some(String::from("Hello\n")));
    let next = block_on(framed.try_next()).unwrap();
    assert_eq!(next, Some(String::from("World\n")));
    let next = block_on(framed.try_next()).unwrap();
    assert_eq!(next, None);
}
