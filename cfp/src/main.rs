use async_trait::async_trait;
use pingora::prelude::*;
use std::sync::Arc;


pub struct MyProxy;

#[derive(Debug)]
pub struct MyCtx {
    // 用于传递中间状态（如客户端真实IP）
    pub client_ip: String,
}

#[async_trait]
impl ProxyHttp for MyProxy {
    type CTX = MyCtx;

    fn new_ctx(&self) -> Self::CTX {
        MyCtx {
            client_ip: String::new(),
        }
    }

    // 1. 在请求最开始，读取连接 Socket 的客户端 IP
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        println!("request_filter");
        if let Some(client_addr) = session.client_addr() {
            // 提取纯 IP 字符串 (忽略端口)
            if let Some(ip) = client_addr.as_inet() {
                ctx.client_ip = ip.ip().to_string();
                // println!("ctx={:?}",ctx);
            }
        }
        Ok(false) // 返回 false 表示继续执行后续处理
    }

    // 2. 选择 Upstream 后端 (支持 TLS/HTTPS)
    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        println!("upstream_peer");
        // 配置目标 Upstream: 地址为 127.0.0.1:8443，启用 TLS (HTTPS)
        let mut peer = HttpPeer::new("111.63.65.247:443", true, "www.baidu.com".to_string());
        
        // 如果后端使用了自签名证书，可按需关闭证书校验：
        // peer.options.verify_cert = false;
        
        Ok(Box::new(peer))
    }

    // 3. 在向 Upstream 发送请求前修改 Header
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        println!("upstream_request_filter");
        if !ctx.client_ip.is_empty() {
            // 设置 X-Real-IP
            let _ = upstream_request.insert_header("X-Real-IP", &ctx.client_ip);

            // 追加或设置 X-Forwarded-For
            if let Some(existing_xff) = upstream_request.headers.get("X-Forwarded-For") {
                if let Ok(xff_str) = existing_xff.to_str() {
                    let new_xff = format!("{}, {}", xff_str, ctx.client_ip);
                    let _ = upstream_request.insert_header("X-Forwarded-For", new_xff);
                }
            } else {
                let _ = upstream_request.insert_header("X-Forwarded-For", &ctx.client_ip);
            }
        }
        Ok(())
    }
}

// #[tokio::main]
// async 
fn main() -> pingora::Result<()> {
    env_logger::init();

    let mut server = Server::new(None)?;
    server.bootstrap();

    let mut service = http_proxy_service(&server.configuration, MyProxy);
    
    // 配置 HTTPS 监听服务 (示例读取证书)
    // let cert_path = "/path/to/server.crt";
    // let key_path = "/path/to/server.key";
    // service.add_tls("0.0.0.0:443", cert_path, key_path).unwrap();

    // 临时测试绑定 8080 端口
    service.add_tcp("0.0.0.0:8080");

    server.add_service(service);
    server.run_forever();  
}

// use futures::future;
// use futures::select;

// #[tokio::main] 
// async fn main() -> anyhow::Result<()> {
//     let mut a_fut = future::ready(4);
//     let mut b_fut = future::ready(6);
//     let mut total = 0;

//     loop {
//         select! {
//             a = a_fut => {
//                 println!("total={},a={}",total,a);
//                 total += a
//             },
//             b = b_fut => {
//                 println!("total={},b={}",total,b);
//                 total += b
//             },
//             default => {
//                 println!("complete default={}",total);
//                 panic!()
//             }, // 该分支永远不会运行，因为 `Future` 会先运行，然后是 `complete`
//             complete => {
//                 println!("complete total={}",total);
//                 break
//             },
//         };
//     }
//     assert_eq!(total, 10);

//     Ok(())
// }
