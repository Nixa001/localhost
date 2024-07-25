use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::server::Server;
use super::connection::Connection;
use super::config::{TIMEOUT, get_requested_path, DEFAULT_PAGE, ERROR_403_PAGE, ERROR_404_PAGE, ERROR_500_PAGE};
use crate::utils::file_ops::{generate_response, read_file};

pub struct MultiServer {
    pub servers: Vec<Server>,
    pub next_id: usize,
}

impl MultiServer {
    pub fn new(addrs: &[&str]) -> std::io::Result<Self> {
        let mut servers = Vec::new();
        for &addr in addrs {
            servers.push(Server::new(addr)?);
        }
        Ok(MultiServer {
            servers,
            next_id: 0,
        })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut ready_to_read = HashSet::new();
        let mut ready_to_write = HashSet::new();

        loop {
            // Check for new connections and add them to our sets
            for (server_idx, server) in self.servers.iter_mut().enumerate() {
                match server.listener.accept() {
                    Ok((stream, addr)) => {
                        println!("New connection: {}", addr);
                        stream.set_nonblocking(true)?;
                        let id = self.next_id;
                        self.next_id += 1;
                        let connection = Connection::new(stream);
                        server.connections.insert(id, connection);
                        ready_to_read.insert((server_idx, id));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
            }

            // Process ready connections
            let mut to_remove = Vec::new();
            for &(server_idx, id) in &ready_to_read {
                if let Some(conn) = self.servers[server_idx].connections.get_mut(&id) {
                    let mut buffer = [0; 1024];
                    match conn.read(&mut buffer) {
                        Ok(0) => {
                            to_remove.push((server_idx, id));
                        }
                        Ok(n) => {
                            conn.buffer.extend_from_slice(&buffer[..n]);
                            conn.last_activity = Instant::now();
                            if Self::is_request_complete(&conn.buffer) {
                                let response = Self::process_request(&conn.buffer);
                                conn.response = Some(response);
                                conn.buffer.clear();
                                ready_to_write.insert((server_idx, id));
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => {
                            to_remove.push((server_idx, id));
                        }
                    }
                }
            }

            for &(server_idx, id) in &ready_to_write {
                if let Some(conn) = self.servers[server_idx].connections.get_mut(&id) {
                    if let Some(response) = conn.response.take() {
                        match conn.write(&response) {
                            Ok(_) => {
                                conn.last_activity = Instant::now();
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                conn.response = Some(response);
                            }
                            Err(_) => {
                                to_remove.push((server_idx, id));
                            }
                        }
                    }
                }
            }

            // Remove closed or timed out connections
            for (server_idx, id) in to_remove {
                if let Some(conn) = self.servers[server_idx].connections.remove(&id) {
                    println!("Connection closed: {:?}", conn.stream.peer_addr());
                }
                ready_to_read.remove(&(server_idx, id));
                ready_to_write.remove(&(server_idx, id));
            }

            // Check for timeouts
            let now = Instant::now();
            for (server_idx, server) in self.servers.iter_mut().enumerate() {
                server.connections.retain(|&id, conn| {
                    if now.duration_since(conn.last_activity) > TIMEOUT {
                        println!("Connection timed out: {:?}", conn.stream.peer_addr());
                        ready_to_read.remove(&(server_idx, id));
                        ready_to_write.remove(&(server_idx, id));
                        false
                    } else {
                        true
                    }
                });
            }

            // Simulate epoll_wait
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn is_request_complete(buffer: &[u8]) -> bool {
        buffer.windows(4).any(|window| window == b"\r\n\r\n")
    }

    fn process_request(buffer: &[u8]) -> Vec<u8> {
        let request = String::from_utf8_lossy(buffer);
        let path = get_requested_path(&request);
        let file_path = if path.is_empty() {
            DEFAULT_PAGE.to_string()
        } else {
            format!("src/www/{}", path)
        };

        match read_file(&file_path) {
            Ok(contents) => generate_response("200 OK", "text/html", &contents),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => match read_file(ERROR_404_PAGE) {
                    Ok(contents) => generate_response("404 NOT FOUND", "text/html", &contents),
                    Err(_) => generate_response("404 NOT FOUND", "text/plain", "404 Not Found"),
                },
                std::io::ErrorKind::PermissionDenied => match read_file(ERROR_403_PAGE) {
                    Ok(contents) => generate_response("403 FORBIDDEN", "text/html", &contents),
                    Err(_) => generate_response("403 FORBIDDEN", "text/plain", "403 Forbidden"),
                },
                _ => match read_file(ERROR_500_PAGE) {
                    Ok(contents) => generate_response("500 INTERNAL SERVER ERROR", "text/html", &contents),
                    Err(_) => generate_response("500 INTERNAL SERVER ERROR", "text/plain", "500 Internal Server Error"),
                },
            },
        }
    }
}
