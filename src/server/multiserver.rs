use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};

use super::config::{ERROR_403_PAGE, ERROR_404_PAGE, ERROR_405_PAGE, ERROR_500_PAGE, TIMEOUT};
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

            let mut to_remove = Vec::new();
            let mut to_process = Vec::new();

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
                                to_process.push((server_idx, id));
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(_) => {
                            to_remove.push((server_idx, id));
                        }
                    }
                }
            }
            for (server_idx, id) in to_process {
                let request_buffer =
                    if let Some(conn) = self.servers[server_idx].connections.get(&id) {
                        conn.buffer.clone()
                    } else {
                        continue;
                    };

                let response = Self::process_request(&self.servers[server_idx], &request_buffer);

                if let Some(conn) = self.servers[server_idx].connections.get_mut(&id) {
                    conn.response = Some(response);
                    conn.buffer.clear();
                    ready_to_write.insert((server_idx, id));
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
        let method = Self::get_request_method(&request);

        println!("Requested hostname: {}", hostname);
        println!("Requested path: {}", path);
        println!("Request method: {}", method);

        // Check if the method is allowed
        if !["GET", "POST", "DELETE"].contains(&&method[..]) {
            println!("Method not allowed: {}", method);
            return match read_file(ERROR_405_PAGE) {
                Ok(contents) => generate_response("405 METHOD NOT ALLOWED", "text/html", &contents),
                Err(_) => generate_response(
                    "405 METHOD NOT ALLOWED",
                    "text/plain",
                    "405 Method Not Allowed",
                ),
            };
        }

        // Handle file upload for POST requests
        if method == "POST" && path == "upload" {
            return Self::handle_file_upload(server, &request);
        }

        // Find the corresponding virtual host
        let vhost = server
            .virtual_hosts
            .iter()
            .find(|vh| vh.hostname == hostname);

        println!("Matching vhost: {:?}", vhost.map(|vh| &vh.hostname));

        let file_path = if let Some(vh) = vhost {
            if path.is_empty() {
                let fp = format!("{}/index.html", vh.root_directory);
                println!("Attempting to serve default page: {}", fp);
                fp
            } else {
                let fp = format!("{}/{}", vh.root_directory, path);
                println!("Attempting to serve: {}", fp);
                fp
            }
        } else {
            println!("No matching virtual host found, serving 404 page");
            ERROR_404_PAGE.to_string()
        };

        match read_file(&file_path) {
            Ok(contents) => {
                println!("Successfully read file: {}", file_path);
                generate_response("200 OK", "text/html", &contents)
            }
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

    fn handle_file_upload(server: &Server, request: &str) -> Vec<u8> {
        let boundary = Self::get_boundary(request);
        let parts = request.split(&boundary).collect::<Vec<&str>>();

        for part in parts.iter().skip(1) {
            if part.contains("filename=") {
                let filename = Self::extract_filename(part);
                let content = Self::extract_content(part);

                let upload_dir = if server.listener.local_addr().unwrap().port() == 8080 {
                    "src/www/upload/server1"
                } else {
                    "src/www/upload/server2"
                };

                if let Err(e) = fs::create_dir_all(upload_dir) {
                    eprintln!("Failed to create upload directory: {}", e);
                    return generate_response(
                        "500 INTERNAL SERVER ERROR",
                        "text/plain",
                        "Failed to create upload directory",
                    );
                }

                let file_path = format!("{}/{}", upload_dir, filename);
                match fs::File::create(&file_path) {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(content.as_bytes()) {
                            eprintln!("Failed to write file: {}", e);
                            return generate_response(
                                "500 INTERNAL SERVER ERROR",
                                "text/plain",
                                "Failed to write file",
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to create file: {}", e);
                        return generate_response(
                            "500 INTERNAL SERVER ERROR",
                            "text/plain",
                            "Failed to create file",
                        );
                    }
                }

                return generate_response("200 OK", "text/plain", "File uploaded successfully");
            }
        }

        generate_response(
            "400 BAD REQUEST",
            "text/plain",
            "No file found in the request",
        )
    }

    fn get_boundary(request: &str) -> String {
        for line in request.lines() {
            if line.starts_with("Content-Type: multipart/form-data; boundary=") {
                return line.split("boundary=").last().unwrap_or("").to_string();
            }
        }
        String::new()
    }

    fn extract_filename(part: &str) -> String {
        for line in part.lines() {
            if line.contains("filename=") {
                return line
                    .split("filename=")
                    .last()
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
            }
        }
        String::new()
    }

    fn extract_content(part: &str) -> String {
        let content_start = part.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
        let content_end = part.rfind("\r\n").unwrap_or(part.len());
        part[content_start..content_end].to_string()
    }

    fn get_request_method(request: &str) -> String {
        let lines: Vec<&str> = request.lines().collect();
        if let Some(first_line) = lines.first() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if !parts.is_empty() {
                return parts[0].to_uppercase();
            }
        }
        String::new()
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
