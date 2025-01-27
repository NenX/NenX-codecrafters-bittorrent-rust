use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::MyTorrentInfo;

#[derive(Debug, Clone)]

pub struct MyExtMetaDataPayload {
    pub ext_msg_id: u8,
    pub dic: MyExtMetaDataPayloadDic,
    pub info: Option<MyTorrentInfo>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyExtMetaDataPayloadDic {
    pub msg_type: usize,
    pub piece: usize,
    // size of MyTorrentInfo
    pub total_size: Option<usize>,
}
impl MyExtMetaDataPayload {
    pub fn new(ext_msg_id: u8, msg_type: usize, piece: usize) -> Self {
        let m = MyExtMetaDataPayload {
            ext_msg_id,
            dic: MyExtMetaDataPayloadDic {
                msg_type,
                piece,
                total_size: None,
            },
            info: None,
        };
        m
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut v = vec![];
        v.push(self.ext_msg_id);
        let a = serde_bencode::to_bytes(&self.dic)?;
        v.extend(a);

        if let Some(info) = &self.info {
            let a = serde_bencode::to_bytes(&info)?;
            v.extend(a);
        }
        Ok(v)
    }
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if 0 == b.len() {
            return None;
        }
        let ext_msg_id: u8 = b.get(0).unwrap().clone();
        let dic: MyExtMetaDataPayloadDic =
            serde_bencode::from_bytes(&b[1..]).expect("parse ext dic");

        if let Some(total_size) = dic.total_size {
            let info: MyTorrentInfo =
                serde_bencode::from_bytes(&b[(b.len() - total_size)..]).expect("parse ext dic");

            let a = Self {
                ext_msg_id,
                dic,
                info: Some(info),
            };
            return Some(a);
        }

        let a = Self {
            ext_msg_id,
            dic,
            info: None,
        };
        return Some(a);
    }
}

#[cfg(test)]
mod a {

    use super::*;

    #[test]
    fn tt() {}
}
