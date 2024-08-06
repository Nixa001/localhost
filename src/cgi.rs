use std::process::Command;
use std::io;
use crate::http::{HttpRequest, HttpResponse};

const CHUNKED_THRESHOLD: usize = 1_000_000; // 1MB

pub struct CGIHandler {
    php_path: String,
    python_path: String,
}

impl CGIHandler {
    pub fn new(php_path: String, python_path: String) -> Self {
        CGIHandler { php_path, python_path }
    }

    pub fn handle(&self, _request: &HttpRequest, script_path: &str) -> io::Result<HttpResponse> {
        let (_interpreter, interpreter_path) = if script_path.ends_with(".php") {
            ("php", &self.php_path)
        } else if script_path.ends_with(".py") {
            ("python", &self.python_path)
        } else {
            return Ok(HttpResponse::new(400, b"Unsupported CGI script type".to_vec(), "text/plain"));
        };

        let output = Command::new(interpreter_path)
            .arg(script_path)
            .output()?;

        if output.status.success() {
            let content_type = "text/html"; // Par défaut, on suppose que le CGI renvoie du HTML
            
            if output.stdout.len() > CHUNKED_THRESHOLD {
                // Réponse chunked pour les grandes sorties
                let chunks = self.create_chunks(&output.stdout);
                Ok(HttpResponse::new_chunked(200, chunks, content_type))
            } else {
                // Réponse non-chunked pour les petites sorties
                Ok(HttpResponse::new(200, output.stdout, content_type))
            }
        } else {
            Ok(HttpResponse::new(500, output.stderr, "text/plain"))
        }
    }

    fn create_chunks(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let chunk_size = 8192; // 8KB chunks
        data.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect()
    }
}