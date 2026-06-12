
use bytes::{Buf, BufMut, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};


use crate::meta::{HttpRequest, HttpResponse};

pub struct HttpCodec;

impl Decoder for HttpCodec {
    type Item = HttpRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 創建一個足夠大的不鏽鋼陣列用來暫存解析出來的 Headers 引用
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        // 1. 嘗試解析 HTTP 請求頭
        match req.parse(src) {
            Ok(httparse::Status::Complete(header_len)) => {
                // 狀態為 Complete 代表成功讀取到 \r\n\r\n，請求頭已完整接收

                // 2. 獲取 Content-Length 以確定後續 Body 的長度
                let mut content_length = 0;
                for header in req.headers.iter() {
                    if header.name.eq_ignore_ascii_case("Content-Length") {
                        if let Ok(val_str) = std::str::from_utf8(header.value) {
                            if let Ok(len) = val_str.parse::<usize>() {
                                content_length = len;
                            }
                        }
                        break;
                    }
                }

                // 3. 檢查當前緩衝區的總長度是否足夠（Header 長度 + Body 長度）
                let total_frame_length = header_len + content_length;
                if src.len() < total_frame_length {
                    // 發生「半包」：Body 數據尚未完全到達網卡，通知 Tokio 繼續讀取
                    return Ok(None);
                }

                // 🚀 解析 URL 與 Query Parameters
                let method = req.method.unwrap_or("").to_string();
                let full_path = req.path.unwrap_or("");
                let mut path = full_path.to_string();
                let mut query_params = Vec::new();

                // 檢查是否帶有 '?' 記號
                if let Some(pos) = full_path.find('?') {
                    // 切割出純路徑
                    path = full_path[..pos].to_string();

                    // 獲取 '?' 後面的查詢字串 (例如: "id=100&type=system")
                    let query_str = &full_path[pos + 1..];

                    // 依據 '&' 切分多個參數對
                    for pair in query_str.split('&') {
                        if !pair.is_empty() {
                            // 依據 '=' 切分鍵與值，限制切分為 2 部分
                            let mut parts = pair.splitn(2, '=');
                            let key = parts.next().unwrap_or("").to_string();
                            let val = parts.next().unwrap_or("").to_string();

                            // 🌟 此處的百分號編碼（Percent-Encoding）隱患已先做標記（見文末高亮提醒）
                            query_params.push((key, val));
                        }
                    }
                }

                // 4. 數據充足，提取關鍵數據並轉換所有權（打破對 src 的借用）
                let mut parsed_headers = Vec::new();
                let mut cont_typ = Option::None;
                for h in req.headers.iter() {
                    let _name = h.name.to_string();
                    let _value = String::from_utf8_lossy(h.value).into_owned();

                    if _name.eq_ignore_ascii_case("Content-Type") {
                        cont_typ = Option::Some(_value);
                    } else {
                        parsed_headers.push((_name, _value));
                    }
                }

                // 5. 操縱緩衝區游標
                src.advance(header_len); // 消耗掉已經解析完的 Header 字節
                let body = src.split_to(content_length).to_vec(); // 切割出 Body 字節

                let http_request = HttpRequest {
                    method: method,
                    path: path,
                    query_params: query_params,
                    content_type: cont_typ,
                    headers: parsed_headers,
                    body: body,
                };

                // 6. 「黏包」處理：如果客戶端開啟 Keep-Alive 連續發送多個 HTTP 請求，
                // 剩餘的字節仍留在 src 中，Tokio 會自動再次觸發 decode
                Ok(Some(http_request))
            }
            Ok(httparse::Status::Partial) => {
                // 發生「半包」：連 \r\n\r\n 都還沒收齊，繼續等待網絡數據
                Ok(None)
            }
            Err(e) => {
                // 惡意請求或協議錯誤，斷開連接
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Malformed HTTP Request: {}", e),
                ))
            }
        }
    }
}

// 實作 Encoder：將 HttpResponse 結構體轉化為標準 HTTP/1.1 文本報文
impl Encoder<HttpResponse> for HttpCodec {
    type Error = io::Error;

    fn encode(&mut self, item: HttpResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // 1. 寫入狀態行 (Status Line) -> HTTP/1.1 200 OK\r\n
        let status_line = format!("HTTP/1.1 {} {}\r\n", item.status_code, item.status_text);
        dst.put_slice(status_line.as_bytes());

        // 2. 自動計算並寫入關鍵的 Content-Length 標頭
        let mut has_content_length = false;
        for (name, value) in &item.headers {
            if name.eq_ignore_ascii_case("Content-Length") {
                has_content_length = true;
            }
            dst.put_slice(format!("{}: {}\r\n", name, value).as_bytes());
        }

        if !has_content_length {
            dst.put_slice(format!("Content-Length: {}\r\n", item.body.len()).as_bytes());
        }

        // 3. 寫入空行（CRLF）標記 Headers 結束
        dst.put_slice(b"\r\n");

        // 4. 寫入實體負載 (Body)
        dst.put_slice(&item.body);
        Ok(())
    }
}
