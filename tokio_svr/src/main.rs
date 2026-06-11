use std::net::SocketAddr;
use tokio::net::{TcpListener, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

/// UDP 伺服器核心邏輯
async fn run_udp_server(addr: SocketAddr) -> Result<(), tokio::io::Error> {
    let socket = UdpSocket::bind(addr).await?;
    println!("UDP server listening on {}", addr);

    let mut buf = [0u8; 2048]; // UDP 數據包上限通常建議設較大

    loop {
        // 異步接收 UDP 數據包
        let (len, peer) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        println!("UDP received from {}: {:?} | String: {:?}", peer, data, String::from_utf8_lossy(data));

        // 根據接收到的對端地址，將數據回傳
        let len_sent = socket.send_to(data, peer).await?;
        if len_sent != len {
            eprintln!("Warning: UDP sent byte count mismatch (sent {}/{} bytes)", len_sent, len);
        }
    }
}