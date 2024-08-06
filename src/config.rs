use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub id: String,
    pub host: IpAddr,
    pub ports: Vec<u16>,
    pub server_names: Vec<String>,
    pub error_pages: HashMap<u16, String>,
    pub client_max_body_size: usize,
    pub routes: Vec<RouteConfig>,
}

#[derive(Clone, Debug)]
pub struct RouteConfig {
    pub path: String,
    pub methods: Vec<String>,
    pub root: String,
    pub index: Option<String>,
    pub cgi_extensions: HashMap<String, String>,
    pub directory_listing: bool,
    pub redirections: HashMap<String, String>,
    pub protected: bool,
}

impl ServerConfig {
    pub fn from_file(path: &str) -> io::Result<Vec<Self>> {
        let content = fs::read_to_string(path)?;
        let configs = Self::parse(&content)?;

        if configs.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "No valid server configurations found in the file"));
        }

        let mut used_ports = HashSet::new();
        for config in &configs {
            for &port in &config.ports {
                if !used_ports.insert(port) {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Port {} is configured multiple times", port)));
                }
            }
        }
        
        Ok(configs)
    }

    fn parse(content: &str) -> io::Result<Vec<Self>> {
        let mut configs = Vec::new();
        let mut current_config: Option<ServerConfig> = None;
        let mut current_route: Option<RouteConfig> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            match key {
                "server" => {
                    if let Some(mut config) = current_config.take() {
                        if let Some(route) = current_route.take() {
                            config.routes.push(route);
                        }
                        configs.push(config);
                    }
                    current_config = Some(ServerConfig {
                        id: format!("server_{}", configs.len()),
                        host: "0.0.0.0".parse().unwrap(),
                        ports: vec![],
                        server_names: vec![value.to_string()],
                        error_pages: HashMap::new(),
                        client_max_body_size: 1_000_000,
                        routes: vec![],
                    });
                },
                "host" => if let Some(config) = current_config.as_mut() {
                    config.host = value.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                },
                "port" => if let Some(config) = current_config.as_mut() {
                    config.ports.push(value.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?);
                },
                "error_page" => if let Some(config) = current_config.as_mut() {
                    let error_parts: Vec<&str> = value.splitn(2, ' ').collect();
                    if error_parts.len() == 2 {
                        let code: u16 = error_parts[0].parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                        config.error_pages.insert(code, error_parts[1].to_string());
                    }
                },
                "client_max_body_size" => if let Some(config) = current_config.as_mut() {
                    config.client_max_body_size = value.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                },
                "route" => {
                    if let Some(route) = current_route.take() {
                        if let Some(config) = current_config.as_mut() {
                            config.routes.push(route);
                        }
                    }
                    current_route = Some(RouteConfig {
                        path: value.to_string(),
                        methods: vec![],
                        root: String::new(),
                        index: None,
                        cgi_extensions: HashMap::new(),
                        directory_listing: false,
                        redirections: HashMap::new(),
                        protected: false,
                    });
                },
                "methods" => if let Some(route) = current_route.as_mut() {
                    route.methods = value.split_whitespace().map(|s| s.to_string()).collect();
                },
                "root" => if let Some(route) = current_route.as_mut() {
                    route.root = value.to_string();
                },
                "index" => if let Some(route) = current_route.as_mut() {
                    route.index = Some(value.to_string());
                },
                "cgi" => if let Some(route) = current_route.as_mut() {
                    let cgi_parts: Vec<&str> = value.splitn(2, ' ').collect();
                    if cgi_parts.len() == 2 {
                        route.cgi_extensions.insert(cgi_parts[0].to_string(), cgi_parts[1].to_string());
                    }
                },
                "protected" => if let Some(route) = current_route.as_mut() {
                    route.protected = value.to_lowercase() == "true";
                },
                "directory_listing" => if let Some(route) = current_route.as_mut() {
                    route.directory_listing = value.to_lowercase() == "on";
                },
                "redirect" => if let Some(route) = current_route.as_mut() {
                    let redirect_parts: Vec<&str> = value.splitn(2, ' ').collect();
                    if redirect_parts.len() == 2 {
                        route.redirections.insert(redirect_parts[0].to_string(), redirect_parts[1].to_string());
                    }
                },
                _ => {}
            }
        }

        if let Some(mut config) = current_config.take() {
            if let Some(route) = current_route.take() {
                config.routes.push(route);
            }
            configs.push(config);
        }

        Ok(configs)
    }
}