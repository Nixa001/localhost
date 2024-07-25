use std::fs;

pub fn read_file(path: &str) -> std::io::Result<String> {
    fs::read_to_string(path)
}

pub fn generate_response(status: &str, content_type: &str, content: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        content_type,
        content.len(),
        content
    )
    .into_bytes()
}
