use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
// use std::path::Path;

const TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_PAGE: &str = "src/www/index.html";
const ERROR_403_PAGE: &str = "src/www/errors/403.html";
const ERROR_400_PAGE: &str = "src/www/errors/400.html";
const ERROR_404_PAGE: &str = "src/www/errors/404.html";
const ERROR_405_PAGE: &str = "src/www/errors/405.html";
const ERROR_413_PAGE: &str = "src/www/errors/413.html";
const ERROR_500_PAGE: &str = "src/www/errors/500.html";

struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    response: Option<Vec<u8>>,
    last_activity: Instant,
}

struct Server {
    listener: TcpListener,
    connections: HashMap<usize, Connection>,
}

struct MultiServer {
    servers: Vec<Server>,
    next_id: usize,
}

impl Server {
    fn new(addr: &str) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Server {
            listener,
            connections: HashMap::new(),
        })
    }
}
fn read_file(path: &str) -> std::io::Result<String> {
    fs::read_to_string(path)
}

fn generate_response(status: &str, content_type: &str, content: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        content_type,
        content.len(),
        content
    )
    .into_bytes()
}
impl MultiServer {
    fn new(addrs: &[&str]) -> std::io::Result<Self> {
        let mut servers = Vec::new();
        for &addr in addrs {
            servers.push(Server::new(addr)?);
        }
        Ok(MultiServer {
            servers,
            next_id: 0,
        })
    }

    fn run(&mut self) -> std::io::Result<()> {
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
                        let connection = Connection {
                            stream,
                            buffer: Vec::new(),
                            response: None,
                            last_activity: Instant::now(),
                        };
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
                    match conn.stream.read(&mut buffer) {
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
                        match conn.stream.write_all(&response) {
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
        let path = Self::get_requested_path(&request);
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
}

fn main() -> std::io::Result<()> {
    let addrs = ["127.0.0.1:8080", "127.0.0.1:8081"];
    let mut multi_server = MultiServer::new(&addrs)?;
    multi_server.run()
}
