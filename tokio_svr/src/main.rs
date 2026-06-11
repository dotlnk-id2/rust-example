use std::net::SocketAddr;
use tokio::net::{TcpListener, UdpSocket};

use bytes::{Buf, BufMut, BytesMut};
use std::io;
use tokio::sync::mpsc;
use tokio_util::codec::{Decoder, Encoder};

// 必須引入此 Trait，編譯器才能在 FramedRead 上找到 .next() 方法
use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::Framed; // 👈 由 FramedRead 改為雙向的 Framed // 👈 必須引入 SinkExt 才能使用 .send()

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

/// 定義解析成功後的 HTTP 請求結構體
#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String, // 👈 改造後此處僅保留純路徑 (例如: "/search")
    pub query_params: Vec<(String, String)>, // 👈 新增：用於儲存 URL 參數
    pub content_type: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
// 封裝自定義的 HTTP 回應結構
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

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
                    query_params:query_params,
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
async fn run_tcp_server(addr: SocketAddr, pt: ProtocolType) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(addr).await?;
    println!("{:?} server listening on {}", pt, addr);

    loop {
        let (socket, client_addr) = listener.accept().await?;
        println!("New {:?} connection from: {}", pt, client_addr);

        tokio::spawn(async move {
            match pt {
                ProtocolType::HTTP => {
                    // 使用 Framed::new 建立雙向通道
                    let mut framed = Framed::new(socket, HttpCodec);

                    while let Some(result) = framed.next().await {
                        match result {
                            Ok(request) => {
                                // // 業務處理：依據請求內容構建 HTTP 回應報文
                                // println!("Received HTTP Request: \n{:#?}", request);

                                // let response = HttpResponse {
                                //     status_code: 200,
                                //     status_text: "OK",
                                //     headers: vec![
                                //         ("Content-Type".to_string(), "text/plain".to_string()),
                                //         ("Connection".to_string(), "keep-alive".to_string()),
                                //     ],
                                //     body: b"Hello from Tokio HTTP Asynchronous Server!".to_vec(),
                                // };

                                // // 🌟 這裡引發的下游網路狀態控制隱患，已先做標記（見文末高亮提醒）
                                // if let Err(e) = framed.send(response).await {
                                //     eprintln!("Failed to send HTTP response: {}", e);
                                //     break;
                                // }

                                // 1. 將請求丟入分發器，取得計算完畢的 Response 物件
                                let response = dispatch_http_request(request).await;

                                // 2. 透過雙向 Framed 將回應序列化並發送回客戶端
                                if let Err(e) = framed.send(response).await {
                                    eprintln!("Failed to send HTTP response: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("HTTP Protocol error: {}", e);
                                break;
                            }
                        }
                    }
                }
                ProtocolType::TCP => {
                    let mut framed = Framed::new(socket, TcpCodec);

                    while let Some(result) = framed.next().await {
                        match result {
                            Ok(complete_packet) => {
                                println!(
                                    "Received TCP custom packet: {} bytes",
                                    complete_packet.len()
                                );

                                // 業務處理：原樣返回（Echo）或加工後返回
                                let response_packet = complete_packet; // 此處示例為原樣返回

                                if let Err(e) = framed.send(response_packet).await {
                                    eprintln!("Failed to send TCP response: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("TCP Protocol error: {}", e);
                                break;
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("Not support protocol");
                }
            }
            println!("{:?} client {} disconnected or completed", pt, client_addr);
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
            let processed_data = process_udp_packet(data).await;

            // 將結果回傳給特定對端
            if let Err(e) = socket_tx.send_to(&processed_data, peer).await {
                eprintln!("UDP send error to {}: {}", peer, e);
            }
        });
    }

    Ok(())
}

// 模擬您的業務邏輯函數
async fn process_udp_packet(data: Vec<u8>) -> Vec<u8> {
    // 實際業務處理...
    data
}

/// 模擬業務邏輯處理函數
async fn process_http_packet(client_addr: SocketAddr, packet: HttpRequest) {
    // 此處收到的 packet 絕對是完整且獨立的，不會發生截斷或黏合
    println!(
        "Processed http packet from {}: \n{:#?} ",
        client_addr, packet
    );
    // TODO: 進行反序列化 (如 Protobuf/JSON) 與業務分發
}

async fn process_tcp_packet(addr: SocketAddr, packet: Vec<u8>) {}

fn canal_error_log(addr: SocketAddr, e: std::io::Error) {
    eprintln!("Protocol violation or IO error from {}: {}", addr, e);
}

/// HTTP 路由分發器：負責將不同 Method 與 Path 的請求導向專屬處理函數
async fn dispatch_http_request(request: HttpRequest) -> HttpResponse {
    // 將方法名轉為大寫並匹配字串切片
    match request.method.to_uppercase().as_str() {
        "GET" => handle_get(request).await,
        "POST" => handle_post(request).await,
        "OPTIONS" => handle_options(request).await,
        _ => {
            // 對於未實作的 Method（如 PUT, DELETE），回傳 405 Method Not Allowed
            HttpResponse {
                status_code: 405,
                status_text: "Method Not Allowed",
                headers: vec![
                    ("Content-Type".to_string(), "text/plain".to_string()),
                    ("Connection".to_string(), "keep-alive".to_string()),
                ],
                body: b"405 Method Not Allowed".to_vec(),
            }
        }
    }
}

/// 專門處理 GET 請求
async fn handle_get(request: HttpRequest) -> HttpResponse {
    println!("Handling GET request for path: {:#?}", request);
    
    // 尋找名為 "id" 的參數
    let target_id = request.query_params.iter()
        .find(|(k, _)| k == "id")
        .map(|(_, v)| v.as_str());

    let body_content = match request.path.as_str() {
        "/user" => {
            if let Some(id) = target_id {
                format!("Fetching data for User ID: {}", id).into_bytes()
            } else {
                b"Missing 'id' parameter".to_vec()
            }
        }
        _ => b"Hello World".to_vec(),
    };

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        body: body_content,
    }
}

/// 專門處理 POST 請求
async fn handle_post(request: HttpRequest) -> HttpResponse {
    println!("Handling POST request for path: {:#?}", request);

    // 運用您在 Codec 中貼心提取的 content_type 進行嚴謹校驗
    if let Some(ref c_type) = request.content_type {
        if c_type.contains("application/json") {
            // 這裡可以安全地對 request.body 進行 JSON 反序列化 (例如使用 serde_json)
            println!("Received JSON Payload size: {} bytes", request.body.len());
        }
    }

    HttpResponse {
        status_code: 201,
        status_text: "Created",
        headers: vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Connection".to_string(), "keep-alive".to_string()),
        ],
        body: b"{\"message\":\"Data processed successfully\"}".to_vec(),
    }
}

/// 專門處理 OPTIONS 請求（CORS 跨域預檢）
async fn handle_options(request: HttpRequest) -> HttpResponse {
    println!("Handling OPTIONS preflight request for path: {:#?}", request);

    // OPTIONS 請求通常不需要 Body，但必須回傳正確的 跨域允許 Headers
    HttpResponse {
        status_code: 204, // 204 No Content 是標準 OPTIONS 最常見的回傳碼
        status_text: "No Content",
        headers: vec![
            ("Access-Control-Allow-Origin".to_string(), "*".to_string()),
            (
                "Access-Control-Allow-Methods".to_string(),
                "GET, POST, OPTIONS".to_string(),
            ),
            (
                "Access-Control-Allow-Headers".to_string(),
                "Content-Type, Authorization".to_string(),
            ),
            ("Access-Control-Max-Age".to_string(), "86400".to_string()), // 快取預檢結果 24 小時
            ("Connection".to_string(), "keep-alive".to_string()),
        ],
        body: Vec::new(), // 空 Body
    }
}

/// 404 輔助函數
fn handle_404() -> HttpResponse {
    HttpResponse {
        status_code: 404,
        status_text: "Not Found",
        headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        body: b"404 Not Found".to_vec(),
    }
}
