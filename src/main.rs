use crate::fs::File;
use httparse;
use multipart::server::Multipart;
use std::fs::{self};
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::thread;

#[derive(Clone, Debug)]
struct Config {
    servers: Vec<ServerConfig>,
    error_pages: Vec<String>,
    max_body_size: usize,
    routes: Vec<Route>,
}

#[derive(Clone, Debug)]
struct ServerConfig {
    addr: SocketAddr,
}

#[derive(Clone, Debug)]
struct Route {
    path: String,
    methods: Vec<String>,
    redirection: Option<String>,
    directory: String,
    default_file: String,
    cgi: Option<String>,
    directory_listing: bool,
}

impl Config {
    fn load_from_file(file_path: &str) -> std::io::Result<Self> {
        let content = fs::read_to_string(file_path)?;
        let mut config = Config {
            servers: Vec::new(),
            error_pages: Vec::new(),
            max_body_size: 0,
            routes: Vec::new(),
        };

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            match parts[0] {
                "server" => {
                    if let Ok(addr) = parts[1].parse() {
                        config.servers.push(ServerConfig { addr });
                    }
                }
                "error_page" => config.error_pages.push(parts[1].to_string()),
                "max_body_size" => {
                    if let Ok(size) = parts[1].parse() {
                        config.max_body_size = size;
                    }
                }
                "route" => {
                    let route_parts: Vec<&str> = parts[1].split(',').collect();
                    if route_parts.len() == 7 {
                        let route = Route {
                            path: route_parts[0].to_string(),
                            methods: route_parts[1].split('|').map(String::from).collect(),
                            redirection: if route_parts[2].is_empty() {
                                None
                            } else {
                                Some(route_parts[2].to_string())
                            },
                            directory: route_parts[3].to_string(),
                            default_file: route_parts[4].to_string(),
                            cgi: if route_parts[5].is_empty() {
                                None
                            } else {
                                Some(route_parts[5].to_string())
                            },
                            directory_listing: route_parts[6].parse().unwrap_or(false),
                        };
                        config.routes.push(route);
                    }
                }
                _ => {}
            }
        }

        Ok(config)
    }
}

fn main() -> io::Result<()> {
    println!("Starting the server...");

    let config = match Config::load_from_file("src/server_config.toml") {
        Ok(config) => {
            println!("Configuration loaded successfully.");
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("Current directory: {:?}", std::env::current_dir()?);
            return Err(e);
        }
    };

    if config.servers.is_empty() {
        eprintln!("No server configurations found. Check your server_config.txt file.");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No server configurations",
        ));
    }

    println!("Loaded {} server configurations", config.servers.len());

    let config = Arc::new(config);
    let mut handles = vec![];

    for (index, server_config) in config.servers.iter().enumerate() {
        match TcpListener::bind(server_config.addr) {
            Ok(listener) => {
                println!("Server {} listening on: {}", index, server_config.addr);
                let config = config.clone();

                let handle = thread::spawn(move || {
                    for stream in listener.incoming() {
                        match stream {
                            Ok(stream) => {
                                let config = config.clone();
                                thread::spawn(move || {
                                    if let Err(e) = handle_client(stream, &config) {
                                        eprintln!("Error handling client: {}", e);
                                    }
                                });
                            }
                            Err(e) => eprintln!("Error accepting connection: {}", e),
                        }
                    }
                });

                handles.push(handle);
            }
            Err(e) => {
                eprintln!(
                    "Failed to bind server {} to {}: {}",
                    index, server_config.addr, e
                );
            }
        }
    }

    if handles.is_empty() {
        eprintln!("No servers were successfully started.");
        return Err(io::Error::new(ErrorKind::Other, "No servers started"));
    }

    println!("All servers started successfully. Press Ctrl+C to stop.");

    for handle in handles {
        if let Err(e) = handle.join() {
            eprintln!("Error in server thread: {:?}", e);
        }
    }

    println!("All server threads have finished. Exiting.");
    Ok(())
}

fn handle_client(mut stream: TcpStream, config: &Config) -> std::io::Result<()> {
    let mut buffer = [0; 1024];
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    let n = stream.read(&mut buffer)?;
    let result = req
        .parse(&buffer[..n])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if result.is_partial() {
        return Ok(()); // Need more data, but we'll just ignore for simplicity
    }

    let method = req.method.unwrap();
    let path = req.path.unwrap();

    match method {
        "GET" => handle_get(&mut stream, path, config),
        "POST" => handle_post(&mut stream, path, &buffer[..n], config),
        "DELETE" => handle_delete(&mut stream, path, config),
        _ => send_error_page(&mut stream, 405, config),
    }
}

