use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{TcpListener, UdpSocket};

// use bytes::{Buf, BufMut, BytesMut};
// use std::io;
use tokio::sync::mpsc;
// use tokio_util::codec::{Decoder, Encoder};

// 必須引入此 Trait，編譯器才能在 FramedRead 上找到 .next() 方法
use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::Framed; // 👈 由 FramedRead 改為雙向的 Framed // 👈 必須引入 SinkExt 才能使用 .send()

use sqlx::postgres::PgPoolOptions;
use sqlx_postgres::PgConnectOptions;



// 🚀 1. 關鍵：宣告引入 protocol 模組資料夾
mod meta;
mod protocol;

// 🚀 2. 使用在 mod.rs 中重出口的乾淨路徑
use meta::{HttpRequest, HttpResponse};
use protocol::{HttpCodec, TcpCodec};

const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::Config::builder()
        .add_source(config::File::with_name(DEFAULT_CONFIG_PATH))
        .build()
        .expect(format!("load fs={} error", DEFAULT_CONFIG_PATH).as_str());

    let app_cfg: meta::config_meta::AppConfig = cfg.try_deserialize()?;

    // 数据库连接
    let db_opt = &app_cfg.pool_opt;
    let db_cfg = app_cfg.database.get("slave-2").unwrap();
    println!(
         "database connection config: num={} wait_time={}s ",
         db_opt.min_conn,
         db_opt.acquire_timeout
     );
    let db_st = tokio::time::Instant::now();

    let opt = PgConnectOptions::new()
        .ssl_mode(sqlx_postgres::PgSslMode::Require)
        //.ssl_mode(sqlx_postgres::PgSslMode::Prefer)
        .host(&db_cfg.db_host)
        .port(db_cfg.db_port)
        .database(&db_cfg.db_name)
        .username(&db_cfg.db_user)
        .password(&db_cfg.from_raw_pwd().unwrap())
        .application_name(&db_cfg.db_alias);

    let db_url = db_cfg.clone().from_db().unwrap();
    let db_pool = PgPoolOptions::new()
        .min_connections(db_opt.min_conn)
        .max_connections(db_opt.max_conn)
        .acquire_timeout(Duration::from_secs(db_opt.acquire_timeout))
        .idle_timeout(Duration::from_secs(db_opt.idle_timeout))
        .max_lifetime(Duration::from_secs(db_opt.max_lifetime))
        .acquire_slow_threshold(Duration::from_millis(500))
        .test_before_acquire(true)
        .connect_with(opt)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

     println!(
         "{:?} conn to {:?} num={:?} elapse_time={:?} finish!!!!",
         db_cfg,
         db_url,
         db_opt.min_conn,
         db_st.elapsed()
     );

    let ctx = Arc::new(AppContext {
        pg_db: db_pool,
    });

    // 定義監聽地址
    let http_addr: SocketAddr = format!("{}:{}", app_cfg.http_bind, app_cfg.http_port)
        .parse()
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to parse SocketAddr: {}", e),
            )
        })?;
    let tcp_addr: SocketAddr = format!("{}:{}", app_cfg.tcp_bind, app_cfg.tcp_port)
        .parse()
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to parse SocketAddr: {}", e),
            )
        })?;
    let udp_addr: SocketAddr = format!("{}:{}", app_cfg.udp_bind, app_cfg.udp_port)
        .parse()
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Failed to parse SocketAddr: {}", e),
            )
        })?;

    println!("Starting Echo Server...");

    

    // 使用 tokio::select! 同時併發監聽 TCP 與 UDP 服務
    tokio::select! {
        http_res = run_tcp_server(http_addr,TcpProtocolType::HTTP,Arc::clone(&ctx)) => {
            if let Err(e) = http_res {
                eprintln!("HTTP server error: {}", e);
            }
        }
        tcp_res = run_tcp_server(tcp_addr,TcpProtocolType::TCP,Arc::clone(&ctx)) => {
            if let Err(e) = tcp_res {
                eprintln!("TCP server error: {}", e);
            }
        }
        udp_res = run_udp_server(udp_addr,Arc::clone(&ctx)) => {
            if let Err(e) = udp_res {
                eprintln!("UDP server error: {}", e);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum TcpProtocolType {
    HTTP,
    TCP,
    GRPC,
}

async fn run_tcp_server(addr: SocketAddr, pt: TcpProtocolType, ctx: Arc<AppContext>) -> Result<(), std::io::Error> {
    let listener = TcpListener::bind(addr).await?;
    println!("{:?} server listening on {}", pt, addr);

    loop {
        let (socket, client_addr) = listener.accept().await?;
        println!("New {:?} connection from: {}", pt, client_addr);

        tokio::spawn(async move {
            match pt {
                TcpProtocolType::HTTP => {
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
                TcpProtocolType::TCP => {
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

async fn run_udp_server(addr: SocketAddr, ctx: Arc<AppContext>) -> Result<(), tokio::io::Error> {
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
    println!("Handling request for path: {:?}", request);

    // 方法 	安全 	冪等 	可快取
    // GET 	    是 	    是 	    是
    // HEAD 	是 	    是 	    是
    // OPTIONS  是 	    是 	    否
    // TRACE 	是 	    是 	    否
    // PUT 	    否 	    是 	    否
    // DELETE 	否 	    是 	    否
    // POST 	否 	    否 	    條件的*
    // PATCH 	否 	    否 	    條件的*
    // CONNECT  否 	    否 	    否

    // 將方法名轉為大寫並匹配字串切片
    match request.method.to_uppercase().as_str() {
        "GET" => handle_get(request).await,
        "POST" => handle_post(request).await,
        "OPTIONS" => handle_options(request).await,
        "PUT" => HttpResponse {
            status_code: 200,
            status_text: "OK",
            headers: vec![
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Connection".to_string(), "keep-alive".to_string()),
            ],
            body: b"PUT".to_vec(),
        },
        "CONNECT" => HttpResponse {
            status_code: 200,
            status_text: "OK",
            headers: vec![
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Connection".to_string(), "keep-alive".to_string()),
            ],
            body: b"CONNECT".to_vec(),
        },
        "DELETE" => HttpResponse {
            status_code: 200,
            status_text: "OK",
            headers: vec![
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Connection".to_string(), "keep-alive".to_string()),
            ],
            body: b"DELETE".to_vec(),
        },
        _ => {
            // 對於未實作的 Method（如 HEAD, TRACE, PATCH），回傳 405 Method Not Allowed
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
    // 尋找名為 "id" 的參數
    let target_id = request
        .query_params
        .iter()
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

// fn new_socket_addr(bind:IpAddr,port:u16) -> SocketAddr {
//     // 🚀 核心優化：直接在棧上透過二進位制數值構造，具備 O(1) 極致性能
//     SocketAddr::new(bind, port)
// }

#[derive(Clone)]
pub struct AppContext {
    pub pg_db: sqlx::Pool<sqlx::Postgres>,
    
}
