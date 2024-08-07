use crate::cgi::CGIHandler;
use crate::config::ServerConfig;
use crate::error::ServerError;
use crate::error_handler::ErrorHandler;
use crate::http::{HttpMethod, HttpRequest, HttpResponse, ResponseBody};
use crate::logger;
use crate::router::{RouteMatch, RouteMatchResult, Router};
use crate::session::SessionManager;
use crate::static_file::StaticFile;
use chrono::{DateTime, Local};
use logger::Logger;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::net::TcpListener as StdTcpListener;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_EVENTS: usize = 1024 * 1024 * 10;

pub struct Server {
    configs: HashMap<String, ServerConfig>,
    poll: Poll,
    listeners: HashMap<Token, (TcpListener, String)>, // String est l'ID du serveur
    clients: HashMap<Token, (TcpStream, String)>,     // String est l'ID du serveur
    routers: HashMap<String, Router>,                 // String est l'ID du serveur
    next_token: usize,
    error_handler: ErrorHandler,
    cgi_handler: CGIHandler,
    session_manager: SessionManager,
    logger: Arc<Logger>,
    request_start_times: HashMap<Token, Instant>,
}

impl Server {
    pub fn new(configs: Vec<ServerConfig>, logger: Arc<Logger>) -> Result<Self, ServerError> {
        if configs.is_empty() {
            return Err(ServerError::Internal(
                "No valid server configurations found".to_string(),
            ));
        }
        let poll = Poll::new().map_err(|e| ServerError::Io(e))?;
        let mut listeners = HashMap::new();
        let mut routers = HashMap::new();
        let mut config_map = HashMap::new();

        for config in configs {
            for &port in &config.ports {
                let addr: SocketAddr = SocketAddr::new(config.host, port);
                match StdTcpListener::bind(addr) {
                    Ok(std_listener) => {
                        std_listener
                            .set_nonblocking(true)
                            .map_err(|e| ServerError::Io(e))?;
                        let mut listener = TcpListener::from_std(std_listener);
                        let token = Token(listeners.len());
                        poll.registry()
                            .register(&mut listener, token, Interest::READABLE)
                            .map_err(|e| ServerError::Io(e))?;
                        listeners.insert(token, (listener, config.id.clone()));
                        logger.info(&format!("Successfully bound to {}:{}", config.host, port));
                    }
                    Err(e) => {
                        logger.warn(&format!(
                            "Failed to bind to {}:{} - {}",
                            config.host, port, e
                        ));
                        continue;
                    }
                }
            }

            routers.insert(config.id.clone(), Router::new(config.routes.clone()));

            // Check root directories
            for route in &config.routes {
                let path = std::path::Path::new(&route.root);
                if !path.exists() {
                    std::fs::create_dir_all(path).map_err(|e| ServerError::Io(e))?;
                    logger.info(&format!("Created directory: {}", route.root));
                } else if !path.is_dir() {
                    return Err(ServerError::Internal(format!(
                        "Path is not a directory: {}",
                        route.root
                    )));
                }
            }

            config_map.insert(config.id.clone(), config);
        }

        if config_map.is_empty() {
            return Err(ServerError::Internal(
                "No valid server configurations after processing".to_string(),
            ));
        }

        let error_handler = ErrorHandler::new(&config_map.values().next().unwrap());
        let cgi_handler =
            CGIHandler::new("/usr/bin/php".to_string(), "/usr/bin/python3".to_string());
        let session_manager = SessionManager::new(std::time::Duration::from_secs(3600));
        let listeners_len = listeners.len();

        Ok(Server {
            configs: config_map,
            poll,
            listeners,
            clients: HashMap::new(),
            next_token: listeners_len + 1,
            routers,
            error_handler,
            cgi_handler,
            session_manager,
            logger,
            request_start_times: HashMap::new(),
        })
    }