fn handle_get(stream: &mut TcpStream, path: &str, config: &Config) -> std::io::Result<()> {
    if let Some(route) = config.routes.iter().find(|r| path.starts_with(&r.path)) {
        if !route.methods.contains(&"GET".to_string()) {
            return send_error_page(stream, 405, config);
        }

        if let Some(ref redirection) = route.redirection {
            let response = format!(
                "HTTP/1.1 301 Moved Permanently\r\nLocation: {}\r\n\r\n",
                redirection
            );
            stream.write_all(response.as_bytes())?;
            return Ok(());
        }

        // Déterminez le répertoire basé sur l'adresse locale du serveur
        let local_addr = stream.local_addr()?;
        let upload_dir = match local_addr.port() {
            8080 => "src/www/upload/server1",
            8081 => "src/www/upload/server2",
            _ => return send_error_page(stream, 500, config),
        };

        // Construisez le chemin du fichier
        let file_path = if path == "/" || path.is_empty() {
            format!("{}/{}", upload_dir, route.default_file)
        } else {
            format!("{}{}", upload_dir, path)
        };

        if Path::new(&file_path).exists() {
            let metadata = fs::metadata(&file_path)?;
            if metadata.is_dir() && route.directory_listing {
                // Listing de répertoire
                print!("{}", file_path);
                let entries = fs::read_dir(&file_path)?;
                let mut listing = String::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        listing.push_str(&format!("{}\n", entry.file_name().to_string_lossy()));
                    }
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    listing.len(),
                    listing
                );
                stream.write_all(response.as_bytes())?;
            } else if metadata.is_file() {
                // Servir le fichier
                let content = fs::read(&file_path)?;
                let content_type = get_content_type(&file_path);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                    content_type,
                    content.len()
                );
                stream.write_all(response.as_bytes())?;
                stream.write_all(&content)?;
            } else {
                return send_error_page(stream, 403, config); // Forbidden
            }
        } else {
            return send_error_page(stream, 404, config);
        }
    } else {
        return send_error_page(stream, 404, config);
    }
    Ok(())
}

fn get_content_type(file_path: &str) -> &'static str {
    match Path::new(file_path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
    {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

fn handle_post(
    stream: &mut TcpStream,
    path: &str,
    body: &[u8],
    config: &Config,
) -> std::io::Result<()> {
    if path == "/upload" {
        let boundary = get_boundary(stream)?;
        let mut multipart = Multipart::with_body(body, boundary);

        let upload_dir = match stream.local_addr()?.port() {
            8080 => "src/www/upload/server1",
            8081 => "src/www/upload/server2",
            _ => return send_error_page(stream, 500, config),
        };

        std::fs::create_dir_all(upload_dir)?;

        while let Some(mut field) = multipart.read_entry()? {
            if let Some(file_name) = field.headers.filename {
                let file_path = Path::new(upload_dir).join(file_name);
                let mut file = File::create(&file_path)?;
                io::copy(&mut field.data, &mut file)?;
            }
        }

        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK";
        stream.write_all(response.as_bytes())?;
    } else {
        return send_error_page(stream, 404, config);
    }
    Ok(())
}
fn get_boundary(mut stream: &TcpStream) -> io::Result<String> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    let mut buffer = [0; 1024];

    let n = stream.read(&mut buffer)?;
    req.parse(&buffer[..n])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    for header in req.headers {
        if header.name.to_lowercase() == "content-type" {
            let value = std::str::from_utf8(header.value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            if let Some(boundary) = value.split("boundary=").nth(1) {
                return Ok(boundary.trim_matches('"').to_string());
            }
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Boundary not found",
    ))
}

fn handle_delete(stream: &mut TcpStream, path: &str, config: &Config) -> std::io::Result<()> {
    if path.starts_with("/delete/") {
        let file_name = path.trim_start_matches("/delete/");
        let upload_dir = match stream.local_addr()?.port() {
            8080 => "src/www/upload/server1",
            8081 => "src/www/upload/server2",
            _ => return send_error_page(stream, 500, config),
        };

        let file_path = Path::new(upload_dir).join(file_name);
        if file_path.exists() && file_path.is_file() {
            fs::remove_file(file_path)?;
            let response =
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK";
            stream.write_all(response.as_bytes())?;
        } else {
            return send_error_page(stream, 404, config);
        }
    } else {
        return send_error_page(stream, 404, config);
    }
    Ok(())
}

fn send_error_page(
    stream: &mut TcpStream,
    status_code: usize,
    config: &Config,
) -> std::io::Result<()> {
    let status_line = match status_code {
        404 => "HTTP/1.1 404 Not Found",
        405 => "HTTP/1.1 405 Method Not Allowed",
        413 => "HTTP/1.1 413 Payload Too Large",
        500 => "HTTP/1.1 500 Internal Server Error",
        _ => "HTTP/1.1 400 Bad Request",
    };

    let error_page = config
        .error_pages
        .get(status_code)
        .cloned()
        .unwrap_or_else(|| format!("<h1>{}</h1>", status_line));

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        error_page.len(),
        error_page
    );

    stream.write_all(response.as_bytes())
}

fn execute_cgi(cgi_path: &str, query_string: &str) -> std::io::Result<String> {
    let output = Command::new(cgi_path).arg(query_string).output()?.stdout;
    Ok(String::from_utf8_lossy(&output).into_owned())
}
