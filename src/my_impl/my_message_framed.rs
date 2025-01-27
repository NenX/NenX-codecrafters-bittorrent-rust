use std::io;

use anyhow::Context;
use anyhow::Result;
use bytes::Buf;
use bytes::BufMut;
use bytes::BytesMut;
use tokio::net::TcpStream;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use tokio_util::codec::Framed;

use super::MyPeerMsg;
use super::MyPeerMsgTag;

const LEN_BYTE: usize = u32::BITS as usize / 8;
const MAX: usize = u16::MAX as usize;
pub type MyFramed<'a> = Framed<&'a mut TcpStream, MyPeerMsgFramed>;
pub struct MyPeerMsgFramed;

impl Decoder for MyPeerMsgFramed {
    type Item = MyPeerMsg;
    type Error = std::io::Error;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let total_len = src.len();

        if total_len < LEN_BYTE {
            return Ok(None);
        }

        let len_slice = src[..4].try_into().context("bytes to len").unwrap();
        let len = u32::from_be_bytes(len_slice) as usize;

        if len == 0 {
            src.advance(LEN_BYTE);
            return self.decode(src);
        }
        let expected_len = LEN_BYTE + len;
        if total_len < expected_len {
            src.reserve(expected_len - total_len);
            return Ok(None);
        }
        if len - 1 > MAX {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let tag = MyPeerMsgTag::try_from(src[LEN_BYTE])
            .context(format!("into tag {:?}", src))
            .unwrap();

        let payload = if len == 1 {
            vec![]
        } else {
            src[LEN_BYTE + 1..LEN_BYTE + len].to_vec()
        };
        src.advance(4 + len);

        let msg = MyPeerMsg { payload, tag };

        Ok(Some(msg))
    }
}
impl Encoder<MyPeerMsg> for MyPeerMsgFramed {
    type Error = std::io::Error;
    fn encode(&mut self, item: MyPeerMsg, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload_len = item.payload.len();

        if payload_len > MAX {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        let total_len_slice = u32::to_be_bytes((payload_len + 1).try_into().unwrap());
        dst.reserve(LEN_BYTE + 1 + payload_len);

        dst.extend_from_slice(&total_len_slice);

        dst.put_u8(item.tag as u8);

        dst.extend_from_slice(&item.payload);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anyhow::{Context, Result};
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio::time::{self};
    use tokio_util::codec;

    use super::*;
    #[test]
    fn a() {
        let mut f = MyPeerMsgFramed;
        let a = b"\0\0\01\x14\0d1:md11:ut_metadatai239ee13:metadata_sizei132ee";
        let a = f.decode(&mut BytesMut::from_iter(a.iter()));
    }
    #[tokio::test]
    async fn test1() -> Result<()> {
        let f1 = move || async move {
            let l = tokio::net::TcpListener::bind("0.0.0.0:2233").await.unwrap();
            let (socket, _) = l.accept().await.unwrap();
            let mut frame = codec::Framed::new(socket, MyPeerMsgFramed);

            let a = frame.next().await.context("read message").unwrap().unwrap();

            let m = MyPeerMsg {
                tag: MyPeerMsgTag::Interested,
                payload: [1].repeat(MAX),
            };
            frame.send(m.clone()).await.context("send").unwrap();
            frame.send(m).await.context("send").unwrap();
        };
        let f2 = || async {
            let socket = tokio::net::TcpStream::connect("0.0.0.0:2233")
                .await
                .unwrap();
            let mut frame = codec::Framed::new(socket, MyPeerMsgFramed);
            let m = MyPeerMsg {
                tag: MyPeerMsgTag::Interested,
                payload: vec![1, 2, u8::MAX, 4],
            };
            frame.send(m).await.context("send").unwrap();
            let a = frame.next().await.context("read message").unwrap().unwrap();
            let len = a.payload.len();
            let slice = a.payload.split_at(len - 5);

            let a = frame.next().await.context("read message").unwrap().unwrap();
        };
        let h1 = tokio::spawn(async move {
            f1().await;
        });
        let h2 = tokio::spawn(async move {
            time::sleep(Duration::from_secs(1)).await;
            f2().await;
        });
        h1.await.context("h1 wait").unwrap();
        h2.await.context("h2 wait").unwrap();
        Ok(())
    }
}
