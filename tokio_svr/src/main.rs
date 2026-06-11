use std::net::SocketAddr;
use tokio::net::{TcpListener, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tokio::sync::mpsc;

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

#[derive(Debug, Clone)]
pub enum ProtocolType {
    HTTP,
    TCP,
    UDP
}

/// TCP 伺服器核心邏輯
async fn run_tcp_server(addr: SocketAddr,pt: ProtocolType) -> Result<(), tokio::io::Error> {
    let listener = TcpListener::bind(addr).await?;
    println!("{:?} server listening on {}",pt, addr);

    loop {
        // 異步等待客戶端連接
        let (mut socket, client_addr) = listener.accept().await?;
        println!("New {:?} connection from: {}", pt, client_addr);

        // 為每個 TCP 連接衍生一個獨立的 Tokio Task，實現非阻塞併發
        tokio::spawn(async move {
            let mut buf = [0u8; 8];
            loop {
                match socket.read(&mut buf).await {
                    // Return Ok(0) 代表客戶端關閉了連接 (EOF)
                    Ok(0) => {
                        println!("TCP client {} disconnected", client_addr);
                        break;
                    }
                    Ok(n) => {
                        let data = &buf[..n];
                        // 打印讀取到的字節（包含原始字節與嘗試解析為字串的表現形式）
                        println!("TCP received from {}: {:?} | String: {:?}", client_addr, data, String::from_utf8_lossy(data));

                        // 將數據原樣寫回調用方
                        if let Err(e) = socket.write_all(data).await {
                            eprintln!("Failed to write to TCP socket: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from TCP socket: {}", e);
                        break;
                    }
                }
            }
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
