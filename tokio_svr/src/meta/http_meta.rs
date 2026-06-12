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
