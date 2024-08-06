use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

pub struct Logger {
    file: Mutex<std::fs::File>,
}

impl Logger {
    pub fn new(log_file: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        Ok(Logger {
            file: Mutex::new(file),
        })
    }

    pub fn log(&self, level: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let log_message = format!("[{}] {} - {}\n", timestamp, level, message);
        
        if let Ok(mut file) = self.file.lock() {
            if let Err(e) = file.write_all(log_message.as_bytes()) {
                eprintln!("Failed to write to log file: {}", e);
            }
        }
    }

    pub fn info(&self, message: &str) {
        self.log("INFO", message);
    }

    pub fn warn(&self, message: &str) {
        self.log("WARN", message);
    }

    pub fn error(&self, message: &str) {
        self.log("ERROR", message);
    }
}