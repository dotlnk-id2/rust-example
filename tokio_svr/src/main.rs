use std::net::SocketAddr;
use tokio::net::{TcpListener, UdpSocket};

use bytes::{Buf, BytesMut};
use std::io;
use tokio::sync::mpsc;
use tokio_util::codec::Decoder;

// 必須引入此 Trait，編譯器才能在 FramedRead 上找到 .next() 方法
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 定義監聽地址
    let http_addr: SocketAddr = "127.0.0.1:8088".parse()?;
    let tcp_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let udp_addr: SocketAddr = "127.0.0.1:8081".parse()?;

    println!("Starting Echo Server...");

    // 使用 tokio::select! 同時併發監聽 TCP 與 UDP 服務
    tokio::select! {
        http_res = run_tcp_server(http_addr,ProtocolType::HTTP) => {
            if let Err(e) = http_res {
                eprintln!("http server error: {}", e);
            }
        }
        tcp_res = run_tcp_server(tcp_addr,ProtocolType::TCP) => {
            if let Err(e) = tcp_res {
                eprintln!("TCP server error: {}", e);
            }
        }
        udp_res = run_udp_server(udp_addr) => {
            if let Err(e) = udp_res {
                eprintln!("UDP server error: {}", e);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum ProtocolType {
    HTTP,
    TCP,
    UDP,
}

/// 定義我們的自定義協議：[4 bytes 長度 (大端序)] + [N bytes 實際數據]
pub struct TcpProtocolCodec;

impl Decoder for TcpProtocolCodec {
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

/// 定義解析成功後的 HTTP 請求結構體
#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
pub struct HttpDecoder;

impl Decoder for HttpDecoder {
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

                // 4. 數據充足，提取關鍵數據並轉換所有權（打破對 src 的借用）
                let method = req.method.unwrap_or("").to_string();
                let path = req.path.unwrap_or("").to_string();

                let mut parsed_headers = Vec::new();
                for h in req.headers.iter() {
                    let name = h.name.to_string();
                    let value = String::from_utf8_lossy(h.value).into_owned();
                    parsed_headers.push((name, value));
                }

                // 5. 操縱緩衝區游標
                src.advance(header_len); // 消耗掉已經解析完的 Header 字節
                let body = src.split_to(content_length).to_vec(); // 切割出 Body 字節

                let http_request = HttpRequest {
                    method,
                    path,
                    headers: parsed_headers,
                    body,
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

async fn run_tcp_server(addr: SocketAddr, pt: ProtocolType) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(addr).await?;
    println!("{:?} server listening on {}", pt, addr);

    loop {
        let (socket, client_addr) = listener.accept().await?;
        println!("New {:?} connection from: {}", pt, client_addr);

        // 👈 🌟 這裡引發的下游併發資源隔離隱患，已先做標記（見文末高亮提醒）
        tokio::spawn(async move {
            match pt {
                // 2. 正確使用全路徑枚舉匹配
                ProtocolType::HTTP => {
                    // 3. 移除 Box::pin，直接在棧上分配，享受零成本抽象（Zero-cost abstraction）
                    let mut framed_reader = tokio_util::codec::FramedRead::new(socket, HttpDecoder);

                    while let Some(result) = framed_reader.next().await {
                        match result {
                            Ok(complete_packet) => {
                                process_http_packet(client_addr, complete_packet).await;
                            }
                            Err(e) => {
                                canal_error_log(client_addr, e);
                                break;
                            }
                        }
                    }
                }
                ProtocolType::TCP => {
                    // 將先前的自定義 TCP 長度解碼器完美整合進此分支
                    let mut framed_reader = tokio_util::codec::FramedRead::new(socket, TcpProtocolCodec);

                    while let Some(result) = framed_reader.next().await {
                        match result {
                            Ok(complete_packet) => {
                                process_tcp_packet(client_addr, complete_packet).await;
                            }
                            Err(e) => {
                                canal_error_log(client_addr, e);
                                break;
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("ProtocolType not support");
                }
            }
            println!("TCP client {} disconnected or completed", client_addr);
        });
    }
}

async fn run_udp_server(addr: SocketAddr) -> Result<(), tokio::io::Error> {
    // 1. 綁定 UDP 端口，並用 Arc 封裝以利在多個 Task 間共享
    let socket = std::sync::Arc::new(UdpSocket::bind(addr).await?);
    println!("UDP server listening on {}", addr);

    // 2. 創建一個異步通道 (Channel)，容量設為 1024 提供背壓保護
    let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1024);

    // 3. 克隆 Arc 指針給生產者 Task
    let socket_rx = std::sync::Arc::clone(&socket);

    // 【生產者 Task】：負責以極快速度從網卡接收數據，並塞入隊列
    tokio::spawn(async move {
        let mut buf = [0u8; 2048];
        loop {
            // 因為 Arc<UdpSocket> 實現了異步讀寫，直接呼叫 recv_from 即可
            match socket_rx.recv_from(&mut buf).await {
                Ok((len, peer)) => {
                    // 將字節切片拷貝進獨立的 Vec，確保內存絕對隔離，並發送至通道
                    if let Err(_) = tx.send((buf[..len].to_vec(), peer)).await {
                        eprintln!("Receiver channel closed");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("UDP recv error: {}", e);
                }
            }
        }
    });

    // 4. 【消費者主循環】：從隊列拿數據，處理耗時業務並回傳
    // 讓此循環常駐在當前線程，防止 run_udp_server 函數結束而導致進程退出
    while let Some((data, peer)) = rx.recv().await {
        // 每次回包都克隆一次 Arc 指針，並衍生新的 Task 處理，實現完全併發回包
        let socket_tx = std::sync::Arc::clone(&socket);

        tokio::spawn(async move {
            // 執行耗時業務
            let processed_data = complex_business_logic(data).await;

            // 將結果回傳給特定對端
            if let Err(e) = socket_tx.send_to(&processed_data, peer).await {
                eprintln!("UDP send error to {}: {}", peer, e);
            }
        });
    }

    Ok(())
}



// 模擬您的業務邏輯函數
async fn complex_business_logic(data: Vec<u8>) -> Vec<u8> {
    // 實際業務處理...
    data
}

/// 模擬業務邏輯處理函數
async fn process_http_packet(client_addr: SocketAddr, packet: HttpRequest) {
    // 此處收到的 packet 絕對是完整且獨立的，不會發生截斷或黏合
    println!(
        "Processed http packet from {}: http {:?} ",
        client_addr,
        packet
    );
    // TODO: 進行反序列化 (如 Protobuf/JSON) 與業務分發
}

async fn process_tcp_packet(addr: SocketAddr, packet: Vec<u8>) {
}

fn canal_error_log(addr: SocketAddr, e: std::io::Error) {
    eprintln!("Protocol violation or IO error from {}: {}", addr, e);
}
