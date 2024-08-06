use std::{collections::HashMap, fmt};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum HttpMethod {
    GET,
    POST,
    DELETE,
    UNSUPPORTED,
}

#[derive(Debug)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub host: Option<String>,
}

impl HttpRequest {
    pub fn parse(raw: &[u8]) -> Result<Self, String> {
        let request = String::from_utf8_lossy(raw);
        let lines: Vec<&str> = request.lines().collect();
        if lines.is_empty() {
            return Err("Empty request".to_string());
        }

        let first_line: Vec<&str> = lines[0].split_whitespace().collect();
        if first_line.len() != 3 {
            return Err("Invalid request line".to_string());
        }

        let method = match first_line[0] {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "DELETE" => HttpMethod::DELETE,
            _ => HttpMethod::UNSUPPORTED,
        };

        let path = first_line[1].to_string();
        let version = first_line[2].to_string();
        let mut headers = HashMap::new();
        let mut body_start = 0;
        let mut host = None;

        for (i, line) in lines[1..].iter().enumerate() {
            if line.is_empty() {
                body_start = i + 2;
                break;
            }
            let parts: Vec<&str> = line.splitn(2, ": ").collect();
            if parts.len() == 2 {
                let key = parts[0].to_lowercase();
                let value = parts[1].to_string();
                if key == "host" {
                    host = Some(value.clone());
                }
                headers.insert(key, value);
            }
        }

        let body = if body_start < lines.len() {
            lines[body_start..].join("\n").into_bytes()
        } else {
            Vec::new()
        };

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
            host,
        })
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::UNSUPPORTED => write!(f, "UNSUPPORTED"),
        }
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: ResponseBody,
}

#[derive(Debug)]
pub enum ResponseBody {
    Whole(Vec<u8>),
    Chunked(Vec<Vec<u8>>),
}

impl HttpResponse {
    pub fn new(status: u16, body: Vec<u8>, content_type: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.to_string());
        headers.insert("Content-Length".to_string(), body.len().to_string());
        HttpResponse {
            status,
            headers,
            body: ResponseBody::Whole(body),
        }
    }

    pub fn new_chunked(status: u16, chunks: Vec<Vec<u8>>, content_type: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.to_string());
        HttpResponse {
            status,
            headers,
            body: ResponseBody::Chunked(chunks),
        }
    }

}