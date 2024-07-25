use std::collections::HashMap;
use std::net::TcpListener;

use super::connection::Connection;

pub struct Server {
    pub listener: TcpListener,
    pub connections: HashMap<usize, Connection>,
}

impl Server {
    pub fn new(addr: &str) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Server {
            listener,
            connections: HashMap::new(),
        })
    }
}
