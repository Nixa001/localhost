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
    pub chunked: bool,
    pub chunks: Vec<Vec<u8>>,
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
        let mut chunked = false;

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
                } else if key == "transfer-encoding" && value.to_lowercase() == "chunked" {
                    chunked = true;
                }
                headers.insert(key, value);
            }
        }

        let mut body = if body_start < lines.len() {
            lines[body_start..].join("\n").into_bytes()
        } else {
            Vec::new()
        };

        let mut chunks = Vec::new();
        if chunked {
            chunks = Self::parse_chunks(&body)?;
            body = chunks.concat();
        }

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
            host,
            chunked,
            chunks,
        })
    }

    fn parse_chunks(body: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        let mut chunks = Vec::new();
        let mut remaining = body;

        while !remaining.is_empty() {
            let mut chunk_size_end = 0;
            for (i, &byte) in remaining.iter().enumerate() {
                if byte == b'\r' && remaining.get(i + 1) == Some(&b'\n') {
                    chunk_size_end = i;
                    break;
                }
            }

            let chunk_size = String::from_utf8_lossy(&remaining[..chunk_size_end]);
            let chunk_size = usize::from_str_radix(&chunk_size, 16)
                .map_err(|_| "Invalid chunk size".to_string())?;

            if chunk_size == 0 {
                break;
            }

            let chunk_start = chunk_size_end + 2;
            let chunk_end = chunk_start + chunk_size;

            if remaining.len() < chunk_end + 2 {
                return Err("Incomplete chunk".to_string());
            }

            chunks.push(remaining[chunk_start..chunk_end].to_vec());
            remaining = &remaining[chunk_end + 2..];
        }

        Ok(chunks)
    }

    pub fn append_chunk(&mut self, data: &[u8]) -> Result<(), String> {
        if !self.chunked {
            return Err("Request is not chunked".to_string());
        }

        let mut remaining = data;
        while !remaining.is_empty() {
            let mut chunk_size_end = 0;
            for (i, &byte) in remaining.iter().enumerate() {
                if byte == b'\r' && remaining.get(i + 1) == Some(&b'\n') {
                    chunk_size_end = i;
                    break;
                }
            }

            let chunk_size = String::from_utf8_lossy(&remaining[..chunk_size_end]);
            let chunk_size = usize::from_str_radix(&chunk_size, 16)
                .map_err(|_| "Invalid chunk size".to_string())?;

            if chunk_size == 0 {
                self.chunks.push(Vec::new());
                break;
            }

            let chunk_start = chunk_size_end + 2;
            let chunk_end = chunk_start + chunk_size;

            if remaining.len() < chunk_end + 2 {
                return Err("Incomplete chunk".to_string());
            }

            self.chunks.push(remaining[chunk_start..chunk_end].to_vec());
            remaining = &remaining[chunk_end + 2..];
        }

        Ok(())
    }

    pub fn is_chunked_request_complete(&self) -> bool {
        if !self.chunked {
            return true;
        }

        self.chunks
            .last()
            .map_or(false, |last_chunk| last_chunk.is_empty())
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