    fn handle_directory_listing(
        &self,
        path: &Path,
        request_path: &str,
    ) -> Result<HttpResponse, ServerError> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_name = entry.file_name().into_string().unwrap_or_default();
            let file_type = entry.file_type()?;
            let metadata = entry.metadata()?;
            let size = metadata.len();
            let modified: DateTime<Local> = metadata.modified()?.into();
            entries.push((file_name, file_type.is_dir(), size, modified));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0)); // Tri alphabétique

        let mut html = String::from(
            r#"<!DOCTYPE html>
    <html>
    <head>
        <title>Directory Listing</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 0; padding: 20px; background-color: #f0f0f0; }
            h1 { color: #333; }
            table { width: 100%; border-collapse: collapse; margin-bottom: 20px; background-color: white; }
            th, td { padding: 10px; text-align: left; border-bottom: 1px solid #ddd; }
            th { background-color: #4CAF50; color: white; }
            tr:hover { background-color: #f5f5f5; }
            a { color: #1a73e8; text-decoration: none; }
            a:hover { text-decoration: underline; }
            .upload-form { background-color: white; padding: 20px; border-radius: 5px; }
            .upload-form input[type="file"] { margin-right: 10px; }
            .upload-form input[type="submit"], .delete-btn { 
                background-color: #4CAF50; 
                color: white; 
                border: none; 
                padding: 5px 10px; 
                cursor: pointer; 
                border-radius: 3px;
            }
            .delete-btn { background-color: #f44336; }
        </style>
    </head>
    <body>"#,
        );

        html.push_str(&format!("<h1>Directory Listing:  {}</h1>", request_path));
        html.push_str("<table><tr><th>Name</th><th>Type</th><th>Size</th><th>Last Modified</th><th>Actions</th></tr>");

        if request_path != "/" {
            let parent = Path::new(request_path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("/");
            html.push_str(&format!(
                "<tr><td colspan='5'><a href='{}'>..</a> (Parent Directory)</td></tr>",
                parent
            ));
        }

        for (name, is_dir, size, modified) in entries {
            let full_path = if request_path.ends_with('/') {
                format!("{}{}", request_path, name)
            } else {
                format!("{}/{}", request_path, name)
            };
            html.push_str("<tr>");
            html.push_str(&format!("<td><a href='{}'>{}</a></td>", full_path, name));
            html.push_str(&format!(
                "<td>{}</td>",
                if is_dir { "Directory" } else { "File" }
            ));
            html.push_str(&format!(
                "<td>{}</td>",
                if is_dir {
                    "-".to_string()
                } else {
                    human_readable_size(size)
                }
            ));
            html.push_str(&format!(
                "<td>{}</td>",
                modified.format("%Y-%m-%d %H:%M:%S")
            ));
            if !is_dir {
                html.push_str(&format!(
                    r#"<td>
                    <form action="{}" method="post" style="display:inline;">
                        <input type="hidden" name="_method" value="DELETE" />
                        <input type="submit" value="Delete" class="delete-btn" />
                    </form>
                </td>"#,
                    full_path
                ));
            } else {
                html.push_str("<td></td>");
            }
            html.push_str("</tr>");
        }

        html.push_str("</table>");

        // Ajouter le formulaire d'upload en bas du tableau
        html.push_str(&format!(
            r#"<div class="upload-form">
                <h2>Upload File</h2>
                <form action="{}" method="post" enctype="multipart/form-data">
                    <input type="file" name="file" required/>
                    <input type="submit" value="Upload"  />
                </form>
            </div>"#,
            request_path
        ));

        html.push_str("</body></html>");
        Ok(HttpResponse::new(200, html.into_bytes(), "text/html"))
    }
    pub fn run(&mut self) -> Result<(), ServerError> {
        let mut events = Events::with_capacity(MAX_EVENTS);

        self.logger.info("Server starting...");

        loop {
            match self.poll.poll(&mut events, None) {
                Ok(_) => {
                    for event in events.iter() {
                        self.logger.info(&format!("Received event: {:?}", event));
                        match event.token() {
                            token if self.listeners.contains_key(&token) => {
                                self.logger.info(&format!(
                                    "Received listener event for token {:?}",
                                    token
                                ));
                                if let Err(e) = self.accept_connection(token) {
                                    self.logger
                                        .error(&format!("Error accepting connection: {}", e));
                                }
                            }
                            token => {
                                self.logger
                                    .info(&format!("Received client token event: {:?}", token));
                                match self.handle_client(token) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        self.logger.error(&format!("Error handling client: {}", e));
                                        if let Err(remove_err) = self.remove_client(token) {
                                            self.logger.error(&format!(
                                                "Error removing client: {}",
                                                remove_err
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    self.logger.error(&format!("Error polling events: {}", e));
                    return Err(ServerError::from(e));
                }
            }
        }
    }

    fn accept_connection(&mut self, token: Token) -> io::Result<()> {
        if let Some((listener, config)) = self.listeners.get_mut(&token) {
            loop {
                match listener.accept() {
                    Ok((mut stream, addr)) => {
                        self.logger
                            .info(&format!("New connection accepted from: {}", addr));
                        let client_token = Token(self.next_token);
                        self.next_token += 1;
                        self.poll.registry().register(
                            &mut stream,
                            client_token,
                            Interest::READABLE,
                        )?;
                        self.clients.insert(client_token, (stream, config.clone()));
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => {
                        self.logger
                            .error(&format!("Error accepting connection: {}", e));
                        return Err(e);
                    }
                }
            }
        } else {
            self.logger
                .error(&format!("No listener found for token {:?}", token));
        }
        self.logger.info("Exiting accept_connection");
        Ok(())
    }

    fn handle_request(
        &mut self,
        request: &HttpRequest,
        server_id: &str,
    ) -> Result<HttpResponse, ServerError> {
        self.logger.info(&format!(
            "Handling {:?} request for {:?} on server {}",
            request.method, request.path, server_id
        ));
        // Vérifier si la méthode est autorisée
        match request.method {
            HttpMethod::GET | HttpMethod::POST | HttpMethod::DELETE => {}
            _ => return Ok(self.create_error_response(405)), // Method Not Allowed
        }

        if request.path == "/login" {
            return self.handle_login(request);
        }

        let session_id = self.get_session_id(request);
        let config = self
            .configs
            .get(server_id)
            .ok_or_else(|| ServerError::Internal("Server configuration not found".to_string()))?;
        let router = self
            .routers
            .get(server_id)
            .ok_or_else(|| ServerError::Internal("Router not found".to_string()))?;

        // Gérer les requêtes DELETE simulées
        let method = if request.method == HttpMethod::POST {
            if let Some(body) = String::from_utf8(request.body.clone()).ok() {
                if body.contains("_method=DELETE") {
                    HttpMethod::DELETE
                } else {
                    HttpMethod::POST
                }
            } else {
                HttpMethod::POST
            }
        } else {
            request.method
        };

        match router.match_route(request) {
            RouteMatchResult::Match(route_match) => {
                if route_match.route.protected
                    && !self.session_manager.is_authenticated(&session_id)
                {
                    return Ok(self.create_error_response(403));
                }

                if let Some(redirect_to) = route_match.redirect_to {
                    let mut response = HttpResponse::new(301, Vec::new(), "text/plain");
                    response.headers.insert("Location".to_string(), redirect_to);
                    return Ok(response);
                } else if self.is_cgi_request(&route_match.file_path) {
                    match self.cgi_handler.handle(request, &route_match.file_path) {
                        Ok(response) => return Ok(response),
                        Err(e) => {
                            self.logger.error(&format!("CGI handler error: {}", e));
                            return Ok(self.create_error_response(500));
                        }
                    }
                } else {
                    let response = match method {
                        HttpMethod::GET => self.handle_get(&route_match, request)?,
                        HttpMethod::POST => self.handle_post(request, &route_match, &config),
                        HttpMethod::DELETE => self.handle_delete(&route_match),
                        HttpMethod::UNSUPPORTED => self.create_error_response(405),
                    };

                    let mut response = response;
                    if !request.headers.contains_key("cookie") {
                        response.headers.insert(
                            "Set-Cookie".to_string(),
                            format!("session_id={}; HttpOnly", session_id),
                        );
                    }
                    Ok(response)
                }
            }
            RouteMatchResult::MethodNotAllowed(_) => Ok(self.create_error_response(405)),
            RouteMatchResult::NotFound => Ok(self.create_error_response(404)),
        }
    }

    fn create_session_id(&mut self, _request: &HttpRequest) -> String {
        self.session_manager.create_session()
    }

    fn get_session_id(&self, request: &HttpRequest) -> String {
        if let Some(cookie) = request.headers.get("cookie") {
            if let Some(session_id) = cookie.split(';').find_map(|s| {
                let parts: Vec<&str> = s.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == "session_id" {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            }) {
                return session_id;
            }
        }
        String::new()
    }
    fn handle_login(&mut self, request: &HttpRequest) -> Result<HttpResponse, ServerError> {
        if request.method == HttpMethod::POST {
            // Check credentials
            let body = String::from_utf8_lossy(&request.body);
            let params: HashMap<_, _> = body
                .split('&')
                .filter_map(|kv| {
                    let mut parts = kv.splitn(2, '=');
                    Some((parts.next()?, parts.next()?))
                })
                .collect();
            if params.get("username") == Some(&"user") && params.get("password") == Some(&"123") {
                let session_id = self.create_session_id(request);
                self.session_manager.authenticate(&session_id);
                let mut response = HttpResponse::new(302, Vec::new(), "text/plain");
                response
                    .headers
                    .insert("Location".to_string(), "/".to_string());
                response.headers.insert(
                    "Set-Cookie".to_string(),
                    format!("session_id={}; HttpOnly", session_id),
                );
                return Ok(response);
            }
        }

        // Display login form
        let login_form = r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Login</title>
        <style>
            body {
                font-family: Arial, sans-serif;
                background-color: #f0f0f0;
                display: flex;
                justify-content: center;
                align-items: center;
                height: 100vh;
                margin: 0;
            }
            .login-container {
                background-color: white;
                padding: 2rem;
                border-radius: 8px;
                box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                width: 300px;
            }
            h2 {
                text-align: center;
                color: #333;
            }
            form {
                display: flex;
                flex-direction: column;
            }
            label {
                margin-top: 1rem;
                color: #555;
            }
            input[type="text"], input[type="password"] {
                padding: 0.5rem;
                margin-top: 0.5rem;
                border: 1px solid #ddd;
                border-radius: 4px;
            }
            input[type="submit"] {
                margin-top: 1.5rem;
                padding: 0.75rem;
                background-color: #4CAF50;
                color: white;
                border: none;
                border-radius: 4px;
                cursor: pointer;
                transition: background-color 0.3s;
            }
            input[type="submit"]:hover {
                background-color: #45a049;
            }
        </style>
    </head>
    <body>
        <div class="login-container">
            <h2>Login</h2>
            <form method="post" action="/login">
                <label for="username">Username:</label>
                <input type="text" id="username" name="username" required>
                
                <label for="password">Password:</label>
                <input type="password" id="password" name="password" required>
                
                <input type="submit" value="Login">
            </form>
        </div>
    </body>
    </html>
    "#;

        Ok(HttpResponse::new(
            200,
            login_form.as_bytes().to_vec(),
            "text/html",
        ))
    }
    fn is_cgi_request(&self, file_path: &str) -> bool {
        file_path.ends_with(".php") || file_path.ends_with(".py")
    }

    fn handle_get(
        &self,
        route_match: &RouteMatch,
        request: &HttpRequest,
    ) -> Result<HttpResponse, ServerError> {
        let path = Path::new(&route_match.file_path);

        if path.is_dir() {
            if route_match.route.directory_listing {
                // Vérifier les permissions de lecture
                if fs::metadata(path)
                    .map(|m| m.permissions().readonly())
                    .unwrap_or(true)
                {
                    return Ok(self.create_error_response(403));
                }
                return self.handle_directory_listing(path, &request.path);
            } else {
                return Err(ServerError::Http(403)); // Forbidden
            }
        }
        match StaticFile::new(&route_match.file_path) {
            Ok(mut file) => {
                println!("file size: {:?}", file.size);
                if file.size > 1_000_000 {
                    // 1MB threshold for chunked transfer
                    let mut chunks = Vec::new();
                    let mut buffer = vec![0; 262144]; // 256KB buffer
                    loop {
                        match file.read_chunk(&mut buffer) {
                            Ok(0) => break, // Fin du fichier atteinte
                            Ok(n) => chunks.push(buffer[..n].to_vec()),
                            Err(e) => return Err(ServerError::Io(e)),
                        }
                        if file.current_position >= file.size {
                            break; // Assurez-vous de sortir de la boucle une fois le fichier entièrement lu
                        }
                    }
                    Ok(HttpResponse::new_chunked(200, chunks, &file.content_type))
                } else {
                    Ok(HttpResponse::new(200, file.content, &file.content_type))
                }
            }
            Err(_) => Err(ServerError::Http(404)),
        }
    }

    fn handle_post(
        &self,
        request: &HttpRequest,
        route_match: &RouteMatch,
        config: &ServerConfig,
    ) -> HttpResponse {
        if request.body.len() > config.client_max_body_size {
            return self.create_error_response(413);
        }
        let content_type = request
            .headers
            .get("content-type")
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        if content_type.starts_with("multipart/form-data") {
            return self.handle_file_upload(request, route_match);
        } else {
            // Traiter tous les autres types de contenu, y compris
            // application/x-www-form-urlencoded et application/octet-stream
            match fs::write(&route_match.file_path, &request.body) {
                Ok(_) => {
                    HttpResponse::new(200, b"File uploaded successfully".to_vec(), "text/plain")
                }
                Err(_) => self.create_error_response(500),
            }
        }
    }

    fn handle_delete(&self, route_match: &RouteMatch) -> HttpResponse {
        match fs::remove_file(&route_match.file_path) {
            Ok(_) => HttpResponse::new(200, b"Deleted".to_vec(), "text/plain"),
            Err(_) => self.create_error_response(400),
        }
    }

    fn handle_file_upload(&self, request: &HttpRequest, route_match: &RouteMatch) -> HttpResponse {
        self.logger.info(&format!(
            "Handling file upload. Content-Type: {:?}",
            request.headers.get("content-type")
        ));
        self.logger
            .info(&format!("Request body size: {} bytes", request.body.len()));

        // Log all headers for debugging
        for (key, value) in &request.headers {
            self.logger.info(&format!("Header: {} = {}", key, value));
        }

        if let Some(content_type) = request.headers.get("content-type") {
            if content_type.starts_with("multipart/form-data") {
                let uploads_dir = Path::new(&route_match.route.root).join("uploads");
                if let Err(e) = fs::create_dir_all(&uploads_dir) {
                    self.logger
                        .error(&format!("Failed to create uploads directory: {}", e));
                    return HttpResponse::new(500, b"Internal Server Error".to_vec(), "text/plain");
                }

                // Nouvelle méthode d'extraction du nom de fichier
                let filename = self
                    .extract_filename_from_body(&request.body)
                    .unwrap_or_else(|| {
                        self.logger
                            .warn("Failed to extract filename from body, falling back to headers");
                        self.extract_filename_from_headers(&request.headers)
                            .unwrap_or_else(|| {
                                self.logger
                                    .warn("Failed to extract filename from headers, using default");
                                "uploaded_file".to_string()
                            })
                    });

                self.logger
                    .info(&format!("Extracted filename: {}", filename));

                let safe_filename = sanitize_filename(&filename);
                let file_path = get_unique_filename(&uploads_dir, &safe_filename);

                let content = extract_file_content(&request.body);

                if let Err(e) = fs::write(&file_path, content) {
                    self.logger
                        .error(&format!("Failed to save uploaded file: {}", e));
                    return HttpResponse::new(500, b"Failed to save file".to_vec(), "text/plain");
                }

                self.logger.info(&format!("File saved: {:?}", file_path));
                HttpResponse::new(200, b"File uploaded successfully".to_vec(), "text/plain")
            } else {
                HttpResponse::new(
                    400,
                    b"Bad Request: Content-Type must be multipart/form-data".to_vec(),
                    "text/plain",
                )
            }
        } else {
            HttpResponse::new(
                400,
                b"Bad Request: Missing Content-Type".to_vec(),
                "text/plain",
            )
        }
    }

    fn extract_filename_from_body(&self, body: &[u8]) -> Option<String> {
        let body_str = std::str::from_utf8(body).ok()?;
        let lines: Vec<&str> = body_str.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("Content-Disposition:") && line.contains("filename=") {
                self.logger
                    .info(&format!("Found Content-Disposition line: {}", line));
                if let Some(filename) = line.split("filename=").nth(1) {
                    let filename = filename.trim_matches('"');
                    self.logger
                        .info(&format!("Extracted filename from body: {}", filename));
                    return Some(filename.to_string());
                }
            }
            // Log a few lines around the Content-Disposition for context
            if i > 0 {
                self.logger
                    .info(&format!("Previous line: {}", lines[i - 1]));
            }
            if i < lines.len() - 1 {
                self.logger.info(&format!("Next line: {}", lines[i + 1]));
            }
        }

        self.logger.warn("Could not find filename in body");
        None
    }

    fn extract_filename_from_headers(&self, headers: &HashMap<String, String>) -> Option<String> {
        headers.get("content-disposition").and_then(|disp| {
            self.logger
                .info(&format!("Content-Disposition header: {}", disp));
            disp.split(';').find_map(|part| {
                let part = part.trim();
                if part.starts_with("filename=") {
                    let filename = part.trim_start_matches("filename=").trim_matches('"');
                    self.logger
                        .info(&format!("Extracted filename from header: {}", filename));
                    Some(filename.to_string())
                } else {
                    None
                }
            })
        })
    }
    fn handle_client(&mut self, token: Token) -> io::Result<()> {
        self.logger.info(&format!("Handling client: {:?}", token));
        if !self.clients.contains_key(&token) {
            return Ok(());
        }

        let now = Instant::now();
        let start_time = self.request_start_times.entry(token).or_insert(now);

        if now.duration_since(*start_time) > REQUEST_TIMEOUT {
            return self.handle_timeout(token);
        }

        match self.read_request(token) {
            Ok(Some(request)) => {
                self.logger
                    .info(&format!("Received request: {:?}", request));
                let server_id = self.clients.get(&token).unwrap().1.clone();
                self.process_request(token, request, &server_id)
            }
            Ok(None) => Ok(()),
            Err(e) => self.handle_error(token, e),
        }
    }

    fn read_request(&mut self, token: Token) -> io::Result<Option<HttpRequest>> {
        let mut buffer = vec![0; 16384];
        if let Some((stream, _)) = self.clients.get_mut(&token) {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    self.logger.info("Client disconnected");
                    self.remove_client(token)?;
                    Ok(None)
                }
                Ok(n) => {
                    let request = HttpRequest::parse(&buffer[..n]);
                    if request.is_err() && n == buffer.len() {
                        return Ok(None);
                    }
                    request
                        .map(Some)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    self.logger.info("Would block on read, trying again later");
                    Ok(None)
                }
                Err(e) => {
                    self.logger
                        .error(&format!("Error reading from client: {}", e));
                    Err(e)
                }
            }
        } else {
            Ok(None)
        }
    }

    fn process_request(
        &mut self,
        token: Token,
        request: HttpRequest,
        server_id: &str,
    ) -> io::Result<()> {
        let keep_alive = request
            .headers
            .get("connection")
            .map(|v| v.to_lowercase() == "keep-alive")
            .unwrap_or(false);

        let response = match self.handle_request(&request, server_id) {
            Ok(res) => res,
            Err(e) => {
                self.logger.error(&format!("Error handling request: {}", e));
                self.create_error_response(500)
            }
        };

        let mut response = response;
        if keep_alive {
            response
                .headers
                .insert("Connection".to_string(), "keep-alive".to_string());
        } else {
            response
                .headers
                .insert("Connection".to_string(), "close".to_string());
        }

        self.send_response(token, &response)?;

        if !keep_alive {
            self.remove_client(token)?;
        } else {
            self.request_start_times.remove(&token);
        }

        Ok(())
    }

    fn handle_timeout(&mut self, token: Token) -> io::Result<()> {
        let error_response = self.create_error_response(408);
        self.send_response(token, &error_response)?;
        self.remove_client(token)
    }

    fn handle_error(&mut self, token: Token, error: io::Error) -> io::Result<()> {
        self.logger
            .error(&format!("Error handling client: {}", error));
        let status = match error.kind() {
            io::ErrorKind::InvalidData => 400,
            io::ErrorKind::TimedOut => 408,
            _ => 500,
        };
        let error_response = self.create_error_response(status);
        self.send_response(token, &error_response)?;
        self.remove_client(token)
    }

    fn send_response(&mut self, token: Token, response: &HttpResponse) -> io::Result<()> {
        self.logger
            .info(&format!("Sending response: {:?}", response));
        if let Some((stream, _)) = self.clients.get_mut(&token) {
            Self::write_response(stream, response)
        } else {
            Ok(())
        }
    }

    fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> io::Result<()> {
        let status_text = match response.status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            408 => "Request Timeout",
            500 => "Internal Server Error",
            413 => "Payload Too Large",
            301 => "Redirection",
            302 => "Redirection",
            _ => "Unknown",
        };
        stream.write_all(format!("HTTP/1.1 {} {}\r\n", response.status, status_text).as_bytes())?;
        for (key, value) in &response.headers {
            stream.write_all(format!("{}: {}\r\n", key, value).as_bytes())?;
        }
        stream.write_all(b"\r\n")?;

        match &response.body {
            ResponseBody::Whole(body) => {
                println!("Sending whole body of size {}", body.len());
                stream.write_all(body)?;
            }
            ResponseBody::Chunked(chunks) => {
                println!("Sending chunked body with {} chunks", chunks.len());
                stream.write_all(b"Transfer-Encoding: chunked\r\n\r\n")?;
                for (_i, chunk) in chunks.iter().enumerate() {
                    stream.write_all(format!("{:X}\r\n", chunk.len()).as_bytes())?;
                    stream.write_all(chunk)?;
                    stream.write_all(b"\r\n")?;
                }
                println!("Sending final chunk");
                stream.write_all(b"0\r\n\r\n")?;
            }
        }
        stream.flush()
    }

    fn create_error_response(&self, status_code: u16) -> HttpResponse {
        // Chemin vers le dossier des pages d'erreur personnalisées
        let custom_error_pages_dir = Path::new("custom_error_pages");

        // Chercher d'abord dans les pages d'erreur configurées
        for (_server_id, config) in &self.configs {
            if let Some(error_page) = config.error_pages.get(&status_code) {
                let custom_error_page_path = custom_error_pages_dir.join(error_page);
                match fs::read_to_string(&custom_error_page_path) {
                    Ok(content) => {
                        return HttpResponse::new(status_code, content.into_bytes(), "text/html")
                    }
                    Err(e) => self.logger.warn(&format!(
                        "Failed to read custom error page {}: {}",
                        custom_error_page_path.display(),
                        e
                    )),
                }
            }
        }

        // Si aucune page d'erreur personnalisée n'est configurée ou si la lecture a échoué, utiliser les pages d'erreur par défaut
        let default_error_page_path =
            Path::new("error_pages").join(format!("{}.html", status_code));
        match fs::read_to_string(&default_error_page_path) {
            Ok(content) => HttpResponse::new(status_code, content.into_bytes(), "text/html"),
            Err(_) => {
                // Fallback si la page d'erreur spécifique n'existe pas
                let fallback_content =
                    format!("<html><body><h1>Error {}</h1></body></html>", status_code);
                HttpResponse::new(status_code, fallback_content.into_bytes(), "text/html")
            }
        }
    }

    fn remove_client(&mut self, token: Token) -> io::Result<()> {
        if let Some((mut stream, _)) = self.clients.remove(&token) {
            self.poll.registry().deregister(&mut stream)?;
        }
        self.request_start_times.remove(&token);
        Ok(())
    }
}

fn human_readable_size(size: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

fn extract_file_content(body: &[u8]) -> Vec<u8> {
    let body_str = String::from_utf8_lossy(body);
    let lines: Vec<&str> = body_str.lines().collect();

    // Trouver l'index de la ligne qui commence par "Content-Type:"
    if let Some(content_type_index) = lines
        .iter()
        .position(|line| line.starts_with("Content-Type:"))
    {
        // Le contenu du fichier commence à deux lignes après "Content-Type:"
        let content_start = content_type_index + 2;
        // Le contenu se termine à la ligne avant la dernière (qui est le délimiteur de fin)
        let content_end = lines.len() - 1;

        // Joindre les lignes de contenu et les convertir en bytes
        lines[content_start..content_end].join("\n").into_bytes()
    } else {
        // Si on ne trouve pas la ligne "Content-Type:", on retourne le body tel quel
        body.to_vec()
    }
}

fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|&c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
        .collect()
}

fn get_unique_filename(dir: &Path, filename: &str) -> PathBuf {
    let file_stem = Path::new(filename).file_stem().unwrap().to_str().unwrap();
    let extension = Path::new(filename)
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap();
    let mut counter = 0;
    let mut file_path = dir.join(filename);

    while file_path.exists() {
        counter += 1;
        let new_filename = format!("{}_{}.{}", file_stem, counter, extension);
        file_path = dir.join(new_filename);
    }

    file_path
}
