// 1. 宣告擁有這兩個子檔案模組
pub mod http;
pub mod tcp;

// 2. 🌟 舉一反三的優雅優化：將常用的結構體重出口（Re-export）
// 這樣 main.rs 就可以直接引入 protocol::HttpCodec，不用寫 protocol::http::HttpCodec
pub use http::HttpCodec;
pub use tcp::TcpCodec;