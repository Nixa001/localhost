use crate::config::ServerConfig;

pub struct ErrorHandler {
    error_pages: std::collections::HashMap<u16, String>,
}

impl ErrorHandler {
    pub fn new(config: &ServerConfig) -> Self {
        ErrorHandler {
            error_pages: config.error_pages.clone(),
        }
    }
}
