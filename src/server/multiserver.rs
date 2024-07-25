use std::collections::HashSet;
use std::time::{Duration, Instant};

use super::config::{
    get_requested_path, DEFAULT_PAGE, ERROR_403_PAGE, ERROR_404_PAGE, ERROR_500_PAGE, TIMEOUT,
};
use super::connection::Connection;
use super::server::{Server, VirtualHost};
use crate::utils::file_ops::{generate_response, read_file};

pub struct MultiServer {
    servers: Vec<Server>,
    next_id: usize,
}

impl MultiServer {
    pub fn new(configs: &[(&str, Vec<(&str, &str)>)]) -> std::io::Result<Self> {
        let mut servers = Vec::new();
        for &(addr, ref vhosts) in configs {
            let virtual_hosts = vhosts
                .iter()
                .map(|&(hostname, root_dir)| VirtualHost {
                    hostname: hostname.to_string(),
                    root_directory: root_dir.to_string(),
                })
                .collect();
            servers.push(Server::new(addr, virtual_hosts)?);
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
                                let response =
                                    Self::process_request(self.servers[server_idx], &conn.buffer);
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

    fn process_request(server: &Server, buffer: &[u8]) -> Vec<u8> {
        let request = String::from_utf8_lossy(buffer);
        let hostname = Self::get_hostname(&request);
        let path = Self::get_requested_path(&request);

        // Trouver le virtual host correspondant
        let vhost = server
            .virtual_hosts
            .iter()
            .find(|vh| vh.hostname == hostname);

        let file_path = if let Some(vh) = vhost {
            if path.is_empty() {
                format!("{}/index.html", vh.root_directory)
            } else {
                format!("{}/{}", vh.root_directory, path)
            }
        } else {
            // Utiliser un hôte par défaut ou renvoyer une erreur
            "src/www/error/404.html".to_string()
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
                    Ok(contents) => {
                        generate_response("500 INTERNAL SERVER ERROR", "text/html", &contents)
                    }
                    Err(_) => generate_response(
                        "500 INTERNAL SERVER ERROR",
                        "text/plain",
                        "500 Internal Server Error",
                    ),
                },
            },
        }
    }
    fn get_requested_path(request: &str) -> String {
        let lines: Vec<&str> = request.lines().collect();
        if let Some(first_line) = lines.first() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() > 1 {
                return parts[1].trim_start_matches('/').to_string();
            }
        }
        String::new()
    }
    fn get_hostname(request: &str) -> String {
        for line in request.lines() {
            if line.to_lowercase().starts_with("host:") {
                return line.split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }
        String::new()
    }
}
