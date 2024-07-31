use std::net::SocketAddr;

pub struct ServerConfig {
    pub addresses: Vec<SocketAddr>,
}

impl ServerConfig {
    pub fn new() -> Self {
        ServerConfig {
            addresses: vec![
                "127.0.0.1:8080".parse().unwrap(),
                "127.0.0.1:8081".parse().unwrap(),
                "127.0.0.2:8080".parse().unwrap(),
                "127.0.0.2:8081".parse().unwrap(),
            ],
        }
    }
}
