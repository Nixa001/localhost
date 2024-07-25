// src/server/config.rs
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_PAGE: &str = "src/www/index.html";
pub const ERROR_403_PAGE: &str = "src/www/errors/403.html";
// pub const ERROR_400_PAGE: &str = "src/www/errors/400.html";
pub const ERROR_404_PAGE: &str = "src/www/errors/404.html";
// pub const ERROR_405_PAGE: &str = "src/www/errors/405.html";
// pub const ERROR_413_PAGE: &str = "src/www/errors/413.html";
pub const ERROR_500_PAGE: &str = "src/www/errors/500.html";

pub fn get_requested_path(request: &str) -> String {
    let lines: Vec<&str> = request.lines().collect();
    if let Some(first_line) = lines.first() {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() > 1 {
            return parts[1].trim_start_matches('/').to_string();
        }
    }
    String::new()
}
