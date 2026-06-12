
use bytes::{Buf, BufMut, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

/// 定義我們的自定義協議：[4 bytes 長度 (大端序)] + [N bytes 實際數據]
pub struct TcpCodec;

impl Decoder for TcpCodec {
    type Item = Vec<u8>; // 解碼成功後返回完整的字節包
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 1. 檢查是否足夠讀取 4 bytes 的長度頭
        if src.len() < 4 {
            return Ok(None); // 數據不足，通知 Tokio 繼續讀取網絡流
        }

        // 2. 預讀取長度頭 (不消耗游標)
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let payload_length = u32::from_be_bytes(length_bytes) as usize;

        // 3. 檢查當前緩衝區的數據是否足夠包含整個封包 (Header + Payload)
        let total_frame_length = 4 + payload_length;
        if src.len() < total_frame_length {
            // 發生「半包」：通知 Tokio 繼續接收數據，直到滿足 total_frame_length
            return Ok(None);
        }

        // 4. 數據充足，開始提取！
        src.advance(4); // 推進游標，消耗掉 4 bytes 的 Header
        let payload = src.split_to(payload_length).freeze().to_vec(); // 零拷貝切割出 Payload

        // 5. 發生「黏包」時：剩餘的數據會保留在 src 中，等待下一次 decode 被調用
        Ok(Some(payload))
    }
}

// 實作 Encoder：將要返回的字節包加上 4 字節大端序長度頭
impl Encoder<Vec<u8>> for TcpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // 1. 寫入 4 字節的長度前綴
        dst.put_u32(item.len() as u32);
        // 2. 寫入實際數據
        dst.put_slice(&item);
        Ok(())
    }
}
