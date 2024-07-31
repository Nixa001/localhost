pub struct HttpResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status_code: u16, content_type: String, body: String) -> Self {
        HttpResponse {
            status_code,
            content_type,
            body,
        }
    }

    pub fn to_string(&self) -> String {
        format!(
            "HTTP/1.1 {} OK\r\nContent-Type: {}\r\n\r\n{}",
            self.status_code, self.content_type, self.body
        )
    }
}

pub fn handle_request(_request: &[u8], server_id: usize, port: u16) -> HttpResponse {
    let body = format!(
        "<html><body><h1>Hello from server {} port {}</h1></body></html>",
        server_id, port
    );

    HttpResponse::new(200, "text/html".to_string(), body)
}
