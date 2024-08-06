use crate::config::ServerConfig;
use crate::static_file::StaticFile;
use std::io;

pub struct ErrorHandler {
    error_pages: std::collections::HashMap<u16, String>,
}

impl ErrorHandler {
    pub fn new(config: &ServerConfig) -> Self {
        ErrorHandler {
            error_pages: config.error_pages.clone(),
        }
    }

    pub fn get_error_content(&self, status_code: u16) -> io::Result<(Vec<u8>, String)> {
        println!("status_code = {} ", status_code);
        if let Some(error_page_path) = self.error_pages.get(&status_code) {
            match StaticFile::new(error_page_path) {
                Ok(file) => Ok((file.content, file.content_type)),
                Err(_) => self.default_error_content(status_code),
            }
        } else {
            println!("couldnt get error page");
            self.default_error_content(status_code)
        }
    }

    fn default_error_content(&self, status_code: u16) -> io::Result<(Vec<u8>, String)> {
        let content = format!("{} {}", status_code, self.status_message(status_code));
        Ok((content.into_bytes(), "text/plain".to_string()))
    }

    fn status_message(&self, status_code: u16) -> &'static str {
        match status_code {
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            _ => "Unknown Error",
        }
    }
}